use crate::crypto::{key_pair, merkle::MerkleTree, hash::{H256, Hashable}};
use crate::transaction::{Transaction, generate_random_transaction};
use ring::rand::SecureRandom;
use serde::{Serialize, Deserialize};
use rand::Rng;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Header {
    pub parent: H256,
    nonce: u32,
    difficulty: H256,
    timestamp: u128,
    merkle_root: H256,
}

#[derive(Serialize, Debug,Default, Clone)]
pub struct Block {
    pub header: Header,
    content: Vec<Transaction>
}

impl Hashable for Header {
    fn hash(&self) -> H256 {
        let serialized = bincode::serialize(&self).unwrap();
        ring::digest::digest(&ring::digest::SHA256, &serialized).into()
    }
}

impl Hashable for Block {
    fn hash(&self) -> H256 {
        self.header.hash()
    }
}

impl Block {
    pub fn new(parent: H256, nonce:u32, difficulty:H256, timestamp:u128,
               merkle_root:H256, content:Vec<Transaction>) -> Block {
        Block{ header: Header{ parent, nonce, difficulty, timestamp,merkle_root}, content}
    }

    pub fn get_difficulty(&self) -> H256 {
        self.header.difficulty
    }
}

pub fn generate_random_block(parent: &H256) -> Block {
    let parent_array: [u8; 32] = parent.into();

    // init random difficulty
    let sr = ring::rand::SystemRandom::new();
    let mut result = [0u8; 32];
    sr.fill(&mut result).unwrap();

    // init random transactions
    let trans:Vec<Transaction> = vec![
        generate_random_transaction().into(),
        generate_random_transaction().into(),
        generate_random_transaction().into()
    ];

    let merkle_tree = MerkleTree::new(&trans);
    let root = merkle_tree.root();

    let blk = Block::new(
        H256::from(parent_array),
        rand::thread_rng().gen::<u32>(),
        H256::from(result),
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis(),
        root,
        trans,
    );

    //println!("{:?}", blk);
    blk
}

#[cfg(any(test, test_utilities))]
pub mod test {
    use super::*;
    use crate::crypto::hash::H256;

}
