use crate::crypto::{merkle::MerkleTree, hash::{H256,H160, Hashable}};
use crate::transaction::{Transaction, generate_random_signedtrans,Input,Output,SignedTrans,sign};
use ring::{digest, rand::SecureRandom, signature::{Ed25519KeyPair, Signature, KeyPair}};
use serde::{Serialize, Deserialize};
use rand::Rng;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::crypto::key_pair;

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Header {
    pub parent: H256,
    nonce: u32,
    pub difficulty: H256,
    timestamp: u128,
    merkle_root: H256,
}

#[derive(Serialize, Deserialize, Debug,Default, Clone)]
pub struct Block {
    pub header: Header,
    content: Vec<SignedTrans>
}

impl Hashable for Header {
    fn hash(&self) -> H256 {
        let serialized = bincode::serialize(&self).unwrap();
        ring::digest::digest(&ring::digest::SHA256, &serialized).into()
    }
}

impl Header {
    pub fn get_create_time(&self) -> u128 {
        self.timestamp
    }
}

impl Hashable for Block {
    fn hash(&self) -> H256 {
        self.header.hash()
    }
}

impl Block {
    pub fn new(parent: H256, nonce:u32, difficulty:H256, timestamp:u128,
               merkle_root:H256, content:Vec<SignedTrans>) -> Block {
        Block{ header: Header{ parent, nonce, difficulty, timestamp,merkle_root}, content}
    }

    pub fn get_difficulty(&self) -> H256 {
        self.header.difficulty
    }
}

pub fn generate_random_block(parent: &H256) -> Block {
    let parent_array: [u8; 32] = parent.into();

    // init random difficulty
    let mut result = [0u8; 32];
    // let sr = ring::rand::SystemRandom::new();
    // sr.fill(&mut result).unwrap(); // random difficulty
    result[0] = 1;

    // init random transactions
    let trans:Vec<SignedTrans> = vec![
        generate_random_signedtrans().into(),
        generate_random_signedtrans().into(),
        generate_random_signedtrans().into()
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

pub fn generate_genesis_block(parent: &H256) -> Block {
    let parent_array: [u8; 32] = parent.into();

    // init random difficulty
    let mut result = [0u8; 32];
    // let sr = ring::rand::SystemRandom::new();
    // sr.fill(&mut result).unwrap(); // random difficulty
    result[0] = 1;

    let index =1;
    let previous_hash = H256::from([0u8; 32]);
    let inputs = Input{index, previous_hash};
    let val =1;
    let address = H160::from([0u8;20]);
    let outputs = Output{val, address};
    let transaction=Transaction{ header: Default::default(),inputs:vec![inputs], outputs:vec![outputs]};

    let key = key_pair::random();
    let s = sign(&transaction, &key);
    let p = key.public_key().as_ref().to_vec();

    let signedtrans = SignedTrans{transaction,signature:s,public_key:p};

    // init  Signedtransactions
    let trans:Vec<SignedTrans> = vec![
        generate_random_signedtrans().into(),
    ];



    let merkle_tree = MerkleTree::new(&trans);
    let root = merkle_tree.root();

    let blk = Block::new(
        H256::from(parent_array),
        0,
        H256::from(result),
        0,
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
