use serde::{Serialize, Deserialize};
use ring::{digest, rand::SecureRandom, signature::{Ed25519KeyPair, Signature, KeyPair}};
use crate::crypto::{key_pair, hash::{H256, Hashable}};

const MAX_LEN: usize = 1/*tag:SEQUENCE*/ + 2/*len*/ +
    (2 * (1/*tag:INTEGER*/ + 1/*len*/ + 1/*zero*/ + (384 + 7) / 8));

#[derive(Serialize, Deserialize, Debug, Default,Clone)]
struct Header {
    msg: String
}

#[derive(Serialize, Deserialize, Debug, Default,Clone)]
pub struct Transaction {
    header: Header,
    //sig: Option<[u8; MAX_LEN]>
}

/// Create digital signature of a transaction
pub fn sign(t: &Transaction, key: &Ed25519KeyPair) -> Signature {
    let serialized = bincode::serialize(&t.header).unwrap();
    let msg = digest::digest(&digest::SHA256, &serialized);
    key.sign(msg.as_ref())
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
    let mut result = [0u8; 32];
    sr.fill(&mut result).unwrap();
    let mut trans = Transaction{ header: Header{ msg: result[0].to_string()} };
    // let key = key_pair::random();
    // trans.sig = Some(*sign(&trans, &key).as_ref());
    trans
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
        assert!(verify(&t, &(key.public_key()), &signature));
    }
}
