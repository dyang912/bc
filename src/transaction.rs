use serde::{Serialize, Deserialize};
use rand::Rng;
use crate::crypto::key_pair;
use std::collections::HashMap;
use ring::{digest, rand::SecureRandom, signature::{Ed25519KeyPair, Signature, KeyPair}};
use crate::crypto::hash::{H256,H160,Hashable, generate_rand_hash256,generate_rand_hash160};

// const MAX_LEN: usize = 1/*tag:SEQUENCE*/ + 2/*len*/ +
//     (2 * (1/*tag:INTEGER*/ + 1/*len*/ + 1/*zero*/ + (384 + 7) / 8));

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Header {
    msg: String
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Input {
    pub index: u8,
    pub previous_hash: H256,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Output {
    pub val: u8,
    pub address: H160
}


#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Transaction {
    pub header: Header,
    pub inputs: Vec<Input>,
    pub outputs: Vec<Output>
    //sig: Option<[u8; MAX_LEN]>
}


#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct SignedTrans {
    pub transaction: Transaction,
    pub signature: Vec<u8>,
    pub public_key: Vec<u8>,
}

impl Hashable for SignedTrans {
    fn hash(&self) -> H256 {
        //unimplemented!()
        let encoded: Vec<u8> = bincode::serialize(&self).unwrap();
        let mut cat = digest::Context::new(&digest::SHA256);
        cat.update(&encoded);
        let fin = cat.finish();
        let val = <H256>::from(fin);
        val
    }
}
/// Create digital signature of a transaction
pub fn sign(t: &Transaction, key: &Ed25519KeyPair) ->  Vec<u8> {
    let serialized = bincode::serialize(&t.header).unwrap();
    let msg = digest::digest(&digest::SHA256, &serialized);
    let sig =key.sign(msg.as_ref()).as_ref().to_vec();
    return sig;
}

/// Verify digital signature of a transaction, using public key instead of secret key
pub fn verify(t: &Transaction, public_key: &<Ed25519KeyPair as KeyPair>::PublicKey, signature: &Signature) -> bool {
    let serialized = bincode::serialize(t).unwrap();
    let msg = digest::digest(&digest::SHA256, &serialized);
    let peer_public_key = ring::signature::UnparsedPublicKey::new(&ring::signature::ED25519, public_key.as_ref());
    peer_public_key.verify(msg.as_ref(), signature.as_ref()).is_ok()
}

impl Hashable for Transaction {
    fn hash(&self) -> H256 {
        let serialized = bincode::serialize(&self).unwrap();
        ring::digest::digest(&ring::digest::SHA256, &serialized).into()
    }
}

pub fn generate_random_transaction() -> Transaction {
    let sr = ring::rand::SystemRandom::new();
    let mut rng = rand::thread_rng();
    let mut result = [0u8; 32];
    sr.fill(&mut result).unwrap();
    let hash:H256 = generate_rand_hash256();
    let index:u8 = rng.gen();
    let inputs = Input{index, previous_hash:hash};
    let val:u8 = rng.gen();
    let address = generate_rand_hash160();
    let outputs = Output{val, address};
    let trans = Transaction{ header: Header{ msg: result[0].to_string()},inputs:vec![inputs], outputs:vec![outputs] };
    // let key = key_pair::random();
    // trans.sig = Some(*sign(&trans, &key).as_ref());
    trans
}

pub fn generate_random_signedtrans() -> SignedTrans{
    let key = key_pair::random();
    let t = generate_random_transaction();
    let s = sign(&t, &key);
    let p = key.public_key().as_ref().to_vec();
    SignedTrans{
        transaction: t,
        signature: s,
        public_key: p,
    }
}


#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Mempool {
    pub pool: HashMap<H256, SignedTrans>,
}

impl Mempool {
    pub fn new() -> Self{
        let m =Mempool {
            pool: HashMap::new()
        };
        m
    }

    pub fn add(&mut self, signed: &SignedTrans) {
        let map = self.clone().pool;
        let hash = signed.hash();
        if !map.contains_key(&hash){
            self.pool.insert(hash, signed.clone());
        };
    }

    pub fn remove(&mut self, signed: &SignedTrans) {
        let map = self.clone().pool;
        let hash = signed.hash();
        if map.contains_key(&hash) {
            let res = self.pool.remove(&hash);
        }
        return
    }
}

#[cfg(any(test, test_utilities))]
mod tests {
    use super::*;
    use crate::crypto::key_pair;

    #[test]
    fn sign_verify() {
        let t = generate_random_transaction();
        let key = key_pair::random();
        let signature = sign(&t, &key);
        // assert!(verify(&t, &(key.public_key()), &signature));
    }
}
