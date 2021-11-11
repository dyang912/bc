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

use std::thread;
use std::time::SystemTime;

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

        let mut memory:HashMap<H256,Block>= HashMap::new(); // parent's hash and dangling block
        let mut total_delay:u128 = 0;
        let mut recevied:u128 = 0;

        loop {
            let msg = self.msg_chan.recv().unwrap();
            let (msg, peer) = msg;
            let msg: Message = bincode::deserialize(&msg).unwrap();
            let mut blkchain =self.arc.lock().unwrap();
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
                    let mut need_parent = Vec::new();
                    for hash in hashes{
                        if !blkchain.blocks.contains_key(&hash){
                            need_parent.push(hash.clone());
                        }
                    }
                    if need_parent.len()>0{
                        peer.write(Message::GetBlocks(need_parent));
                    }
                }
                //if the hashes are in blockchain, you can get theses blocks and send them by Blocks message
                Message::GetBlocks(hashes) =>{
                    let mut blocks : Vec<Block> = Vec::new();
                    for item in hashes {
                        let hash = item;
                        if blkchain.blocks.contains_key(&hash){
                            let temp = blkchain.blocks.get(&hash).unwrap().clone();
                            blocks.push(temp.0);
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
                    let mut new_blocks = Vec::new();
                    let mut need_parent = Vec::new();
                    let ts = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap();

                    for block in blocks.iter() {
                        if !blkchain.blocks.contains_key(&block.hash()) {
                            memory.insert(block.header.parent,block.clone());
                            total_delay += ts.as_millis() - block.header.get_create_time();
                            recevied += 1;
                        }
                    }

                    for block in blocks.iter() {
                        if !blkchain.blocks.contains_key(&block.hash()){
                            let new_block_parent = &block.header.parent;
                            // PoW validity check
                            if block.hash() <= block.header.difficulty {
                                // Parent check
                                if blkchain.blocks.contains_key(new_block_parent) &&
                                    block.hash() < blkchain.blocks.get(new_block_parent).unwrap().0.header.difficulty {
                                    blkchain.insert(&block.clone());
                                    memory.remove(&block.header.parent);
                                    new_blocks.push(block.hash().clone());

                                    // Orphan block handler: insert validated blocks stored in memory
                                    let mut inserted: H256 = block.hash();
                                    while memory.contains_key(&inserted) {
                                        let next_insert = memory.get(&inserted).unwrap().clone();
                                        blkchain.insert(&next_insert.clone());
                                        memory.remove(&inserted);
                                        inserted = next_insert.hash();
                                        new_blocks.push(inserted.clone());
                                    }
                                } else {
                                    need_parent.push(new_block_parent.clone());
                                    peer.write(Message::GetBlocks(need_parent.clone()));
                                }
                            }
                        }  
                    }
                    if new_blocks.len()>0{
                        self.server.broadcast(Message::NewBlockHashes(new_blocks));
                    }
                    if recevied > 0 {
                        println!("avg delay:{:?}/{:?}={:?}", total_delay, recevied, total_delay / recevied);
                    }
                }
            }
        }
    }
}
