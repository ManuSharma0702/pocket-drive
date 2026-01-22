use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use std::sync::mpsc::{self, Sender, Receiver};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::path::PathBuf;
use blake2::{Blake2s256, Digest};
use file_hashing::get_hash_file;

use crate::db_listener::db::{FileEntry};


pub enum HasherCmd {
    Generate(Vec<FileEntry>, Sender<Vec<FileEntry>>)
}

pub struct Hasher{
    tx_hasher: Sender<HasherCmd>,
    rx_hasher: Receiver<HasherCmd>
}

impl Hasher {
    
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            tx_hasher: tx,
            rx_hasher: rx
        }
    }

    pub fn get_sender(&self) -> Sender<HasherCmd>{
        self.tx_hasher.clone()
    }

    pub fn run(&self) {
        while let Ok(cmd) = self.rx_hasher.recv() {
            self.execute(cmd);
        }
    }

    fn execute(&self, cmd: HasherCmd) {
        match cmd {
            HasherCmd::Generate(files, sender) => {
                let hashed_files = self.generate_hash_in_bulk(files);
                sender.send(hashed_files).unwrap();
            }
        }

    }

    //Generate the hash and store in sqlite db
    fn generate_hash_in_bulk(&self, paths: Vec<FileEntry>) -> Vec<FileEntry>{
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
            .map(|mut p| {
                let mut hash = Blake2s256::new();
                let res = get_hash_file(&p.path, &mut hash).unwrap();

                let done = counter.fetch_add(1, Ordering::Relaxed) + 1;
                pb.set_position(done as u64);
                p.hash = Some(res);
                p

            })
            .collect();

        pb.finish_with_message("done");
        results
    }

    fn generate_hash(&self, path: &str) -> String {
        let path = PathBuf::from(path);
        let mut hash = Blake2s256::new();
        get_hash_file(&path, &mut hash).unwrap()
    }

    fn hash_equal(&self, hash1: &str, hash2: &str) -> bool{
        hash1.eq(hash2)
    }

}

impl Default for Hasher {
    fn default() -> Self {
        Self::new()
    }
}
