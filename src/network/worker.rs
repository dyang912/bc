use super::message::Message;
use super::peer;
use crate::network::server::Handle as ServerHandle;
use crossbeam::channel;
use log::{debug, warn};
use std::sync::{Arc, Mutex};
use crate::block::Block;
use std::collections::HashMap;
use crate::crypto::hash::{H256, Hashable};
use crate::blockchain::Blockchain;
use std::time::{SystemTime, UNIX_EPOCH};

use std::thread;

#[derive(Clone)]
pub struct Context {
    msg_chan: channel::Receiver<(Vec<u8>, peer::Handle)>,
    num_worker: usize,
    server: ServerHandle,
    arc: Arc<Mutex<Blockchain>>,
}

pub fn new(
    num_worker: usize,
    msg_src: channel::Receiver<(Vec<u8>, peer::Handle)>,
    server: &ServerHandle,
    arc: &Arc<Mutex<Blockchain>>
) -> Context {
    Context {
        msg_chan: msg_src,
        num_worker,
        server: server.clone(),
        arc: Arc::clone(arc),
    }
}

impl Context {
    pub fn start(self) {
        let num_worker = self.num_worker;
        for i in 0..num_worker {
            let cloned = self.clone();
            thread::spawn(move || {
                cloned.worker_loop();
                warn!("Worker thread {} exited", i);
            });
        }
    }

    fn worker_loop(&self) {

        let mut memory: HashMap<H256,Block>= HashMap::new(); // parent's hash and dangling block

        loop {
            let msg = self.msg_chan.recv().unwrap();
            let (msg, peer) = msg;
            let msg: Message = bincode::deserialize(&msg).unwrap();
            match msg {
                Message::Ping(nonce) => {
                    debug!("Ping: {}", nonce);
                    peer.write(Message::Pong(nonce.to_string()));
                }
                Message::Pong(nonce) => {
                    debug!("Pong: {}", nonce);
                }
                //For NewBlockHashes, if the hashes are not already in blockchain, you need to ask for them by sending GetBlocks.
                Message::NewBlockHashes(hashes) => {
                    let mut dic: HashMap<H256, u32> = HashMap::new();
                    let blkchain =self.arc.lock().unwrap();

                    for hash in hashes{
                        if !blkchain.blockchain.contains_key(&hash){
                            dic.insert(hash, 1);
                        }
                    }

                    if dic.len()>0{
                        let mut new_blocks: Vec<H256>= Vec::new();
                        for item in dic {
                            new_blocks.push(item.0);
                        }
                        peer.write(Message::GetBlocks(new_blocks));
                    }
                }
                //if the hashes are in blockchain, you can get theses blocks and send them by Blocks message
                Message::GetBlocks(hashes) =>{
                    let mut dic: HashMap<H256, u32> = HashMap::new();
                    for hash in hashes{
                        dic.insert(hash, 1);
                    }
                    let mut blocks : Vec<Block> = Vec::new();
                    let blkchain =self.arc.lock().unwrap();
                    for item in dic{
                        let hash = item.0;
                        if blkchain.blockchain.contains_key(&hash){
                            let temp = blkchain.blockchain.get(&hash).unwrap().clone();
                            blocks.push(temp);
                        }
                    }
                    if blocks.len()>0{
                        peer.write(Message::Blocks(blocks));
                    }
                }
                //for Blocks, insert the blocks into blockchain if not already in it
                Message::Blocks(blocks)=>{
                    //don't find the parents of some blocks in #Block => #GetBlocks
                    //broadcast #NewBlockhashes when received onr from #Block
                    let mut dic_new: HashMap<H256, u32> = HashMap::new();
                    let mut dic_no_parent: HashMap<H256, u32> = HashMap::new();
                    let mut blkchain =self.arc.lock().unwrap();

                    for block in blocks.iter() {
                        if !blkchain.contain(block.hash()) {
                            memory.insert(block.header.parent,block.clone());
                        }
                    }

                    for block in blocks.iter() {
                        if !blkchain.blockchain.contains_key(&block.hash()){
                            let new_block_parent = &block.header.parent;
                            if blkchain.blockchain.contains_key(new_block_parent) && block.hash() <= block.header.difficulty{
                                blkchain.insert(&block.clone());
                                memory.remove(&block.header.parent);
                                println!("{:?} insert {:?}! bc height:{:?}", SystemTime::now().duration_since(SystemTime::UNIX_EPOCH), block.hash(), blkchain.height);
                                dic_new.insert(block.hash(), 1);
                                // insert all children stored in memory
                                let mut inserted: H256 = block.hash();
                                while memory.contains_key(&inserted) {
                                    let next_insert = memory.get(&inserted).unwrap().clone();
                                    blkchain.insert(&next_insert.clone());
                                    println!("{:?} insert ch {:?}! bc height:{:?}", SystemTime::now().duration_since(SystemTime::UNIX_EPOCH), next_insert.hash(), blkchain.height);
                                    memory.remove(&inserted);
                                    inserted = next_insert.hash();
                                    dic_new.insert(inserted, 1);
                                }
                            } else if block.hash()<=block.header.difficulty {
                                dic_no_parent.insert(*new_block_parent, 1);
                            }
                        }  
                    }
                    if dic_new.len()>0{
                        let mut new_hashes: Vec<H256> = Vec::new();
                        for item in dic_new {
                            new_hashes.push(item.0);
                        }
                        self.server.broadcast(Message::NewBlockHashes(new_hashes));
                    }
                    if dic_no_parent.len()>0{
                        let mut no_parents :Vec::<H256> = Vec::new();
                        for item in dic_no_parent {
                            no_parents.push(item.0);
                        }
                        peer.write(Message::GetBlocks(no_parents));
                    }
                }

            }
        }
    }
}
