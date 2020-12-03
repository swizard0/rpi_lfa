use std::{
    time::{
        Instant,
        Duration,
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

#[derive(Debug)]
enum TickState {
    Bootstrap,
    RangeDetect { range: Range, },
    RangeExpandToMin { range: Range, },
    PeriodMeasureUp { range: Range, },
    RangeExpandToMax { range: Range, },
    PeriodMeasureDown { range: Range, },
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
                    TickStateOp::Idle(TickState::RangeExpandToMin { range, })
                } else if reading.value > range.max.value {
                    range.max = reading;
                    TickStateOp::Idle(TickState::RangeExpandToMax { range, })
                } else {
                    TickStateOp::Idle(TickState::RangeDetect { range, })
                },
            TickState::RangeExpandToMin { mut range, } =>
                if reading.value < range.min.value {
                    range.min = reading;
                    TickStateOp::Idle(TickState::RangeExpandToMin { range, })
                } else if reading.value > range.max.value {
                    range.max = reading;
                    TickStateOp::Reset(TickState::RangeExpandToMax { range, })
                } else {
                    TickStateOp::Idle(TickState::PeriodMeasureUp { range, })
                },
            TickState::PeriodMeasureUp { mut range, } =>
                if reading.value < range.min.value {
                    range.min = reading;
                    TickStateOp::Idle(TickState::RangeExpandToMin { range, })
                } else if reading.value > range.max.value {
                    range.max = reading;
                    TickStateOp::Idle(TickState::PeriodMeasureUp { range, })
                } else {
                    let tick_duration = reading.when.duration_since(range.min.when);
                    let range_duration = range.duration();
                    if tick_duration >= range_duration {
                        let frequency = Hertz(0.5 / range_duration.as_secs_f64()); // half of a period
                        TickStateOp::Tick {
                            tick_state: TickState::RangeDetect {
                                range: Range {
                                    min: reading,
                                    max: reading,
                                },
                            },
                            frequency,
                        }
                    } else {
                        TickStateOp::Idle(TickState::PeriodMeasureUp { range, })
                    }
                },
            TickState::RangeExpandToMax { mut range, } =>
                if reading.value < range.min.value {
                    range.min = reading;
                    TickStateOp::Reset(TickState::RangeExpandToMin { range, })
                } else if reading.value > range.max.value {
                    range.max = reading;
                    TickStateOp::Idle(TickState::RangeExpandToMax { range, })
                } else {
                    TickStateOp::Idle(TickState::PeriodMeasureDown { range, })
                },
            TickState::PeriodMeasureDown { mut range, } =>
                if reading.value < range.min.value {
                    range.min = reading;
                    TickStateOp::Idle(TickState::PeriodMeasureDown { range, })
                } else if reading.value > range.max.value {
                    range.min = reading;
                    TickStateOp::Idle(TickState::RangeExpandToMax { range, })
                } else {
                    let tick_duration = reading.when.duration_since(range.max.when);
                    let range_duration = range.duration();
                    if tick_duration >= range_duration {
                        let frequency = Hertz(0.5 / range_duration.as_secs_f64()); // half of a period
                        TickStateOp::Tick {
                            tick_state: TickState::RangeDetect {
                                range: Range {
                                    min: reading,
                                    max: reading,
                                },
                            },
                            frequency,
                        }
                    } else {
                        TickStateOp::Idle(TickState::PeriodMeasureDown { range, })
                    }
                },
        }
    }
}

#[derive(Debug)]
struct Range {
    min: Reading,
    max: Reading,
}

impl Range {
    fn duration(&self) -> Duration {
        if self.min.when < self.max.when {
            self.max.when.duration_since(self.min.when)
        } else {
            self.min.when.duration_since(self.max.when)
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Reading {
    value: Volt,
    when: Instant,
}

#[cfg(test)]
mod tests {
    use std::time::{
        Instant,
        Duration,
    };

    use rand::Rng;

    use crate::{
        Volt,
        Hertz,
        ac_driver::{
            Reading,
            Session,
            InitializingOp,
            EstimatedOp,
        },
    };

    fn noised_signal_gen(
        amplitude: Volt,
        freq: Hertz,
        noise_fraq: f64,
        duration: Duration,
        samples: usize,
    )
        -> Vec<Reading>
    {
        let mut rng = rand::thread_rng();
        let duration_f64 = duration.as_secs_f64();
        let now = Instant::now();
        let noise_amplitude = amplitude.0 * noise_fraq;
        let mut readings: Vec<_> = (0 .. samples)
            .map(|_| {
                let time = rng.gen_range(0.0, duration_f64);
                let wave_arg = 2.0 * std::f64::consts::PI * freq.0 * time;
                let wave_fun = amplitude.0 * wave_arg.sin();
                let noise = rng.gen_range(-noise_amplitude, noise_amplitude);
                Reading {
                    value: Volt(wave_fun + noise),
                    when: now + Duration::from_secs_f64(time),
                }
            })
            .collect();
        readings.sort_by(|a, b| a.when.cmp(&b.when));
        readings
    }

    #[test]
    fn accurate_50hz() {
        let signal = noised_signal_gen(Volt(3.3), Hertz(50.0), 0.1, Duration::from_micros(100000), 100);
        let mut session = Session::new();
        let mut hz_sum = 0.0;
        let mut hz_total = 0;
        for reading in signal {
            session = match session {
                Session::Initializing(state) =>
                    match state.voltage_read(reading.when, reading.value) {
                        InitializingOp::Idle(session) =>
                            session.into(),
                        InitializingOp::CarrierDetected(session) =>
                            session.into(),
                    },
                Session::Estimated(state) => {
                    hz_total += 1;
                    hz_sum += state.values.frequency.0;
                    match state.voltage_read(reading.when, reading.value) {
                        EstimatedOp::Idle(session) =>
                            session.into(),
                        EstimatedOp::CarrierLost(session) =>
                            session.into(),
                    }
                },
            }
        }
        let avg_hz = hz_sum / hz_total as f64;
        assert!(avg_hz > 40.0, "average frequency is {} but expected to be 40 < x < 60", avg_hz);
        assert!(avg_hz < 60.0, "average frequency is {} but expected to be 40 < x < 60", avg_hz);
    }

    #[test]
    fn noisy_100hz() {
        let session = Session::new();
    }
}
