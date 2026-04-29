#![expect(clippy::use_self, reason = "Self can't be used with a generic type parameter")]

use std::sync::Arc;
use wasmtime::{Caller, Extern};

use crate::{
    error::RuntimeError,
    mcu::{channels::RuntimeLink, mmio::Mmio, state::StateRef},
    traits::Mcu,
};

/// Wasmtime store data. Holds shared MCU state and the runtime channel link.
#[derive(Debug)]
pub struct Runtime<M: Mcu + Send + 'static> {
    pub(crate) state: StateRef<M>,
    pub(crate) link: RuntimeLink,
}

impl<M: Mcu + Send + 'static> Clone for Runtime<M> {
    fn clone(&self) -> Self {
        Self {
            state: Arc::clone(&self.state),
            link: self.link.clone(),
        }
    }
}

impl<M: Mcu + Send + 'static> Runtime<M> {
    /// Creates a new runtime with shared state and a channel link.
    pub const fn new(state: StateRef<M>, link: RuntimeLink) -> Self {
        Self { state, link }
    }

    /// Executes an MMIO operation: drains external writes, runs the op, checks for interrupts, and paces.
    pub async fn mmio<O: Mmio>(caller: &mut Caller<'_, Runtime<M>>, op: O) -> Result<O::Output, RuntimeError> {
        Runtime::<M>::meter(caller)?;

        let (val, preempt) = {
            let runtime = caller.data_mut();
            let mut state = runtime
                .state
                .lock()
                .map_err(|_| RuntimeError::LockPoisoned("state".into()))?;

            // Drain external writes. This is free, these model the physical world
            // (sensor updates, pin changes), not MCU work.
            let mut writes = runtime
                .link
                .write_rx
                .lock()
                .map_err(|_| RuntimeError::LockPoisoned("write_rx".into()))?;
            while let Ok((addr, val)) = writes.try_recv() {
                state.mcu.write(addr, val)?;
            }

            let cycles = op.cost(&state.mcu);
            let val = op.run(&mut state.mcu)?;

            state.mcu.advance(cycles);
            state.cycles.consume(cycles);

            let irq = state.mcu.interrupt_controller().highest_preempting();
            drop(state);

            (val, irq)
        };

        if let Some(irq) = preempt {
            Runtime::<M>::interrupt(caller, irq).await?;
        }

        Runtime::<M>::pace(caller).await?;

        Ok(val)
    }

    /// Implements Wait-For-Interrupt: sleeps the MCU, advances cycles, and wakes on IRQ or next tick.
    pub async fn wfi(caller: &mut Caller<'_, Runtime<M>>) -> Result<(), RuntimeError> {
        Runtime::<M>::meter(caller)?;

        // Put MCU to sleep
        {
            let mut state = caller
                .data_mut()
                .state
                .lock()
                .map_err(|_| RuntimeError::LockPoisoned("state".into()))?;
            state.mcu.sleep();
        }

        // Wait for interrupt
        loop {
            let preempt = {
                let mut state = caller
                    .data_mut()
                    .state
                    .lock()
                    .map_err(|_| RuntimeError::LockPoisoned("state".into()))?;

                let mut irq = None;
                while !state.cycles.exhausted() {
                    let next = state.mcu.bus().next_event();
                    let cycles = match next {
                        Some(event) if event <= state.cycles.remaining().cast_unsigned() => event,
                        _ => state.cycles.remaining().max(0).cast_unsigned(),
                    };

                    state.mcu.advance(cycles);
                    state.cycles.consume(cycles);

                    irq = state.mcu.interrupt_controller().highest_preempting();
                    if irq.is_some() {
                        break;
                    }
                }

                drop(state);

                irq
            };

            if let Some(irq) = preempt {
                caller
                    .data_mut()
                    .state
                    .lock()
                    .map_err(|_| RuntimeError::LockPoisoned("state".into()))?
                    .mcu
                    .wake();

                Runtime::<M>::interrupt(caller, irq).await?;

                return Ok(());
            }

            let notify = Arc::clone(&caller.data_mut().link.tick_notify);
            notify.notified().await;
        }
    }

    /// Dispatches an ISR: enters the interrupt, calls the exported `__isr_{irq}` handler, then exits.
    pub async fn interrupt(caller: &mut Caller<'_, Runtime<M>>, irq: u8) -> Result<(), RuntimeError> {
        // Entering ISR.
        {
            let runtime = caller.data_mut();
            let mut state = runtime
                .state
                .lock()
                .map_err(|_| RuntimeError::LockPoisoned("state".into()))?;
            state.mcu.interrupt_controller().enter_isr(irq);
        }

        // Call handler.
        let isr_fn = caller
            .get_export(&format!("__isr_{irq}"))
            .and_then(Extern::into_func)
            .ok_or(RuntimeError::MissingIsr { irq })?;
        let typed_isr_fn = isr_fn
            .typed::<(), ()>(&caller)
            .map_err(RuntimeError::Trap)?;

        typed_isr_fn
            .call_async(&mut *caller, ())
            .await
            .map_err(RuntimeError::Trap)?;

        // Exiting ISR.
        {
            let runtime = caller.data_mut();
            let mut state = runtime
                .state
                .lock()
                .map_err(|_| RuntimeError::LockPoisoned("state".into()))?;
            state.mcu.interrupt_controller().exit_isr(irq);
        }

        Ok(())
    }

    /// Samples WASM fuel consumption and converts it into cycle and compute budget charges.
    pub fn meter(caller: &mut Caller<'_, Runtime<M>>) -> Result<(), RuntimeError> {
        const FUEL_PER_CYCLE: f64 = 2.0; // TODO: Move into MCU configuration/intrinsics.

        let now_fuel = caller.get_fuel().map_err(RuntimeError::Trap)?;
        let runtime = caller.data_mut();
        let mut state = runtime
            .state
            .lock()
            .map_err(|_| RuntimeError::LockPoisoned("state".into()))?;

        let compute = state.last_fuel.saturating_sub(now_fuel);

        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            clippy::cast_precision_loss
        )]
        let cycles = (compute as f64 / FUEL_PER_CYCLE) as u64;

        state.last_fuel = now_fuel;
        state.compute.consume(compute);
        state.cycles.consume(cycles);

        drop(state);

        Ok(())
    }

    /// Blocks until the cycle budget is replenished by a tick, throttling firmware execution.
    pub async fn pace(caller: &Caller<'_, Runtime<M>>) -> Result<(), RuntimeError> {
        loop {
            let exhausted = {
                let state = caller
                    .data()
                    .state
                    .lock()
                    .map_err(|_| RuntimeError::LockPoisoned("state".into()))?;
                state.cycles.exhausted()
            };

            if !exhausted {
                return Ok(());
            }

            let runtime = caller.data();
            let notify = Arc::clone(&runtime.link.tick_notify);
            notify.notified().await;
        }
    }
}
