use std::{
    time::{
        Instant,
    },
};

use super::Volt;

pub enum Session {
    Initializing(Initializing),
}

impl Session {
    pub fn new() -> Self {
        Session::Initializing(Initializing {
            state: InitializingState::Bootstrap,
        })
    }
}

// Initializing

pub struct Initializing {
    state: InitializingState,
}

impl From<Initializing> for Session {
    fn from(state: Initializing) -> Session {
        Session::Initializing(state)
    }
}

enum InitializingState {
    Bootstrap,
    RangeDetect { range: Range, },
    RangeExpandToMin { range: Range, },
    RangeExpandToMax { range: Range, },
}

impl Initializing {
    pub fn voltage_read(self, when: Instant, value: Volt) -> InitializingOp {
        let reading = Reading { when, value, };
        match self.state {
            InitializingState::Bootstrap =>
                InitializingOp::Idle(Initializing {
                    state: InitializingState::RangeDetect {
                        range: Range {
                            min: reading,
                            max: reading,
                        },
                    },
                }),
            InitializingState::RangeDetect { mut range, } =>
                if reading.value < range.min.value {
                    range.min = reading;
                    InitializingOp::Idle(Initializing {
                        state: InitializingState::RangeExpandToMin { range, },
                    })
                } else if reading.value > range.max.value {
                    range.max = reading;
                    InitializingOp::Idle(Initializing {
                        state: InitializingState::RangeExpandToMax { range, },
                    })
                } else {
                    InitializingOp::Idle(Initializing {
                        state: InitializingState::RangeDetect { range, },
                    })
                },
            InitializingState::RangeExpandToMin { mut range, } =>
                if reading.value < range.min.value {
                    range.min = reading;
                    InitializingOp::Idle(Initializing {
                        state: InitializingState::RangeExpandToMin { range, },
                    })
                } else if reading.value > range.max.value {
                    range.max = reading;
                    InitializingOp::Idle(Initializing {
                        state: InitializingState::RangeDetect { range, },
                    })
                } else {
                    let duration = when.duration_since(range.min.when);

                    InitializingOp::Idle(Initializing {
                        state: InitializingState::RangeExpandToMin { range, },
                    })
                },
            InitializingState::RangeExpandToMax { mut range, } =>
                if reading.value < range.min.value {
                    range.min = reading;
                    InitializingOp::Idle(Initializing {
                        state: InitializingState::RangeDetect { range, },
                    })
                } else if reading.value > range.max.value {
                    range.max = reading;
                    InitializingOp::Idle(Initializing {
                        state: InitializingState::RangeExpandToMax { range, },
                    })
                } else {
                    let duration = when.duration_since(range.max.when);

                    InitializingOp::Idle(Initializing {
                        state: InitializingState::RangeExpandToMax { range, },
                    })
                },

        }
    }
}

pub enum InitializingOp {
    Idle(Initializing),
}

// inner impl

struct Range {
    min: Reading,
    max: Reading,
}

#[derive(Clone, Copy, Debug)]
struct Reading {
    value: Volt,
    when: Instant,
}
