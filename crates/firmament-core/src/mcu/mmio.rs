#![expect(dead_code, reason = "The full API is not in use yet.")]

use crate::{
    error::McuError,
    traits::Mcu,
    volatile_access::{Addr, Value, Width},
};

/// A single MMIO operation (read or write) that can be costed and executed against an MCU.
pub trait Mmio {
    /// The value produced by this operation.
    type Output;

    /// Target address.
    fn addr(&self) -> Addr;

    /// Access width.
    fn width(&self) -> Width;

    /// Cycle cost of this operation on the given MCU. Defaults to 1.
    fn cost<M: Mcu>(&self, _mcu: &M) -> u64 {
        1
    }

    /// Executes the operation against the MCU.
    fn run<M: Mcu>(self, mcu: &mut M) -> Result<Self::Output, McuError>;
}

/// An MMIO read operation at a given address and width.
pub struct Read(Addr, Width);

impl Read {
    /// Creates a new read operation.
    pub const fn new(addr: Addr, width: Width) -> Self {
        Self(addr, width)
    }
}

impl Mmio for Read {
    type Output = Value;

    fn addr(&self) -> Addr {
        self.0
    }

    fn width(&self) -> Width {
        self.1
    }

    fn cost<M: Mcu>(&self, mcu: &M) -> u64 {
        mcu.rcost(self.0, self.1)
    }

    fn run<M: Mcu>(self, mcu: &mut M) -> Result<Self::Output, McuError> {
        mcu.read(self.0, self.1)
    }
}

/// An MMIO write operation at a given address with a typed value.
pub struct Write(Addr, Value);

impl Write {
    /// Creates a new write operation.
    pub const fn new(addr: Addr, value: Value) -> Self {
        Self(addr, value)
    }
}

impl Mmio for Write {
    type Output = ();

    fn addr(&self) -> Addr {
        self.0
    }

    fn width(&self) -> Width {
        self.1.width()
    }

    fn cost<M: Mcu>(&self, mcu: &M) -> u64 {
        mcu.wcost(self.0, self.1.width())
    }

    fn run<M: Mcu>(self, mcu: &mut M) -> Result<Self::Output, McuError> {
        mcu.write(self.0, self.1)
    }
}
