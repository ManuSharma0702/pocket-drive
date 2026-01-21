use std::{collections::HashMap, env, path::PathBuf, time::{Duration, SystemTime}};

use notify_debouncer_full::DebouncedEvent;
use sqlite::{Connection, State};
use walkdir::WalkDir;
use std::sync::mpsc::{Receiver, Sender, channel};

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub path: PathBuf,
    pub hash: Option<String>,
    pub size: u64,
    pub modified: SystemTime,
}

pub enum DbCmd{
    ProcessEvents(Vec<DebouncedEvent>),
    Get(PathBuf, Sender<Option<FileEntry>>),
    Insert(FileEntry),
    BulkInsert(Vec<FileEntry>),
    Delete(PathBuf),
    Update(FileEntry)
}

#[derive(Hash, PartialEq, Eq)]
pub enum ParserCmd {
    Insert,
    Delete,
    Update
}

pub struct Db{
    conn: Connection,
    tx: Sender<DbCmd>,
    rx: Receiver<DbCmd>
}

impl Db{
    pub fn new() -> Self {
        let (tx, rx) = channel();
        let connection = sqlite::open("memory").unwrap();
        let query = "
            CREATE TABLE IF NOT EXISTS filehash (filepath TEXT, filehash TEXT, size BIGDECIMAL, modified DATETIME);
        ";
        connection.execute(query).unwrap();
        Db{
            conn: connection,
            tx,
            rx
        }
    }

    pub fn get_sender(&self) -> Sender<DbCmd>{
        self.tx.clone()
    }

    pub fn run(&self) {
        while let Ok(cmd) = self.rx.recv() {
            self.execute(cmd).unwrap();
        }
    }

