use std::{sync::Arc, time::Duration};

use crate::{
    error::RuntimeError,
    mcu::{
        budget::Budget,
        channels::ExecutorLink,
        firmware::{Firmware, Image, Linked},
        handle::Command,
        runtime::Runtime,
    },
    traits::{Mcu, ResetKind},
    volatile_access::{Addr, Value, Width},
};

#[derive(Debug)]
enum Transition {
    PowerOn,
    CleanExit,
    Trap(RuntimeError),
    Shutdown,
    Reset(ResetKind),
    Destroy,
}

/// Current lifecycle state of the MCU simulation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// MCU is powered off.
    Off,
    /// Firmware is executing.
    Running,
    /// Firmware trapped; awaiting reset or shutdown.
    Halted,
}

/// Drives the MCU lifecycle state machine (Off -> Running -> Halted) on a spawned task.
#[derive(Debug)]
pub struct Executor<M: Mcu + Send + 'static> {
    image: Arc<Image>,
    runtime: Runtime<M>,
    link: ExecutorLink,
    reset: bool,
}

impl<M: Mcu + Send + 'static> Executor<M> {
    /// Creates a new executor with the given firmware image, runtime, and channel link.
    pub const fn new(image: Arc<Image>, runtime: Runtime<M>, link: ExecutorLink) -> Self {
        Self {
            image,
            runtime,
            link,
            reset: false,
        }
    }

    /// Runs the lifecycle loop until the MCU is destroyed or firmware exits cleanly.
    pub async fn exec(mut self) {
        loop {
            if !self.reset {
                match self.await_power_on().await {
                    Transition::PowerOn => {},
                    Transition::Destroy => break,
                    _ => continue,
                }
            }
            self.reset = false;

            let mut transition = self.run().await;
            loop {
                match transition {
                    Transition::Trap(err) => transition = self.trap(err).await,
                    Transition::Reset(kind) => match self.reset(kind) {
                        Ok(()) => break,
                        Err(err) => transition = Transition::Trap(err),
                    },
                    Transition::Shutdown => {
                        self.shutdown();
                        break;
                    },
                    Transition::CleanExit | Transition::Destroy => return,
                    Transition::PowerOn => break,
                }
            }
        }
    }

    async fn boot_firmware(&self) -> Result<Firmware<M, Linked<M>>, RuntimeError> {
        let firmware = Firmware::new(Arc::clone(&self.image));
        firmware.link(self.runtime.clone()).await
    }

    async fn run(&mut self) -> Transition {
        let _ = self.link.status_tx.send(Status::Running);
        let mut firmware = match self.boot_firmware().await {
            Ok(fw) => fw,
            Err(err) => return Transition::Trap(err),
        };

        let boot = firmware.boot();
        tokio::pin!(boot);

        let _ = self.link.error_tx.send(None);

        loop {
            tokio::select! {
                biased;
                Some(cmd) = self.link.cmd_rx.recv() => {
                    match cmd {
                        Command::Shutdown(reply) => {
                            let _ = reply.send(Ok(()));
                            return Transition::Shutdown;
                        }
                        Command::Destroy => return Transition::Destroy,
                        Command::PowerOn(reply) => {
                            let _ = reply.send(Err(RuntimeError::InvalidState(
                                "already running".into(),
                            )));
                        }
                        Command::Reset(kind, reply) => {
                            let _ = reply.send(Ok(()));
                            return Transition::Reset(kind);
                        }
                        other => self.handle_command(other),
                    }
                }
                result = &mut boot => {
                    return match result {
                        Ok(()) => Transition::CleanExit,
                        Err(err) => Transition::Trap(Self::unwrap(err)),
                    };
                }
            }
        }
    }

    async fn trap(&mut self, err: RuntimeError) -> Transition {
        tracing::error!(%err, "firmware halted");

        let _ = self.link.status_tx.send(Status::Halted);
        let _ = self.link.error_tx.send(Some(Arc::new(err)));

        self.await_recovery().await
    }

    async fn await_power_on(&mut self) -> Transition {
        while let Some(cmd) = self.link.cmd_rx.recv().await {
            match cmd {
                Command::PowerOn(reply) => {
                    let _ = reply.send(Ok(()));
                    return Transition::PowerOn;
                },
                Command::Read(addr, width, reply) => {
                    let _ = reply.send(self.handle_read(addr, width));
                },
                Command::Tick(_, reply) | Command::Shutdown(reply) => {
                    let _ = reply.send(Ok(()));
                },
                Command::Reset(_, reply) => {
                    let _ = reply.send(Err(RuntimeError::InvalidState("cannot reset: MCU is off".into())));
                },
                Command::Destroy => return Transition::Destroy,
            }
        }

        Transition::Destroy
    }

    async fn await_recovery(&mut self) -> Transition {
        while let Some(cmd) = self.link.cmd_rx.recv().await {
            match cmd {
                Command::Reset(kind, reply) => {
                    let _ = reply.send(Ok(()));
                    return Transition::Reset(kind);
                },
                Command::Shutdown(reply) => {
                    let _ = reply.send(Ok(()));
                    return Transition::Shutdown;
                },
                Command::Read(addr, width, reply) => {
                    let _ = reply.send(self.handle_read(addr, width));
                },
                Command::Tick(_, reply) => {
                    let _ = reply.send(Ok(()));
                },
                Command::PowerOn(reply) => {
                    let _ = reply.send(Err(RuntimeError::InvalidState("MCU is halted, use reset".into())));
                },
                Command::Destroy => return Transition::Destroy,
            }
        }

        Transition::Destroy
    }

    fn reset(&mut self, kind: ResetKind) -> Result<(), RuntimeError> {
        {
            let mut state = self
                .runtime
                .state
                .lock()
                .map_err(|_| RuntimeError::LockPoisoned("state".into()))?;
            state.mcu.reset(kind);
        }

        if matches!(kind, ResetKind::Cold) {
            let mut writes = self
                .runtime
                .link
                .write_rx
                .lock()
                .map_err(|_| RuntimeError::LockPoisoned("write_rx".into()))?;
            while writes.try_recv().is_ok() {}
        }

        self.reset = true;
        Ok(())
    }

    fn shutdown(&self) {
        let _ = self.link.status_tx.send(Status::Off);
    }

    fn handle_command(&self, cmd: Command) {
        match cmd {
            Command::Read(addr, width, reply) => {
                let result = self.handle_read(addr, width);
                let _ = reply.send(result);
            },
            Command::Tick(elapsed, reply) => {
                let result = self.handle_tick(elapsed);
                let _ = reply.send(result);
            },
            // Handled in the state machine loop above — listed explicitly so adding
            // a new Command variant produces a compiler error here.
            Command::PowerOn(_) | Command::Reset(..) | Command::Shutdown(_) | Command::Destroy => {
                unreachable!()
            },
        }
    }

    fn handle_read(&self, addr: Addr, width: Width) -> Result<Value, RuntimeError> {
        let mut state = self
            .runtime
            .state
            .lock()
            .map_err(|_| RuntimeError::LockPoisoned("state".into()))?;
        state.mcu.read(addr, width).map_err(RuntimeError::Mcu)
    }

    fn handle_tick(&self, elapsed: Duration) -> Result<(), RuntimeError> {
        {
            let mut state = self
                .runtime
                .state
                .lock()
                .map_err(|_| RuntimeError::LockPoisoned("state".into()))?;
            let spec = state.mcu.spec();

            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                clippy::cast_precision_loss
            )]
            let (cycles, compute) = {
                let cycles = (elapsed.as_secs_f64() * spec.clock_hz as f64) as u64;
                let compute = (cycles as f64 * spec.compute_per_cycle) as u64;

                (cycles, compute)
            };

            state.compute = Budget::new(compute);
            state.cycles = Budget::new(cycles);
        }

        self.runtime.link.tick_notify.notify_waiters();

        Ok(())
    }

    fn unwrap(err: RuntimeError) -> RuntimeError {
        match err {
            RuntimeError::Trap(wasm_err) => match wasm_err.downcast::<RuntimeError>() {
                Ok(inner) => inner,
                Err(wasm_err) => RuntimeError::Trap(wasm_err),
            },
            other => other,
        }
    }
}
