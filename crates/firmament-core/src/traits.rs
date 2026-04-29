use std::ops::Range;
use tokio::sync::mpsc::{Receiver, Sender};

use crate::{
    error::{BusError, DeviceError, McuError},
    volatile_access::{Addr, Value, Width},
};

/// The kind of MCU reset to perform.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResetKind {
    /// Full power-cycle reset; all state is cleared.
    Cold,
    /// Soft reset; preserves peripheral register state.
    Warm,
}

/// Hardware specification for an MCU: clock speed and compute ratio.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spec {
    /// Clock frequency in Hz.
    pub clock_hz: u64,
    /// WASM fuel consumed per simulated clock cycle.
    pub compute_per_cycle: f64,
}

/// Readable memory-mapped region.
pub trait Read {
    type Error;

    /// Reads a value of the given width from the address.
    ///
    /// # Errors
    ///
    /// Returns `Self::Error` if the address is invalid, unsupported width,
    /// or otherwise inaccessible.
    fn read(&mut self, addr: Addr, width: Width) -> Result<Value, Self::Error>;

    /// Returns the cycle cost of a read at this address and width. Defaults to 1.
    fn rcost(&self, _addr: Addr, _width: Width) -> u64 {
        1
    }
}
/// Writable memory-mapped region.
pub trait Write {
    type Error;

    /// Writes a value to the address.
    ///
    /// # Errors
    ///
    /// Returns `Self::Error` if the address is invalid, read-only,
    /// or the value is rejected.
    fn write(&mut self, addr: Addr, val: Value) -> Result<(), Self::Error>;

    /// Returns the cycle cost of a write at this address and width. Defaults to 1.
    fn wcost(&self, _addr: Addr, _width: Width) -> u64 {
        1
    }
}

/// A read/write region that occupies a specific address range.
pub trait Addressable: Read + Write {
    /// Returns the address range this region occupies.
    fn address_range(&self) -> Range<u32>;
}

/// Can be advanced forward by a number of clock cycles.
pub trait Advanceable {
    /// Advances internal state by the given number of cycles.
    fn advance(&mut self, cycles: u64);
}

/// Supports cold and warm resets.
pub trait Resettable {
    /// Resets internal state according to the given [`ResetKind`].
    fn reset(&mut self, kind: ResetKind);
}

/// A device that can raise interrupt requests.
pub trait InterruptEmitter {
    /// Connects the emitter to an IRQ channel.
    fn connect(&mut self, sender: Sender<u8>);

    /// Returns the cycle count until the next scheduled interrupt, if any.
    fn next(&self) -> Option<u64>;
}

/// A peripheral device on the bus with optional advancement, reset, and interrupt capabilities.
pub trait Device: Read<Error = DeviceError> + Write<Error = DeviceError> + Addressable {
    /// Returns this device as advanceable, if it tracks cycle-based state.
    fn as_advanceable(&mut self) -> Option<&mut dyn Advanceable> {
        None
    }

    /// Returns this device as resettable, if it supports reset.
    fn as_resettable(&mut self) -> Option<&mut dyn Resettable> {
        None
    }

    /// Returns this device as an interrupt emitter, if it can raise IRQs.
    fn as_interrupt_emitter(&mut self) -> Option<&mut dyn InterruptEmitter> {
        None
    }
}

/// Address bus that dispatches reads/writes to registered devices.
pub trait Bus: Read<Error = BusError> + Write<Error = BusError> + Addressable + Advanceable {
    /// Registers a device on the bus.
    fn register(&mut self, device: Box<dyn Device>);

    /// Returns the cycle count until the next scheduled bus event, if any.
    fn next_event(&self) -> Option<u64>;
}

/// Interrupt controller that manages IRQ prioritization and dispatch.
pub trait InterruptController: Resettable {
    /// Connects the controller to a device IRQ receiver channel.
    fn connect(&mut self, receiver: Receiver<u8>);

    /// Returns the highest-priority pending IRQ that can preempt, if any.
    fn highest_preempting(&mut self) -> Option<u8>;

    /// Marks the given IRQ as actively being serviced.
    fn enter_isr(&mut self, irq: u8);

    /// Marks the given IRQ as finished.
    fn exit_isr(&mut self, irq: u8);

    /// Sets the priority mask; IRQs at or below this priority are suppressed.
    fn set_priority_mask(&mut self, priority: u8);

    /// Returns the current priority mask.
    fn priority_mask(&self) -> u8;

    /// Enables or disables all interrupts globally.
    fn set_global_enabled(&mut self, enabled: bool);
}

/// A complete microcontroller: bus, interrupt controller, and sleep/wake support.
pub trait Mcu: Read<Error = McuError> + Write<Error = McuError> + Advanceable + Resettable {
    /// Returns this MCU's hardware specification.
    fn spec(&self) -> &Spec;

    /// Returns a mutable reference to the interrupt controller.
    fn interrupt_controller(&mut self) -> &mut dyn InterruptController;

    /// Returns a mutable reference to the address bus.
    fn bus(&mut self) -> &mut dyn Bus;

    /// Wakes the MCU from sleep mode.
    fn wake(&mut self);

    /// Puts the MCU into sleep mode (WFI).
    fn sleep(&mut self);

    /// Returns `true` if the MCU is currently sleeping.
    fn is_sleeping(&self) -> bool;
}
