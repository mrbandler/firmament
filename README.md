<div align="center">

# Firmament

**Flight software, from the ground up.**

[![Built with Rust](https://img.shields.io/badge/Built%20with-Rust-orange.svg)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](./LICENSE)
[![CI](https://github.com/mrbandler/firmament/actions/workflows/ci.yml/badge.svg)](https://github.com/mrbandler/firmament/actions/workflows/ci.yml)

*An embedded systems flight software education platform using KSP/KSA as a physics backend*

[Documentation](https://mrbandler.github.io/firmament/) · [Contributing](./CONTRIBUTING.md)

</div>

---

You're deep in a KSP mission. Your rocket needs to execute a suicide burn. Most mods give you something like `set_throttle(0.8)` and call it a day.

Firmament doesn't do that.

You write a 32-bit value to a CAN transmit register, which sends a frame to an engine controller at bus address `0x10`, which the host maps to a KSP engine part. Your firmware reads altitude from an ADC register, runs the control loop, and fires pyrotechnics via GPIO. The firmware doesn't know KSP exists. It talks to registers, just like real flight software.

Firmament is an embedded systems flight software education platform that uses KSP/KSA as a physics backend. You write bare-metal firmware in any language that compiles to WebAssembly, load it onto a virtual microcontroller, and fly.

And if none of the above convince you this project has merit, here is my last argument: because it's cool, that's why.

> **Firmament is in early development.** The core runtime works, firmware runs on a virtual MCU, but the full vision (game integration, peripherals, multi-MCU, ground station) is still ahead. Star and watch to follow progress.

---

Under the hood, Firmament simulates a virtual microcontroller. Your firmware compiles to WebAssembly and runs inside Wasmtime on the host. The MCU exposes the same interface you'd find on a real embedded chip: memory-mapped registers, a nested vectored interrupt controller, cycle budgets, and sleep via `wfi`.

The firmware runs as a real bare-metal program. It exports `_start`, the host calls it once, and it runs forever. There is no `tick()` callback and no scripting API. Interrupts fire at register access boundaries and sleep suspends the WASM fiber until the next interrupt wakes it.

KSP/KSA or a test harness sit on the other side of the register interface. The host maps ADC channels to vessel sensors, GPIO pins to pyrotechnics, CAN bus addresses to engines and reaction wheels. The firmware never touches the game directly, it reads and writes registers and the host translates.

```
┌─────────────────────┐     ┌──────────────────────┐
│   Your Firmware     │     │   Game / Physics     │
│   (WASM binary)     │     │                      │
│                     │     │  Orbital mechanics   │
│  Reads registers    │◄───►│  Atmospheric flight  │
│  Writes registers   │     │  Thermal model       │
│  Handles interrupts │     │  CommNet             │
│                     │     │                      │
└────────┬────────────┘     └───────────┬──────────┘
         │                              │
         │  WASM imports                │  Host maps
         │  (read/write volatile)       │  registers ↔ vessel
         │                              │
      ┌──▼──────────────────────────────▼──┐
      │           Virtual MCU              │
      │                                    │
      │  MMIO  ·  NVIC  ·  Timers  · Buses │
      └────────────────────────────────────┘
```

## Features

**Hardware simulation**
The virtual MCU provides memory-mapped I/O, nested vectored interrupts, cycle budgets, GPIO, ADC, timers, CAN, SPI, I2C, PWM, and watchdog peripherals.

**Language agnostic**
Anything that compiles to WebAssembly works, whether that's Rust, C, Zig, or AssemblyScript.

**Real firmware patterns**
Your code runs `_start`, sleeps with `wfi`, and wakes on interrupts. No scripting API, no callbacks, just bare-metal.

**Embedded Rust ecosystem**
The firmware crate stack mirrors the real world with PAC, HAL, BSP, and runtime layers, each independently replaceable.

**No game required**
A headless test harness lets you develop and validate firmware without KSP or KSA.

## Quick Start

```bash
git clone https://github.com/mrbandler/firmament.git
cd firmament
```

Build the example firmware:

```bash
cd examples/blink
cargo build --target wasm32-unknown-unknown --release
cd ../..
```

Run it on a virtual MCU:

```bash
cargo run --example blink_runner -p firmament-core
```

The firmware boots, toggles a GPIO register in a loop, and the host prints the result. No game, no hardware. Just registers.

## Project Structure

| Crate | Description |
|-------|-------------|
| `firmament-core` | Host-side runtime: WASM executor, virtual MCU, MMIO, interrupts, cycle budgets |
| `firmament-fm` | Guest-side firmware library: `no_std` WASM imports for register access, sleep, debug logging |

## Contributing

Contributions are welcome! Please read the [Contributing Guide](./CONTRIBUTING.md) to get started.

## Inspiration

Firmament draws inspiration from [kOS](https://ksp-kos.github.io/KOS/) for showing that KSP can be a platform for learning through code, [The Rusty Bits](https://www.youtube.com/@therustybits) for making embedded Rust accessible, and a general enthusiasm for rockets and close-to-hardware code.

## AI Transparency

This project uses AI as a development tool. All AI-generated content is reviewed and refined by a human.

**AI assists with:**

- Exploring ideas and concept refinement
- Generating boilerplate code
- Drafting documentation

**AI does not write:**

- Core logic and algorithms
- Architectural decisions
- Critical code paths

I believe in transparency about AI usage while maintaining quality standards.

## License

[MIT](./LICENSE)

---

<div align="center">

**[Star this repo](https://github.com/mrbandler/firmament)** if you think this is as cool as we do!

</div>
