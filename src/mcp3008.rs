use std::{
    io,
    thread,
    sync::mpsc,
};

use rppal::spi::{
    Spi,
    Bus,
    Mode,
    Segment,
    SlaveSelect,
};

use super::Volt;

pub enum Mcp3008 {
    Initializing(Initializing),
    Ready(Ready),
    Probing(Probing),
}

#[derive(Clone, Debug)]
pub struct Params {
    pub voltage_drain: Vdd,
    pub voltage_ref: Vref,
}

#[derive(Clone, Debug)]
pub enum Vdd {
    Positive3v3,
    Positive5v,
}

#[derive(Clone, Debug)]
pub enum Vref {
    EqualToVdd,
    Other { voltage: Volt, },
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Channel {
    Ch0,
    Ch1,
    Ch2,
    Ch3,
    Ch4,
    Ch5,
    Ch6,
    Ch7,
}

#[derive(Debug)]
pub enum Error {
    SpiThreadSpawn(io::Error),
    SpiThreadLost,
    SpiInitialize(rppal::spi::Error),
    SpiTransferSegments(rppal::spi::Error),
}

impl Mcp3008 {
    pub fn new(params: &Params) -> Result<Self, Error> {
        let hz = match params.voltage_drain {
            Vdd::Positive3v3 =>
                1_350_000,
            Vdd::Positive5v =>
                3_600_000,
        };
        let v_ref = match params.voltage_ref {
            Vref::EqualToVdd =>
                match params.voltage_drain {
                    Vdd::Positive3v3 =>
                        Volt(3.3),
                    Vdd::Positive5v =>
                        Volt(5.0),
                },
            Vref::Other { voltage, } =>
                voltage,
        };

        let (request_tx, request_rx) = mpsc::sync_channel(0);
        let (event_tx, event_rx) = mpsc::sync_channel(0);

        let builder = thread::Builder::new()
            .name("Mcp3008 spi".into())
            .spawn(move || spi_worker(request_rx, event_tx, hz, v_ref))
            .map_err(Error::SpiThreadSpawn)?;

        Ok(Mcp3008::Initializing(Initializing {
            inner: Inner { request_tx, event_rx, },
        }))
    }

    // pub fn probe(&self) -> Result<Op, Error> {
    // }
}

// Initializing

pub struct Initializing {
    inner: Inner,
}

impl From<Initializing> for Mcp3008 {
    fn from(state: Initializing) -> Mcp3008 {
        Mcp3008::Initializing(state)
    }
}

impl Initializing {
    pub fn probe(self) -> Result<InitializingOp, Error> {
        match self.inner.event_rx.try_recv() {
            Ok(Event::SpiInitialized) =>
                Ok(InitializingOp::Ready(Ready { inner: self.inner, })),
            Ok(Event::Error(error)) =>
                Err(error),
            Err(mpsc::TryRecvError::Empty) =>
                Ok(InitializingOp::Idle(self)),
            Err(mpsc::TryRecvError::Disconnected) =>
                Err(Error::SpiThreadLost),
        }

    }

}

pub enum InitializingOp {
    Idle(Initializing),
    Ready(Ready),
}

// Ready

pub struct Ready {
    inner: Inner,
}

impl From<Ready> for Mcp3008 {
    fn from(state: Ready) -> Mcp3008 {
        Mcp3008::Ready(state)
    }
}

impl Ready {
    pub fn probe_channel(self, channel: Channel) -> Probing {
        let channel_value = match channel {
            Channel::Ch0 => 0,
            Channel::Ch1 => 1,
            Channel::Ch2 => 2,
            Channel::Ch3 => 3,
            Channel::Ch4 => 4,
            Channel::Ch5 => 5,
            Channel::Ch6 => 6,
            Channel::Ch7 => 7,
        };
        Probing {
            channel_value,
            state: ProbingState::Request,
            inner: self.inner,
        }
    }
}

// Probing

pub struct Probing {
    channel_value: u8,
    state: ProbingState,
    inner: Inner,
}

impl From<Probing> for Mcp3008 {
    fn from(state: Probing) -> Mcp3008 {
        Mcp3008::Probing(state)
    }
}

enum ProbingState {
    Request,
}

impl Probing {
    pub fn poll(self) -> Result<ProbingOp, Error> {

        Ok(ProbingOp::Idle(self))
    }
}

pub enum ProbingOp {
    Idle(Probing),
}

// impl

struct Inner {
    request_tx: mpsc::SyncSender<Request>,
    event_rx: mpsc::Receiver<Event>,
}

enum Request {
}

enum Event {
    SpiInitialized,
    Error(Error),
}

fn spi_worker(request_rx: mpsc::Receiver<Request>, event_tx: mpsc::SyncSender<Event>, hz: u32, v_ref: Volt) {
    if let Err(error) = spi_worker_loop(request_rx, &event_tx, hz, v_ref) {
        event_tx.send(Event::Error(error)).ok();
    }
}

fn spi_worker_loop(
    request_rx: mpsc::Receiver<Request>,
    event_tx: &mpsc::SyncSender<Event>,
    hz: u32,
    v_ref: Volt,
)
    -> Result<(), Error>
{
    let spi = Spi::new(Bus::Spi0, SlaveSelect::Ss0, hz, Mode::Mode0)
        .map_err(Error::SpiInitialize)?;

// struct Inner {
//     spi: Spi,
//     v_ref: Volt,
//     buffer: [u8; 3],
// }

    unimplemented!()
}

//     pub fn value(&mut self, channel: Channel) -> Result<Volt, Error> {
//         let channel_value = match channel {
//             Channel::Ch0 => 0,
//             Channel::Ch1 => 1,
//             Channel::Ch2 => 2,
//             Channel::Ch3 => 3,
//             Channel::Ch4 => 4,
//             Channel::Ch5 => 5,
//             Channel::Ch6 => 6,
//             Channel::Ch7 => 7,
//         };
//         self.inner.spi.transfer_segments(
//             &[Segment::new(&mut self.inner.buffer, &[0b00000001, 0b10000000 | (channel_value << 4), 0b00000000])],
//         ).map_err(Error::SpiTransferSegments)?;
//         let data = ((self.inner.buffer[1] & 0b00000011) as u16) << 8 | (self.inner.buffer[2] as u16);
//         Ok(Volt(data as f64 * self.inner.v_ref.0 / 1024.0))
//     }
// }
