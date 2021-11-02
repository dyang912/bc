use std::ops::Sub;
use crate::network::server::Handle as ServerHandle;
use std::sync::{Arc, Mutex};
use crate::blockchain::Blockchain;
use crate::block::Block;
use crate::crypto::merkle::MerkleTree;
use crate::transaction::{Transaction, generate_random_transaction};
use std::time::{SystemTime, UNIX_EPOCH};
use rand::Rng;
use crate::network::message::Message;


use log::info;

use crossbeam::channel::{unbounded, Receiver, Sender, TryRecvError};
use std::time;

use std::thread;
use crate::crypto::hash::Hashable;

enum ControlSignal {
    Start(u64), // the number controls the lambda of interval between block generation
    Exit,
}

enum OperatingState {
    Paused,
    Run(u64),
    ShutDown,
}

pub struct Context {
    /// Channel for receiving control signal
    control_chan: Receiver<ControlSignal>,
    operating_state: OperatingState,
    server: ServerHandle,
    arc: Arc<Mutex<Blockchain>>,
    mined: u32,
    inserted: u32,
    start_time: SystemTime,
}

#[derive(Clone)]
pub struct Handle {
    /// Channel for sending signal to the miner thread
    control_chan: Sender<ControlSignal>,
}

pub fn new(
    server: &ServerHandle,
    arc: &Arc<Mutex<Blockchain>>
) -> (Context, Handle) {
    let (signal_chan_sender, signal_chan_receiver) = unbounded();

    let ctx = Context {
        control_chan: signal_chan_receiver,
        operating_state: OperatingState::Paused,
        server: server.clone(),
        arc: Arc::clone(arc),
        mined: 0,
        inserted: 0,
        start_time: SystemTime::now(),
    };

    let handle = Handle {
        control_chan: signal_chan_sender,
    };

    (ctx, handle)
}

impl Handle {
    pub fn exit(&self) {
        self.control_chan.send(ControlSignal::Exit).unwrap();
    }

    pub fn start(&self, lambda: u64) {
        self.control_chan
            .send(ControlSignal::Start(lambda))
            .unwrap();
    }

}

impl Context {
    pub fn start(mut self) {
        thread::Builder::new()
            .name("miner".to_string())
            .spawn(move || {
                self.miner_loop();
            })
            .unwrap();
        info!("Miner initialized into paused mode");
    }

    fn handle_control_signal(&mut self, signal: ControlSignal) {
        match signal {
            ControlSignal::Exit => {
                info!("Miner shutting down");
                self.operating_state = OperatingState::ShutDown;
            }
            ControlSignal::Start(i) => {
                info!("Miner starting in continuous mode with lambda {}", i);
                self.start_time = SystemTime::now();
                // println!("---------- start :{:?}", SystemTime::now());
                self.operating_state = OperatingState::Run(i);
            }
        }
    }

    fn miner_loop(&mut self) {
        // main mining loop
        loop {
            // check and react to control signals
            match self.operating_state {
                OperatingState::Paused => {
                    let signal = self.control_chan.recv().unwrap();
                    self.handle_control_signal(signal);
                    continue;
                }
                OperatingState::ShutDown => {
                    return;
                }
                _ => match self.control_chan.try_recv() {
                    Ok(signal) => {
                        self.handle_control_signal(signal);
                    }
                    Err(TryRecvError::Empty) => {}
                    Err(TryRecvError::Disconnected) => panic!("Miner control channel detached"),
                },
            }
            if let OperatingState::ShutDown = self.operating_state {
                return;
            }

            // get parent
            let mut bc = self.arc.lock().unwrap();
            let parent = bc.tip();

            // get timestamp
            let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();

            // get difficulty
            let difficulty = bc.get_difficulty();

            // generate merkle root
            let trans:Vec<Transaction> = vec![generate_random_transaction().into()];
            let merkle_tree = MerkleTree::new(&trans);
            let root = merkle_tree.root();

            // generate nonce
            let nonce = rand::thread_rng().gen::<u32>();

            let blk = Block::new(parent,nonce,difficulty,timestamp,root,trans);

            self.mined += 1;
            // if self.mined % 100 == 0 {
            //     println!("{:?} {}", difficulty, self.mined);
            // }
            if blk.hash() <= difficulty {
                bc.insert(&blk);
                self.inserted += 1;
                let mut block_vec = Vec::new();
                block_vec.push(blk.hash());
                let msg = Message::NewBlockHashes(block_vec);
                self.server.broadcast(msg);
                println!("insert success! {}, {}/{}", bc.get_length(), self.inserted, self.mined);
            }

            if SystemTime::now().duration_since(self.start_time).unwrap().as_secs() >= 360 {
                println!("---------- result : {:?}, {}/{}, {:?}", difficulty, self.inserted, self.mined, SystemTime::now());
                break
            }

            if let OperatingState::Run(i) = self.operating_state {
                if i != 0 {
                    let interval = time::Duration::from_micros(i as u64);
                    thread::sleep(interval);
                }
            }
        }
    }
}
