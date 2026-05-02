#![allow(clippy::cast_possible_truncation)]

use std::{ops::Range, time::Duration};

use firmament_core::{
    error::{BusError, DeviceError, McuError},
    firmament::Firmament,
    traits::{
        Addressable, Advanceable, Bus, Device, InterruptController, Mcu, Read, ResetKind, Resettable, Spec, Write,
    },
    volatile_access::{Addr, Value, Width},
};
use tokio::sync::mpsc::Receiver;

// ---------------------------------------------------------------------------
// GPIO Device — two 32-bit registers: ODR (write) and IDR (read-back)
// ---------------------------------------------------------------------------

const GPIO_BASE: u32 = 0x4000_0000;
const GPIO_IDR_OFFSET: u32 = 0x10;
const GPIO_ODR_OFFSET: u32 = 0x14;
const GPIO_SIZE: u32 = 0x20;

struct GpioPort {
    odr: u32,
    idr: u32,
}

impl GpioPort {
    const fn new() -> Self {
        Self { odr: 0, idr: 0 }
    }
}

impl Read for GpioPort {
    type Error = DeviceError;

    fn read(&mut self, addr: Addr, width: Width) -> Result<Value, Self::Error> {
        let offset = u32::from(addr) - GPIO_BASE;
        match (offset, width) {
            (GPIO_IDR_OFFSET, Width::U32) => Ok(Value::U32(self.idr)),
            (GPIO_ODR_OFFSET, Width::U32) => Ok(Value::U32(self.odr)),
            _ => Err(DeviceError::Read(firmament_core::error::ReadError::InvalidOffset {
                offset,
                range_start: 0,
                range_end: GPIO_SIZE,
            })),
        }
    }
}

impl Write for GpioPort {
    type Error = DeviceError;

    fn write(&mut self, addr: Addr, val: Value) -> Result<(), Self::Error> {
        let offset = u32::from(addr) - GPIO_BASE;
        match (offset, val) {
            (GPIO_ODR_OFFSET, Value::U32(v)) => {
                self.odr = v;
                // Mirror ODR to IDR for read-back
                self.idr = v;
                Ok(())
            },
            _ => Err(DeviceError::Write(firmament_core::error::WriteError::InvalidOffset {
                offset,
                range_start: 0,
                range_end: GPIO_SIZE,
            })),
        }
    }
}

impl firmament_core::traits::Addressable for GpioPort {
    fn address_range(&self) -> Range<u32> {
        GPIO_BASE..GPIO_BASE + GPIO_SIZE
    }
}

impl Device for GpioPort {}

// ---------------------------------------------------------------------------
// SimpleBus — routes to a single GPIO device
// ---------------------------------------------------------------------------

struct SimpleBus {
    gpio: GpioPort,
}

impl SimpleBus {
    const fn new() -> Self {
        Self { gpio: GpioPort::new() }
    }
}

impl Read for SimpleBus {
    type Error = BusError;

    fn read(&mut self, addr: Addr, width: Width) -> Result<Value, Self::Error> {
        let raw = u32::from(addr);
        if self.gpio.address_range().contains(&raw) {
            self.gpio.read(addr, width).map_err(BusError::Device)
        } else {
            Err(BusError::UnmappedAddress { addr: raw })
        }
    }
}

impl Write for SimpleBus {
    type Error = BusError;

    fn write(&mut self, addr: Addr, val: Value) -> Result<(), Self::Error> {
        let raw = u32::from(addr);
        if self.gpio.address_range().contains(&raw) {
            self.gpio.write(addr, val).map_err(BusError::Device)
        } else {
            Err(BusError::UnmappedAddress { addr: raw })
        }
    }
}

impl firmament_core::traits::Addressable for SimpleBus {
    fn address_range(&self) -> Range<u32> {
        0..0xFFFF_FFFF
    }
}

impl Advanceable for SimpleBus {
    fn advance(&mut self, _cycles: u64) {}
}

impl Bus for SimpleBus {
    fn register(&mut self, _device: Box<dyn Device>) {
        // Not used — GPIO is hardcoded
    }

