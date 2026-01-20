use std::{path::PathBuf, time::SystemTime};

use notify_debouncer_full::DebouncedEvent;
use sqlite::{Connection, State};
use std::sync::mpsc::{Receiver, Sender, channel};

#[derive(Debug)]
pub struct FileEntry {
    pub path: PathBuf,
    pub hash: String,
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
                        hash,
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
                stmt.bind((2, file.hash.as_str()))?;
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
                stmt.bind((2, file.hash.as_str()))?;
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
                    stmt.bind((2, file.hash.as_str()))?;
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

            DbCmd::ProcessEvents(events) => {
                Ok(None)
            }
        }

    }

}

impl Default for Db {
    fn default() -> Self {
        Self::new()
    }
}
