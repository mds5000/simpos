use log::{error, info};
use std::io::Read;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex, MutexGuard};
use std::time::Duration;
use std::{io, thread};

use bytes::{Buf, BufMut, Bytes};
use serialport;

pub enum MotorCmd {
    Position(i32),
    Enable(bool),
    Home,
}

impl MotorCmd {
    fn to_vec(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        match self {
            MotorCmd::Enable(en) => {
                let enable: u8 = if *en { 1 } else { 0 };
                buf.put_u8(b'e');
                buf.put_u8(enable);
                buf.put_u8(b'e');
            }
            MotorCmd::Position(pos) => {
                buf.put_u8(b'p');
                buf.put_i32(*pos);
                buf.put_u8(b'p');
            }
            MotorCmd::Home => {
                buf.put_u8(b'h');
                buf.put_u8(1);
                buf.put_u8(b'h');
            }
        }

        buf
    }

    pub fn from_bytes(buf: &[u8]) -> Self {
        let kind = buf[0];

        MotorCmd::Enable(false)
    }
}

type DataSeries = Vec<[f64; 2]>;

#[derive(Default)]
pub struct MotorTelemetry {
    pub torque_sns: DataSeries,
    pub torque_cmd: DataSeries,
    pub torque_p: DataSeries,
    pub torque_i: DataSeries,
    pub torque_d: DataSeries,

    pub speed_sns: DataSeries,
    pub speed_cmd: DataSeries,
    pub speed_p: DataSeries,
    pub speed_i: DataSeries,
    pub speed_d: DataSeries,

    pub position_sns: DataSeries,
    pub position_cmd: DataSeries,
}

pub struct MotorDriver {
    telem: Arc<Mutex<MotorTelemetry>>,
    pub command: mpsc::Sender<MotorCmd>,
    hangup: Arc<AtomicBool>,
}

impl MotorDriver {
    pub fn connect(serial: &str) -> Result<Self, serialport::Error> {
        let mut port = serialport::new(serial, 921600)
            .timeout(Duration::from_secs(1))
            .open()?;
        let telem = Arc::new(Mutex::new(MotorTelemetry::default()));
        let hangup = Arc::new(AtomicBool::new(false));
        let (tx, rx) = mpsc::channel::<MotorCmd>();

        // Reader Thread
        let rx_telem = Arc::clone(&telem);
        let rx_hangup = Arc::clone(&hangup);
        let mut rx_port = port.try_clone()?;
        let mut rx_reader = io::BufReader::new(rx_port);
        thread::spawn(move || {
            let mut buffer = [0u8; 32];
            info!("Reading");
            while !rx_hangup.load(Ordering::Relaxed) {
                match rx_reader.read_exact(&mut buffer) {
                    Ok(()) => {
                        let mut telem = rx_telem.lock().expect("Locking");
                        MotorDriver::consume_buffer(&buffer, &mut telem);
                    }
                    Err(e) => {
                        error! {"{e}"};
                    }
                }
            }
            info!("Hangup");
        });

        // Writer Thread
        thread::spawn(move || {
            for cmd in rx.iter() {
                port.write_all(&cmd.to_vec());
            }
        });

        Ok(MotorDriver {
            telem,
            command: tx,
            hangup,
        })
    }

    pub fn send_command(&mut self, cmd: MotorCmd) {
        self.command.send(cmd);
    }

    pub fn get_telemetry<'a>(&'a self) -> MutexGuard<'a, MotorTelemetry> {
        self.telem.lock().expect("Locked")
    }

    fn consume_buffer(buffer: &[u8], telem: &mut MotorTelemetry) {
        let mut buf = Bytes::copy_from_slice(buffer);
        let msg_type = buf.get_u8();
        let time = buf.get_u32() as f64;

        match msg_type {
            b'z' => {
                // Speed Telemetry
                telem.torque_sns.push([time, buf.get_f32() as f64]);
                telem.torque_cmd.push([time, buf.get_f32() as f64]);
                telem.torque_p.push([time, buf.get_f32() as f64]);
                telem.torque_i.push([time, buf.get_f32() as f64]);
                telem.torque_d.push([time, buf.get_f32() as f64]);
                telem.speed_sns.push([time, buf.get_f32() as f64]);
            }
            b'p' => {
                // Position Telemetry
                telem.speed_cmd.push([time, buf.get_f32() as f64]);
                telem.speed_p.push([time, buf.get_f32() as f64]);
                telem.speed_i.push([time, buf.get_f32() as f64]);
                telem.speed_d.push([time, buf.get_f32() as f64]);
                telem.position_cmd.push([time, buf.get_f32() as f64]);
                telem.position_sns.push([time, buf.get_i32() as f64]);
            }

            _ => {
                error!("Unknown data")
            }
        }
    }
}

impl Drop for MotorDriver {
    fn drop(&mut self) {
        self.hangup.store(true, Ordering::Relaxed);
    }
}
