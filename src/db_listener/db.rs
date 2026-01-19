use std::{path::PathBuf, time::SystemTime};

use sqlite::{Connection, State};
use tokio::sync::mpsc::{Receiver, Sender, channel};

pub struct FileEntry {
    path: PathBuf,
    hash: String,
    size: u64,
    modified: SystemTime,
}

pub enum DbCmd{
    Get(PathBuf),
    Insert(FileEntry),
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
        let (tx, rx) = channel(1024);
        let connection = sqlite::open(":memory:").unwrap();
        let query = "
            CREATE TABLE filehash (filepath TEXT, filehash TEXT, size: BIGDECIMAL, modified: DATETIME);
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

    pub async fn run(mut self) {
        while let Some(batch) = self.rx.recv().await {
            self.execute(batch).await.unwrap();
        }
    }

    async fn execute(&self, cmd: DbCmd) -> sqlite::Result<Option<FileEntry>>{
        match cmd {
            DbCmd::Get(path) => {
                let mut stmt = self.conn.prepare(
                    "Select * from filehash where filepath = ?"
                )?;
                stmt.bind((1, path.to_str())).unwrap();
                if let Ok(State::Row) = stmt.next() {
                    let path: String = stmt.read(0)?;
                    let hash: String = stmt.read(1)?;
                    let size: i64 = stmt.read(2)?;
                    let modified: i64 = stmt.read(3)?;

                    return Ok(
                        Some(FileEntry{
                            path: PathBuf::from(path),
                            hash,
                            size: size as u64,
                            modified: SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(modified as u64)
                        })
                    );
                };
                Ok(None)
            }
            DbCmd::Insert(file) => {
                let mut stmt = self.conn.prepare(
                    "Insert INTO filehash VALUE (?,?,?,?)"
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
                    "DELETE FROM filehasNh where filepath = ?"
                )?;
                stmt.bind((1, path.to_str()))?;
                stmt.next()?;
                Ok(None)
            }
            
            DbCmd::Update(file) => {
                let mut stmt = self.conn.prepare(
                    "UPDATE filehash SET filehash = ?, size = ?, modified = ? WHERE filepath = ?"
                )?;

                stmt.bind((1, file.hash.as_str()))?;
                stmt.bind((2, file.size as i64))?;
                stmt.bind((2, file.modified.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs() as i64))?;
                stmt.bind((4, file.path.to_str()))?;

                stmt.next()?;

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