    fn execute(&self, cmd: DbCmd) -> sqlite::Result<Option<FileEntry>>{
        match cmd {
            DbCmd::Get(path, sender) => {
                let mut stmt = self.conn.prepare(
                    "Select * from filehash where filepath = ?"
                )?;
                stmt.bind((1, path.to_str())).unwrap();
                if let Ok(State::Row) = stmt.next() {
                    let path: String = stmt.read(0)?;
                    let hash: String = stmt.read(1)?;
                    let size: i64 = stmt.read(2)?;
                    let modified: i64 = stmt.read(3)?;

                    sender.send(Some(FileEntry{
                        path: PathBuf::from(path),
                        hash: Some(hash),
                        size: size as u64,
                        modified: SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(modified as u64)
                    })).unwrap();
                };
                sender.send(None).unwrap();
                Ok(None)
            }
            DbCmd::Insert(file) => {
                let mut stmt = self.conn.prepare(
                    "Insert INTO filehash VALUES (?,?,?,?)"
                )?;
                stmt.bind((1, file.path.to_str()))?;
                stmt.bind((2, file.hash.unwrap_or("".to_string()).as_str()))?;
                stmt.bind((3, file.size as i64))?;
                stmt.bind((4, file.modified.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs() as i64))?;

                stmt.next()?;
                Ok(None)
            }

            DbCmd::Delete(path) => {
                let mut stmt = self.conn.prepare(
                    "DELETE FROM filehash where filepath = ?"
                )?;
                stmt.bind((1, path.to_str()))?;
                stmt.next()?;
                Ok(None)
            }
            
            DbCmd::Update(file) => {
                let mut stmt = self.conn.prepare(
                    "INSERT INTO filehash (filepath, filehash, size, modified)
                    VALUES (?, ?, ?, ?)
                    ON CONFLICT(filepath) DO UPDATE SET
                         filehash = excluded.filehash,
                         size = excluded.size,
                         modified = excluded.modified"
                )?;

                stmt.bind((1, file.path.to_str().unwrap()))?;
                stmt.bind((2, file.hash.unwrap_or("".to_string()).as_str()))?;
                stmt.bind((3, file.size as i64))?;
                stmt.bind((4, file.modified.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs() as i64))?;
                stmt.next()?;

                Ok(None)
            }
            
            DbCmd::BulkInsert(files) => {
                self.conn.execute("BEGIN TRANSACTION")?;
                let mut stmt = self.conn.prepare(
                    "INSERT OR REPLACE INTO filehash (filepath, filehash, size, modified) VALUES (?, ?, ?, ?)"
                )?;

                for file in files {
                    stmt.bind((1, file.path.to_str().unwrap()))?;
                    stmt.bind((2, file.hash.unwrap_or("".to_string()).as_str()))?;
                    stmt.bind((3, file.size as i64))?;
                    stmt.bind((
                        4,
                        file.modified
                            .duration_since(SystemTime::UNIX_EPOCH)
                            .unwrap()
                            .as_secs() as i64,
                    ))?;

                    stmt.next()?;
                    stmt.reset()?;
                }

                self.conn.execute("COMMIT")?;
                dbg!("INSERTED SUCCESSFULLY");

                Ok(None)
            }

            //Walk the entire directory and compare the metadata of file with the metadata in DB;; //3 case: //Diff -> update hash and metadata update the db entry //Missing -> insert //Present, same hash -> ignore //Present in DB, not in directory -> Remove //Get the entire db in memory in Bulk, as a hashMap of <Path, FileEntry> //Create the HashMap of directory files as <Path, FileEntry> //Make comparisons and store in a separate map with <DbCmd, Vec<FileEntry>> //Execute each command over the vector in batch //Send the same command over to the Server for sync
            DbCmd::ProcessEvents(events) => {
                let mut stmt = self.conn.prepare(
                    "SELECT * FROM filehash"
                )?;

                let mut db_map: HashMap<PathBuf, FileEntry> = HashMap::new();

                while let Ok(State::Row) = stmt.next() {
                    let path: String = stmt.read(0)?;
                    let hash: String = stmt.read(1)?;
                    let size: i64 = stmt.read(2)?;
                    let modified: i64 = stmt.read(3)?;

                    let path_buf = PathBuf::from(path);

                    let f = FileEntry {
                        path: path_buf.clone(),
                        hash: Some(hash),
                        size: size as u64,
                        modified: SystemTime::UNIX_EPOCH + Duration::from_secs(modified as u64),
                    };

                    db_map.insert(path_buf, f);
                }
                let args: Vec<String> = env::args().collect();
                let path = &args[1];
                let mut directory_map: HashMap<PathBuf, FileEntry> = HashMap::new();

                for entry in WalkDir::new(path).into_iter().filter_map(Result::ok) {

                    let metadata = match entry.metadata() {
                        Ok(m) => m,
                        Err(_) => continue,
                    };

                    if !metadata.is_file() {
                        continue;
                    }

                    let path = entry.into_path();

                    let modified = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);

                    let f = FileEntry {
                        path: path.clone(),
                        hash: None,
                        size: metadata.len(),
                        modified,
                    };

                    directory_map.insert(path, f);
                }
                //3 cases 
                //Present in Db but not in directory then should be removed
                //Not present in DB Present in directory, then should be added
                //Present in both, but metadata is different then update
                //Present in both, and metadata same, then make no change
                let mut parser_cmds: HashMap<ParserCmd, FileEntry> = HashMap::new();

                for (path, fi) in &db_map {
                    if !directory_map.contains_key(path) {
                        parser_cmds.insert(ParserCmd::Delete, fi.clone());
                    }
                }

                for (path, fi) in directory_map {
                    if db_map.contains_key(&path) {
                        let file1 = fi;
                        let file2 = db_map.get(&file1.path).unwrap();
                        if !self.is_metadata_same(&file1, file2) {
                            parser_cmds.insert(ParserCmd::Insert, file1);
                        }
                    } else {
                        parser_cmds.insert(ParserCmd::Insert, fi);
                    }
                }

                Ok(None)            
            }
        }

    }

    fn is_metadata_same(&self, file1: &FileEntry, file2: &FileEntry) -> bool {
        let t1 = file1.modified.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();
        let t2 = file2.modified.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();

        file1.size == file2.size && t1 == t2
    }

    fn execute_parser_cmds(&self, parser_cmd: HashMap<ParserCmd, FileEntry>) {
        unimplemented!();
    }

}

impl Default for Db {
    fn default() -> Self {
        Self::new()
    }
}
