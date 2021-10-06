use serde::{Serialize,Deserialize};
use ring::signature::{Ed25519KeyPair, Signature, KeyPair};//, VerificationAlgorithm, EdDSAParameters};
use ring::digest;
use ring::rand::SecureRandom;

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Transaction {
    msg: String
}

/// Create digital signature of a transaction
pub fn sign(t: &Transaction, key: &Ed25519KeyPair) -> Signature {
    let serialized = bincode::serialize(t).unwrap();
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

#[cfg(any(test, test_utilities))]
mod tests {
    use super::*;
    use crate::crypto::key_pair;

    pub fn generate_random_transaction() -> Transaction {
        let sr = ring::rand::SystemRandom::new();
        let mut result = [0u8; 32];
        sr.fill(&mut result).unwrap();
        Transaction{msg: result[0].to_string()}
    }

    #[test]
    fn sign_verify() {
        let t = generate_random_transaction();
        let key = key_pair::random();
        let signature = sign(&t, &key);
        assert!(verify(&t, &(key.public_key()), &signature));
    }
}
