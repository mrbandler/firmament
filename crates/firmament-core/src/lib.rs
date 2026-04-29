//! Core crate for the Firmament MCU simulator.
//!
//! Provides a WASM-based microcontroller simulation runtime where firmware
//! compiled to WebAssembly runs against host-provided MMIO, interrupts,
//! and debug facilities.

pub mod error;
pub mod mcu;
pub mod traits;
pub mod volatile_access;
