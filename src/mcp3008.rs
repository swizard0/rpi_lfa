use rppal::spi::{
    Spi,
    Bus,
    Mode,
    Segment,
    SlaveSelect,
};

use super::Volt;

pub struct Mcp3008 {
    inner: Inner,
}

#[derive(Debug)]
pub enum Error {
    SpiInitialize(rppal::spi::Error),
    SpiTransferSegments(rppal::spi::Error),
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
        let spi = Spi::new(Bus::Spi0, SlaveSelect::Ss0, hz, Mode::Mode0)
            .map_err(Error::SpiInitialize)?;
        Ok(Mcp3008 { inner: Inner { spi, v_ref, buffer: [0; 3], }, })
    }

    pub fn value(&mut self, channel: Channel) -> Result<Volt, Error> {
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
        self.inner.spi.transfer_segments(
            &[Segment::new(&mut self.inner.buffer, &[0b00000001, 0b10000000 | (channel_value << 4), 0b00000000])],
        ).map_err(Error::SpiTransferSegments)?;
        let data = ((self.inner.buffer[1] & 0b00000011) as u16) << 8 | (self.inner.buffer[2] as u16);
        Ok(Volt(data as f64 * self.inner.v_ref.0 / 1024.0))
    }
}

struct Inner {
    spi: Spi,
    v_ref: Volt,
    buffer: [u8; 3],
}
