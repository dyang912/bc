use crate::network::server::Handle as ServerHandle;
use std::sync::{Arc, Mutex};
use crate::blockchain::Blockchain;
use crate::block::Block;
use crate::crypto::merkle::MerkleTree;
use crate::transaction::{Transaction, generate_random_transaction};
use std::time::{SystemTime, UNIX_EPOCH};
use rand::Rng;

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
        arc: arc.clone(),
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

            // TODO: actual mining

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

            println!("{:?}, {:?}", difficulty, blk);
            if blk.hash() <= difficulty {
                bc.insert(&blk);
                println!("insert success! {:?}", bc);
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
