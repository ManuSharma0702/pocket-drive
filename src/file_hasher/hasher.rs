use std::path::PathBuf;
use blake2::{Blake2s256, Digest};
use file_hashing::get_hash_file;

//Hasher will contain the tx for DB
struct Hasher;

impl Hasher {
    
    //Generate the hash and store in sqlite db
    fn initialise(&self, path: &str) {

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

