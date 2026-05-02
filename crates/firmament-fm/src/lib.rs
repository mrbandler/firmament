//! Guest-side runtime library for firmament WASM firmware.
//!
//! Provides volatile memory access, wait-for-interrupt, and debug logging
//! via host-imported functions. Intended for `no_std` firmware targeting
//! a simulated MCU.

#![no_std]
#![expect(clippy::cast_possible_truncation)]

#[link(wasm_import_module = "fm")]
extern "C" {
    #[link_name = "read_volatile_u8"]
    fn _read_volatile_u8(addr: u32) -> u8;

    #[link_name = "read_volatile_u16"]
    fn _read_volatile_u16(addr: u32) -> u16;

    #[link_name = "read_volatile_u32"]
    fn _read_volatile_u32(addr: u32) -> u32;

    #[link_name = "write_volatile_u8"]
    fn _write_volatile_u8(addr: u32, val: u8);

    #[link_name = "write_volatile_u16"]
    fn _write_volatile_u16(addr: u32, val: u16);

    #[link_name = "write_volatile_u32"]
    fn _write_volatile_u32(addr: u32, val: u32);

    #[link_name = "wfi"]
    fn _wfi();

    #[link_name = "log"]
    fn _log(ptr: u32, len: u32);
}

/// Width-dispatched volatile memory access.
///
/// Implemented for `u8`, `u16`, and `u32` to route reads and writes
/// through the corresponding host imports.
pub trait VolatileAccess: Sized {
    /// Read a value from the given address.
    fn read(addr: u32) -> Self;
    /// Write a value to the given address.
    fn write(addr: u32, val: Self);
}

impl VolatileAccess for u8 {
    fn read(addr: u32) -> Self {
        unsafe { _read_volatile_u8(addr) }
    }
    fn write(addr: u32, val: Self) {
        unsafe { _write_volatile_u8(addr, val) }
    }
}

impl VolatileAccess for u16 {
    fn read(addr: u32) -> Self {
        unsafe { _read_volatile_u16(addr) }
    }
    fn write(addr: u32, val: Self) {
        unsafe { _write_volatile_u16(addr, val) }
    }
}

impl VolatileAccess for u32 {
    fn read(addr: u32) -> Self {
        unsafe { _read_volatile_u32(addr) }
    }
    fn write(addr: u32, val: Self) {
        unsafe { _write_volatile_u32(addr, val) }
    }
}

/// Reads a value from the MCU memory space. Width is determined by `T`.
///
/// # Safety
///
/// `addr` must be a valid, aligned address in the MCU's memory map
/// (peripheral registers, SRAM, etc.).
pub unsafe fn read_volatile<T: VolatileAccess>(addr: *const T) -> T {
    T::read(addr as u32)
}

/// Writes a value to the MCU memory space. Width is determined by `T`.
///
/// # Safety
///
/// `addr` must be a valid, aligned address in the MCU's memory map
/// (peripheral registers, SRAM, etc.).
pub unsafe fn write_volatile<T: VolatileAccess>(addr: *mut T, val: T) {
    T::write(addr as u32, val);
}

/// Yields execution until the next interrupt fires.
pub fn wfi() {
    unsafe { _wfi() }
}

/// Internal writer that forwards formatted output to the host via the
/// `log` import. Not part of the public API.
#[doc(hidden)]
pub struct LogWriter;

impl core::fmt::Write for LogWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        // The host import traps on error (OOM, poisoned lock), which is
        // unrecoverable in WASM — the instance aborts. Returning Ok(())
        // is correct because we never reach this line after a trap.
        unsafe { _log(s.as_ptr() as u32, s.len() as u32) };
        Ok(())
    }
}

/// Prints a formatted line to the host's log output.
#[macro_export]
macro_rules! println {
    ($($arg:tt)*) => {{
        use core::fmt::Write;
        let mut w = $crate::LogWriter;
        let _ = core::writeln!(w, $($arg)*);
    }};
}

/// Prints formatted output to the host's log output (no trailing newline).
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {{
        use core::fmt::Write;
        let mut w = $crate::LogWriter;
        let _ = core::write!(w, $($arg)*);
    }};
}
