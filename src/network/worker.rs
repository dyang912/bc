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
                    println!("reveiced the broadcast new block ");
                    let mut new_blocks: Vec<H256>= Vec::new();
                    let blkchain =self.arc.lock().unwrap();
                    for hash in hashes{
                        if !blkchain.blockchain.contains_key(&hash){
                            new_blocks.push(hash);
                        }
                    }
                    if new_blocks.len()>0{
                        println!("ask for them by sending GetBlocks");
                        peer.write(Message::GetBlocks(new_blocks));

                    }
                }
                //if the hashes are in blockchain, you can get theses blocks and send them by Blocks message
                Message::GetBlocks(hashes) =>{
                    println!("reveived the request of ask for sending getblocks");
                    let mut blocks : Vec<Block> = Vec::new();
                    let blkchain =self.arc.lock().unwrap();
                    for hash in hashes{
                        if blkchain.blockchain.contains_key(&hash){
                            let temp = blkchain.blockchain.get(&hash).unwrap().clone();
                            blocks.push(temp);
                        }
                    }
                    if blocks.len()>0{
                        println!("sending blocks");
                        peer.write(Message::Blocks(blocks));
                    }
                }
                //for Blocks, insert the blocks into blockchain if not already in it
                Message::Blocks(blocks)=>{
                    for block in blocks.iter() {
                        memory.insert(block.header.parent,block.clone());
                    }

                    //don't find the parents of some blocks in #Block => #GetBlocks
                    //broadcast #NewBlockhashes when received onr from #Block
                    let mut new_hashes: Vec<H256> = Vec::new();
                    let mut no_parents :Vec::<H256> = Vec::new();
                    let mut blkchain =self.arc.lock().unwrap();
                    for block in blocks.iter() {
                        if !blkchain.blockchain.contains_key(&block.hash()){
                            let parent = &block.header.parent;
                            if blkchain.blockchain.contains_key(parent) && block.hash()<=block.header.difficulty{
                                blkchain.insert(&block.clone());
                                new_hashes.push(block.hash());
                                let mut parent: H256 = block.hash();
                                while memory.contains_key(&parent) {
                                    blkchain.insert(&(memory.get(&parent).unwrap()));
                                    let mut new_parent = memory.get(&parent).unwrap().hash();
                                    memory.remove(&parent);
                                    parent = new_parent;
                                    new_hashes.push(parent);
                                }
                                println!("insert success! bc height:{:?}", blkchain.height);
                            }else{
                                if block.hash()<=block.header.difficulty{
                                    no_parents.push(*parent);
                                }
                            }
                        }  
                    }
                    if new_hashes.len()>0{
                        println!("worker broadcast blocks");
                        self.server.broadcast(Message::NewBlockHashes(new_hashes));
                        
                    }
                    if no_parents.len()>0{
                        peer.write(Message::GetBlocks(no_parents));
                    }
                }

            }
        }
    }
}
