use std::{
    time::{
        Instant,
    },
};

use super::{
    Volt,
    Hertz,
};

#[derive(Clone, Copy, PartialEq, PartialOrd, Debug)]
pub struct Values {
    pub taken_at: Instant,
    pub frequency: Hertz,
}

pub enum Session {
    Initializing(Initializing),
    Estimated(Estimated),
}

impl Session {
    pub fn new() -> Self {
        Session::Initializing(Initializing {
            tick_state: TickState::Bootstrap,
        })
    }
}

// Initializing

pub struct Initializing {
    tick_state: TickState,
}

impl From<Initializing> for Session {
    fn from(state: Initializing) -> Session {
        Session::Initializing(state)
    }
}

impl Initializing {
    pub fn voltage_read(self, when: Instant, value: Volt) -> InitializingOp {
        match self.tick_state.next(Reading { when, value, }) {
            TickStateOp::Reset(tick_state) | TickStateOp::Idle(tick_state) =>
                InitializingOp::Idle(Initializing { tick_state, }),
            TickStateOp::Tick { tick_state, frequency, } =>
                InitializingOp::CarrierDetected(Estimated {
                    values: Values { taken_at: when, frequency, },
                    tick_state,
                }),
        }
    }
}

pub enum InitializingOp {
    Idle(Initializing),
    CarrierDetected(Estimated),
}

// Estimated

pub struct Estimated {
    values: Values,
    tick_state: TickState,
}

impl From<Estimated> for Session {
    fn from(state: Estimated) -> Session {
        Session::Estimated(state)
    }
}

impl Estimated {
    pub fn values(&self) -> &Values {
        &self.values
    }

    pub fn voltage_read(self, when: Instant, value: Volt) -> EstimatedOp {
        match self.tick_state.next(Reading { when, value, }) {
            TickStateOp::Reset(tick_state) =>
                EstimatedOp::CarrierLost(Initializing { tick_state, }),
            TickStateOp::Idle(tick_state) =>
                EstimatedOp::Idle(Estimated { values: self.values, tick_state, }),
            TickStateOp::Tick { tick_state, frequency, } =>
                EstimatedOp::Idle(Estimated {
                    values: Values { taken_at: when, frequency, },
                    tick_state,
                }),
        }
    }
}

pub enum EstimatedOp {
    Idle(Estimated),
    CarrierLost(Initializing),
}

// inner impl

enum TickState {
    Bootstrap,
    RangeDetect { range: Range, },
    RangeExpandToMin { range: Range, },
    RangeExpandToMax { range: Range, },
}

enum TickStateOp {
    Reset(TickState),
    Idle(TickState),
    Tick { tick_state: TickState, frequency: Hertz, },
}

impl TickState {
    fn next(self, reading: Reading) -> TickStateOp {
        match self {
            TickState::Bootstrap =>
                TickStateOp::Reset(TickState::RangeDetect {
                    range: Range {
                        min: reading,
                        max: reading,
                    },
                }),
            TickState::RangeDetect { mut range, } =>
                if reading.value < range.min.value {
                    range.min = reading;
                    TickStateOp::Reset(TickState::RangeExpandToMin { range, })
                } else if reading.value > range.max.value {
                    range.max = reading;
                    TickStateOp::Reset(TickState::RangeExpandToMax { range, })
                } else {
                    TickStateOp::Reset(TickState::RangeDetect { range, })
                },
            TickState::RangeExpandToMin { mut range, } =>
                if reading.value < range.min.value {
                    range.min = reading;
                    TickStateOp::Idle(TickState::RangeExpandToMin { range, })
                } else if reading.value > range.max.value {
                    range.max = reading;
                    TickStateOp::Reset(TickState::RangeDetect { range, })
                } else {
                    let tick_duration = reading.when.duration_since(range.min.when);
                    let range_duration = range.min.when.duration_since(range.max.when);
                    if tick_duration >= range_duration {
                        let frequency = Hertz(1.0 / range_duration.as_secs_f64());
                        TickStateOp::Tick {
                            tick_state: TickState::RangeExpandToMax {
                                range: Range {
                                    min: reading,
                                    ..range
                                },
                            },
                            frequency,
                        }
                    } else {
                        TickStateOp::Idle(TickState::RangeExpandToMin { range, })
                    }
                },
            TickState::RangeExpandToMax { mut range, } =>
                if reading.value < range.min.value {
                    range.min = reading;
                    TickStateOp::Reset(TickState::RangeDetect { range, })
                } else if reading.value > range.max.value {
                    range.max = reading;
                    TickStateOp::Idle(TickState::RangeExpandToMax { range, })
                } else {
                    let tick_duration = reading.when.duration_since(range.max.when);
                    let range_duration = range.max.when.duration_since(range.min.when);
                    if tick_duration >= range_duration {
                        let frequency = Hertz(1.0 / range_duration.as_secs_f64());
                        TickStateOp::Tick {
                            tick_state: TickState::RangeExpandToMax {
                                range: Range {
                                    max: reading,
                                    ..range
                                },
                            },
                            frequency,
                        }
                    } else {
                        TickStateOp::Idle(TickState::RangeExpandToMax { range, })
                    }
                },
        }
    }
}

struct Range {
    min: Reading,
    max: Reading,
}

#[derive(Clone, Copy, Debug)]
struct Reading {
    value: Volt,
    when: Instant,
}
