use std::borrow::Borrow;
use crate::block::Block;
use crate::crypto::hash::{H256, Hashable};
use std::collections::HashMap;
use crate::block::{generate_random_block, generate_genesis_block};

#[derive(Debug)]
pub struct Blockchain {
    pub blockchain: HashMap<H256,Block>, //blocks in the blockchain
    blocks: HashMap<H256,(Block,u32)>, //all blocks in the network, u32 refers to the height of that block
    pub height: u32,
    tip: H256
}

impl Blockchain {
    /// Create a new blockchain, only containing the genesis block
    pub fn new() -> Self {
        let mut blocks = HashMap::new();
        let mut blockchain = HashMap::new();

        let genesis = generate_genesis_block(&H256::from([0u8; 32]));

        let hashvalue = genesis.hash();
        blocks.insert(hashvalue,(genesis.clone(),0));
        blockchain.insert(hashvalue,genesis.clone());
        Blockchain{
            blockchain,
            blocks,
            height: 0,
            tip: hashvalue
        }
    }

    /// Insert a block into blockchain
    pub fn insert(&mut self, block: &Block) {
        let newblock = block.clone();
        let parent = &newblock.header.parent;
        let mut nheight =0;

        //The parent of the newly inserted block is the tip of the blockchain, insert new block directly
        if parent == &self.tip {
            self.tip = newblock.hash();
            self.height = self.height+1;
            nheight = self.height;
            self.blockchain.insert(self.tip, block.clone());
        //after insert this block, another branch becomes the longest chain
        } else if self.height < self.blocks.get(&parent).unwrap().1 +1 {
            nheight = self.blocks.get(&parent).unwrap().1 +1;
            self.height = nheight;
            //update blockchain
            let mut new_chain: Vec<H256> = Vec::new(); //the last one element's parent is in the blockchain 
            let mut current_block = &newblock;
            let mut latest_parent = &current_block.header.parent;
            while !self.blockchain.contains_key(&latest_parent){
                current_block = &self.blocks.get(&latest_parent).unwrap().0;
                latest_parent = &current_block.header.parent;  
                new_chain.push(current_block.hash()); 
            }
            //remove the blocks from blockchain
            while self.tip != self.blockchain.get(&latest_parent).unwrap().hash(){ 
                self.blockchain.remove_entry(&self.tip);
                self.tip = self.blocks.get(&self.tip).unwrap().0.header.parent; 
            }
            //insert the blocks in new_chain into blockchain
            let mut temp: Block;
            for i in new_chain.iter().rev(){ 
                temp = self.blocks.get(&i).unwrap().0.clone();
                self.blockchain.insert(*i, temp);
            }
            self.tip = newblock.hash();
            self.blockchain.insert(self.tip, block.clone());
        } else {
            //the blockchain doestn't change, only insert new block into blocks
            nheight = self.blocks.get(&parent).unwrap().1 +1;
        }
        self.blocks.insert(newblock.hash(), (block.clone(), nheight));
    }

    /// Get the last block's hash of the longest chain
    pub fn tip(&self) -> H256 {
        self.tip
    }

    pub fn get_difficulty(&self) -> H256 {
        self.blockchain.get(self.tip.borrow()).unwrap().get_difficulty()
    }

    pub fn get_length(&self) -> u32 {
        self.height
    }

    pub fn contain(&self, h:H256) -> bool {
        self.blockchain.contains_key(&h)
    }

    /// Get all blocks' hash of the longest chain
    #[cfg(any(test, test_utilities))]
    pub fn all_blocks_in_longest_chain(&self) -> Vec<H256> {
        let mut block_hash: Vec<H256> = Vec::new();
        for block in self.blockchain.iter() {
            block_hash.push(*block.0);
        }
        block_hash
    }
}

#[cfg(any(test, test_utilities))]
mod tests {
    use super::*;
    use crate::crypto::hash::Hashable;

    #[test]
    fn insert_one() {
        let mut blockchain = Blockchain::new();
        let genesis_hash = blockchain.tip();
        let block = generate_random_block(&genesis_hash);
        blockchain.insert(&block);
        assert_eq!(blockchain.tip(), block.hash());

    }

    #[test]
      fn verify_several() {
        let mut t : HashMap<i32, i32> = HashMap::new();
        t.insert(1, 2);
        t.insert(2, 3);
        t.insert(3, 2);
        t.insert(4, 1);
        println!("----------{:?}", t.len());
        let mut i = 1;
        while t.contains_key(&i) {
            let next = t.get(&i).unwrap().clone();
            t.remove(&i);
            i = next;
        }
        println!("----------{:?}", t.len());

        let mut blockchain = Blockchain::new();
        let genesis_hash = blockchain.tip();
        let block = generate_random_block(&genesis_hash);
        let block2 = generate_random_block(&genesis_hash);
        let block3 = generate_random_block(&block2.hash());
        let block4 = generate_random_block(&block.hash());
        let block5 = generate_random_block(&block3.hash());
        blockchain.insert(&block);
        blockchain.insert(&block2);
        blockchain.insert(&block3);
        blockchain.insert(&block4);
        blockchain.insert(&block5);
        let result = blockchain.all_blocks_in_longest_chain();
        for i in 0..result.len() {
              println!("{}", result[i]);
            }
        assert_eq!(result, vec![ genesis_hash, block2.hash(), block3.hash(), block5.hash()]);
      }
}
