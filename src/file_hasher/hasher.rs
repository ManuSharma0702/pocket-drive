use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use std::sync::mpsc::Sender;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::path::PathBuf;
use std::time::SystemTime;
use blake2::{Blake2s256, Digest};
use file_hashing::get_hash_file;
use walkdir::WalkDir;

use crate::db_listener::db::{DbCmd, FileEntry};

//Hasher will contain the tx for DB
pub struct Hasher{
    tx_db: Sender<DbCmd>
}

impl Hasher {
    
    pub fn new(tx_db: Sender<DbCmd>) -> Self {
        Self {
            tx_db
        }
    }
    //Generate the hash and store in sqlite db
    pub async fn initialise(&self, directory: &str) {
        let walkdir = WalkDir::new(directory);
        let mut paths: Vec<PathBuf> = Vec::new();

        for file in walkdir.into_iter().filter_map(|file| file.ok()) {
            if file.metadata().unwrap().is_file() {
                paths.push(file.into_path());
            }
        }

        let total = paths.len() as u64;

        let pb = ProgressBar::new(total);
        pb.set_style(
            ProgressStyle::with_template(
                "[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} {msg}",
            )
            .unwrap(),
        );

        let counter = Arc::new(AtomicUsize::new(0));

        let results: Vec<FileEntry> = paths
            .into_par_iter()
            .map(|p| {
                let mut hash = Blake2s256::new();
                let res = get_hash_file(&p, &mut hash).unwrap();

                let done = counter.fetch_add(1, Ordering::Relaxed) + 1;
                pb.set_position(done as u64);
                let metadata = std::fs::metadata(&p).unwrap();
                let size = metadata.len();
                let modified = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);


                FileEntry{
                    path: p,
                    hash: res,
                    size,
                    modified
                }
            })
            .collect();

        pb.finish_with_message("done");

        dbg!("Sending");
        self.tx_db.send(DbCmd::BulkInsert(results)).unwrap();
    }



    fn generate_hash(&self, path: &str) -> String {
        let path = PathBuf::from(path);
        let mut hash = Blake2s256::new();
        get_hash_file(&path, &mut hash).unwrap()
    }

    fn hash_equal(&self, hash1: &str, hash2: &str) -> bool{
        hash1.eq(hash2)
    }

    fn fetch_hash_from_db(path: &str){ 
    }
}

