
pub mod mcp3008;
pub mod freq_driver;

#[derive(Clone, Copy, PartialEq, PartialOrd, Debug)]
pub struct Volt(pub f64);
