use std::sync::{Arc, Mutex};
use std::time::SystemTime;
use crate::network::server::Handle as ServerHandle;
use crate::blockchain::Blockchain;
use crate::signedtrans::generate_random_signedtrans;
use crate::network::message::Message;
use crate::mempool::Mempool;


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
    bc: Arc<Mutex<Blockchain>>,
    mp: Arc<Mutex<Mempool>>,
    start_time: SystemTime,
}

#[derive(Clone)]
pub struct Generator {
    /// Channel for sending signal to the miner thread
    control_chan: Sender<ControlSignal>,
}

pub fn new(
    server: &ServerHandle,
    bc: &Arc<Mutex<Blockchain>>,
    mp: &Arc<Mutex<Mempool>>
) -> (Context, Generator) {
    let (signal_chan_sender, signal_chan_receiver) = unbounded();

    let ctx = Context {
        control_chan: signal_chan_receiver,
        operating_state: OperatingState::Paused,
        server: server.clone(),
        bc: Arc::clone(bc),
        mp: Arc::clone(mp),
        start_time: SystemTime::now(),
    };

    let generator = Generator {
        control_chan: signal_chan_sender,
    };

    (ctx, generator)
}

impl Generator {
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
        info!("Generator initialized into paused mode");
    }

    fn handle_control_signal(&mut self, signal: ControlSignal) {
        match signal {
            ControlSignal::Exit => {
                info!("Generator shutting down");
                self.operating_state = OperatingState::ShutDown;
            }
            ControlSignal::Start(i) => {
                info!("Generator starting in continuous mode with lambda {}", i);
                self.start_time = SystemTime::now();
                // println!("---------- start :{:?}", SystemTime::now());
                self.operating_state = OperatingState::Run(i);
            }
        }
    }

    fn miner_loop(&mut self) {
        let mut mined_size:usize = 0;
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

            // get blockchain state
            let bc = self.bc.lock().unwrap().clone();
            let state = bc.current_state;

            // generate trans using state (may be invalid)
            let trans = generate_random_signedtrans();

            // get mempool
            let mut mp = self.mp.lock().unwrap();

            // add to mempool
            mp.add(&trans);
            drop(mp);

            // broadcast
            let msg = Message::NewTransactionHashes(vec![trans.hash()]);
            self.server.broadcast(msg);

            if let OperatingState::Run(i) = self.operating_state {
                if i != 0 {
                    let interval = time::Duration::from_micros(i as u64);
                    thread::sleep(interval);
                }
            }
        }
    }
}
