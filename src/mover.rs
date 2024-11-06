use std::net::{UdpSocket};
use std::sync::mpsc::Sender;
use std::thread;

use crate::motor::MotorCmd;
use log::info;

pub struct MoverConnection {
    sock: UdpSocket,
}

impl MoverConnection {
    pub fn new(address: &str) -> Result<Self, std::io::Error> {
        let sock = UdpSocket::bind(address)?;

        Ok(MoverConnection {
            sock
        })
    }

    pub fn connect_to_motor(&mut self, output: &Sender<MotorCmd>) {

        let mut sender = output.clone();
        let socket = self.sock.try_clone().expect("Clone");

        thread::spawn(move || {
            let mut buffer = [0u8; 128];
            loop {
                if let Ok(n) = socket.recv(&mut buffer) {
                    if n == 5  && buffer[0] == b'p'{
                        let pos = f32::from_le_bytes(buffer[1..5].try_into().unwrap()) * 200.0;
                        sender.send(MotorCmd::Position(pos.round() as i32));
                        info!("POS: {}", pos);
                    } else {
                        info!("msg {:?}", &buffer[0..n]);
                    }
                }
            }
        });

    }
}