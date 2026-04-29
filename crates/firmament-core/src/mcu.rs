//! WASM-based MCU simulation runtime.
//!
//! Firmware compiled to WebAssembly runs inside a Wasmtime executor with
//! host-provided MMIO, WFI, interrupt dispatch, and debug logging. The
//! public API is exposed through [`Handle`].

mod budget;
mod channels;
mod config;
mod executor;
mod firmware;
mod handle;
mod imports;
mod mmio;
mod runtime;
mod state;

pub use config::Config;
pub use handle::Handle;