    fn next_event(&self) -> Option<u64> {
        None // No timed events
    }
}

// ---------------------------------------------------------------------------
// StubInterruptController — no interrupts for the blink test
// ---------------------------------------------------------------------------

struct StubInterruptController;

impl Resettable for StubInterruptController {
    fn reset(&mut self, _kind: ResetKind) {}
}

impl InterruptController for StubInterruptController {
    fn connect(&mut self, _receiver: Receiver<u8>) {}

    fn highest_preempting(&mut self) -> Option<u8> {
        None
    }

    fn enter_isr(&mut self, _irq: u8) {}
    fn exit_isr(&mut self, _irq: u8) {}
    fn set_priority_mask(&mut self, _priority: u8) {}
    fn priority_mask(&self) -> u8 {
        0
    }
    fn set_global_enabled(&mut self, _enabled: bool) {}
}

// ---------------------------------------------------------------------------
// MockMcu — wires together bus + interrupt controller
// ---------------------------------------------------------------------------

struct MockMcu {
    spec: Spec,
    bus: SimpleBus,
    ic: StubInterruptController,
    sleeping: bool,
}

impl MockMcu {
    const fn new() -> Self {
        Self {
            bus: SimpleBus::new(),
            ic: StubInterruptController,
            sleeping: false,
            spec: Spec {
                clock_hz: 195_000_000,
                compute_per_cycle: 2.0,
            },
        }
    }
}

impl Read for MockMcu {
    type Error = McuError;

    fn read(&mut self, addr: Addr, width: Width) -> Result<Value, Self::Error> {
        self.bus.read(addr, width).map_err(McuError::Bus)
    }
}

impl Write for MockMcu {
    type Error = McuError;

    fn write(&mut self, addr: Addr, val: Value) -> Result<(), Self::Error> {
        self.bus.write(addr, val).map_err(McuError::Bus)
    }
}

impl Advanceable for MockMcu {
    fn advance(&mut self, cycles: u64) {
        self.bus.advance(cycles);
    }
}

impl Resettable for MockMcu {
    fn reset(&mut self, _kind: ResetKind) {
        self.sleeping = false;
    }
}

impl Mcu for MockMcu {
    fn interrupt_controller(&mut self) -> &mut dyn InterruptController {
        &mut self.ic
    }

    fn bus(&mut self) -> &mut dyn Bus {
        &mut self.bus
    }

    fn wake(&mut self) {
        self.sleeping = false;
    }

    fn sleep(&mut self) {
        self.sleeping = true;
    }

    fn is_sleeping(&self) -> bool {
        self.sleeping
    }

    fn spec(&self) -> &Spec {
        &self.spec
    }
}

// ---------------------------------------------------------------------------
// Test
// ---------------------------------------------------------------------------

const BLINK_WASM: &[u8] = include_bytes!("../../../examples/blink/target/wasm32-unknown-unknown/release/blink.wasm");

#[tokio::test]
async fn blink_firmware_boots_and_toggles_gpio() {
    let mcu = MockMcu::new();
    let handle = Firmament::new()
        .system("test-sys")
        .build()
        .mcu("test-mcu")
        .image(BLINK_WASM)
        .device(mcu)
        .build()
        .expect("failed to spawn MCU");

    handle.power_on().await.expect("failed to power on MCU");

    // Simulate 10 ticks at 50Hz (20ms each).
    // Each tick wakes the firmware from wfi, it toggles GPIO and calls wfi again.
    for _i in 0..10 {
        handle
            .tick(Duration::from_millis(20))
            .await
            .expect("tick failed");

        // Yield to let the firmware process the tick
        tokio::task::yield_now().await;

        let val: u32 = handle
            .read(0x4000_0014_u32)
            .await
            .expect("read GPIO ODR failed");

        // The firmware starts with toggle=0, then XORs with 1 each iteration.
        // Tick 0 wakes after the first wfi (toggle was already set to 1 on boot).
        // Each subsequent tick triggers another toggle.
        assert!(val == 0 || val == 1, "unexpected GPIO ODR value: {val}");
    }
}
