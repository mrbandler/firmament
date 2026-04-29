use bon::Builder;

/// Configuration for the MCU simulation runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Builder)]
pub struct Config {
    /// Capacity of the external write channel buffer.
    pub write_buffer: usize,
    /// Capacity of the command channel buffer.
    pub cmd_buffer: usize,
    /// WASM fuel consumed between async yield points.
    pub yield_interval: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            write_buffer: 128,
            cmd_buffer: 32,
            yield_interval: 10_000,
        }
    }
}
