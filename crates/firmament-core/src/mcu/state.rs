use std::sync::{Arc, Mutex};

use crate::{mcu::budget::Budget, traits::Mcu};

/// Thread-safe shared reference to simulation state.
pub type StateRef<M> = Arc<Mutex<State<M>>>;

/// Mutable simulation state shared between the executor and runtime.
#[derive(Debug)]
pub struct State<M: Mcu + Send + 'static> {
    /// The MCU instance.
    pub(crate) mcu: M,
    /// Compute (fuel) budget for the current tick.
    pub(crate) compute: Budget,
    /// Cycle budget for the current tick.
    pub(crate) cycles: Budget,
    /// WASM fuel level at the last metering checkpoint.
    pub(crate) last_fuel: u64,
}

impl<M: Mcu + Send + 'static> State<M> {
    pub const fn new(mcu: M) -> Self {
        Self {
            mcu,
            compute: Budget::new(0),
            cycles: Budget::new(0),
            last_fuel: u64::MAX,
        }
    }
}
