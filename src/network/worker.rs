use super::message::Message;
use super::peer;
use crate::network::server::Handle as ServerHandle;
use crossbeam::channel;
use log::{debug, warn};
use std::sync::{Arc, Mutex};
use crate::block::Block;
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
                    let mut new_blocks: Vec<H256>= Vec::new();
                    let blkchain =self.arc.lock().unwrap();
                    for hash in hashes{
                        if !blkchain.blockchain.contains_key(&hash){
                            new_blocks.push(hash);
                        }
                    }
                    if new_blocks.len()>0{
                        peer.write(Message::GetBlocks(new_blocks));
                    }
                }
                //if the hashes are in blockchain, you can get theses blocks and send them by Blocks message
                Message::GetBlocks(hashes) =>{
                    let mut blocks : Vec<Block> = Vec::new();
                    let blkchain =self.arc.lock().unwrap();
                    for hash in hashes{
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
                    let mut new_hashes: Vec<H256> = Vec::new();
                    // let mut no_parents :Vec::<H256> = Vec::new();
                    let mut blkchain =self.arc.lock().unwrap();
                    for block in blocks {
                        if !blkchain.blockchain.contains_key(&block.hash()){
                            let parent = &block.header.parent;
                            if blkchain.blockchain.contains_key(parent) && block.hash()<=block.header.difficulty{
                                blkchain.blockchain.insert(block.hash(),block.clone());
                                new_hashes.push(block.hash());
                            }
                            // }else{
                            //     if block.hash()<=block.header.difficulty{
                            //         no_parents.push(*parent);
                            //     }
                            // }
                        }  
                    }
                    if new_hashes.len()>0{
                        self.server.broadcast(Message::NewBlockHashes(new_hashes));
                    }
                    // if no_parents.len()>0{
                    //     peer.write(Message::GetBlocks(no_parents));
                    // }
                }

            }
        }
    }
}
