use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::sync::oneshot;
use tracing::{Instrument, Span};

use crate::{
    error::{ErrorHandle, RuntimeError},
    mcu::{
        channels::{Channels, HandleLink},
        config::Config,
        executor::{Executor, Status},
        firmware::Image,
        runtime::Runtime,
        state::State,
    },
    traits::{Mcu, ResetKind},
    volatile_access::{Addr, Value, VolatileAccess, Width},
};

/// An external write: an address-value pair sent from the handle to the runtime.
pub type Write = (Addr, Value);

/// Commands sent from the handle to the executor over the command channel.
#[derive(Debug)]
pub enum Command {
    /// Boot the firmware.
    PowerOn(oneshot::Sender<Result<(), RuntimeError>>),
    /// Reset the MCU with the given kind.
    Reset(ResetKind, oneshot::Sender<Result<(), RuntimeError>>),
    /// Shut down the MCU gracefully.
    Shutdown(oneshot::Sender<Result<(), RuntimeError>>),
    /// Read a value from a bus address.
    Read(Addr, Width, oneshot::Sender<Result<Value, RuntimeError>>),
    /// Advance the simulation clock by the given duration.
    Tick(Duration, oneshot::Sender<Result<(), RuntimeError>>),
    /// Tear down the executor task immediately.
    Destroy,
}

/// Public API for controlling an MCU simulation from external code.
#[derive(Debug)]
pub struct Handle {
    system: String,
    name: String,
    link: HandleLink,
}

impl Handle {
    /// Creates a new MCU simulation from WASM firmware bytes and an MCU instance.
    ///
    /// Compiles the firmware, spawns the executor task, and returns a handle.
    /// The MCU starts in the `Off` state. Call [`power_on`](Self::power_on) to boot.
    ///
    /// # Errors
    ///
    /// Returns [`RuntimeError::CompilationError`] if the WASM bytes are invalid,
    /// or [`RuntimeError::LinkError`] if a required host import is missing.
    pub(crate) fn new<M: Mcu + Send + 'static>(
        system: impl Into<String>,
        name: impl Into<String>,
        config: Config,
        image: &[u8],
        mcu: M,
        span: Span,
    ) -> Result<Self, RuntimeError> {
        let channels = Channels::new(&config);
        let handle = Self {
            system: system.into(),
            name: name.into(),
            link: channels.handle,
        };

        let state = Arc::new(Mutex::new(State::new(mcu)));
        let image = Arc::new(Image::new(image, config.yield_interval)?);
        let runtime = Runtime::new(Arc::clone(&state), channels.runtime);
        let executor = Executor::new(Arc::clone(&image), runtime, channels.executor);

        tokio::spawn(executor.exec().instrument(span));

        Ok(handle)
    }

    /// Returns the name of the MCU handle.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn system(&self) -> &str {
        &self.system
    }

    /// Returns the current MCU lifecycle status.
    #[must_use]
    pub fn status(&self) -> Status {
        *self.link.status_rx.borrow()
    }

    /// Returns the last runtime error, if the MCU is halted.
    #[must_use]
    pub fn error(&self) -> ErrorHandle {
        self.link.error_rx.borrow().clone()
    }

    /// Powers on the MCU, linking and booting the firmware.
    ///
    /// Only valid when the MCU is in the `Off` state.
    ///
    /// # Errors
    ///
    /// Returns [`RuntimeError::InvalidState`] if the MCU is already running or halted.
    /// Returns [`RuntimeError::CompilationError`] or [`RuntimeError::LinkError`] if
    /// firmware linking fails.
    pub async fn power_on(&self) -> Result<(), RuntimeError> {
        let (tx, rx) = oneshot::channel();

        self.link
            .cmd_tx
            .send(Command::PowerOn(tx))
            .await
            .map_err(|_| self.resolve())?;

        rx.await.map_err(|_| self.resolve())?
    }

    /// Resets the MCU and reboots the firmware.
    ///
    /// `Cold` resets wipe all MCU state and drain pending writes.
    /// `Warm` resets reboot the firmware but preserve MCU register state.
    ///
    /// # Errors
    ///
    /// Returns [`RuntimeError::InvalidState`] if the MCU is off.
    pub async fn reset(&self, kind: ResetKind) -> Result<(), RuntimeError> {
        let (tx, rx) = oneshot::channel();
        let cmd = Command::Reset(kind, tx);
        self.link
            .cmd_tx
            .send(cmd)
            .await
            .map_err(|_| self.resolve())?;

        rx.await.map_err(|_| self.resolve())?
    }

    /// Shuts down the MCU. The firmware stops and the MCU enters the `Off` state.
    ///
    /// No-op if already off. The MCU can be powered on again with [`power_on`](Self::power_on).
    ///
    /// # Errors
    ///
    /// Returns [`RuntimeError::ChannelClosed`] if the executor task has exited.
    pub async fn shutdown(&self) -> Result<(), RuntimeError> {
        let (tx, rx) = oneshot::channel();
        let cmd = Command::Shutdown(tx);
        self.link
            .cmd_tx
            .send(cmd)
            .await
            .map_err(|_| self.resolve())?;

        rx.await.map_err(|_| self.resolve())?
    }

    /// Destroys the executor task. No reply is awaited.
    pub async fn destroy(&self) {
        let cmd = Command::Destroy;
        let _ = self.link.cmd_tx.send(cmd).await;
    }

    /// Reads a typed value from a bus address on the MCU.
    ///
    /// The access width is determined by `T` (u8, u16, u32).
    ///
    /// # Errors
    ///
    /// Returns [`RuntimeError::Mcu`] if the bus address is unmapped or the device
    /// rejects the read, or if the returned value width doesn't match `T`.
    pub async fn read<T: VolatileAccess>(&self, addr: impl Into<Addr>) -> Result<T, RuntimeError> {
        let (tx, rx) = oneshot::channel();
        self.link
            .cmd_tx
            .send(Command::Read(addr.into(), T::width(), tx))
            .await
            .map_err(|_| self.resolve())?;
        let val = rx.await.map_err(|_| self.resolve())??;

        T::from_value(val).ok_or_else(|| {
            RuntimeError::Mcu(crate::error::McuError::WidthMismatch {
                requested: T::width().size(),
                actual: val.size(),
            })
        })
    }

    /// Writes a typed value to a bus address on the MCU.
    ///
    /// Writes bypass the command channel and are drained at every MMIO boundary,
    /// modeling external stimulus (sensors, pin changes) that arrives at any time.
    ///
    /// # Errors
    ///
    /// Returns [`RuntimeError::ChannelClosed`] if the executor task has exited.
    pub async fn write<T: VolatileAccess>(&self, addr: impl Into<Addr>, val: T) -> Result<(), RuntimeError> {
        let msg = (addr.into(), val.to_value());
        self.link
            .write_tx
            .send(msg)
            .await
            .map_err(|_| self.resolve())?;

        Ok(())
    }

    /// Advances the simulation clock by the given elapsed duration.
    ///
    /// Computes cycle and compute budgets from the MCU's clock speed, then
    /// notifies the runtime to resume firmware execution.
    ///
    /// # Errors
    ///
    /// Returns [`RuntimeError::ChannelClosed`] if the executor task has exited.
    pub async fn tick(&self, elapsed: Duration) -> Result<(), RuntimeError> {
        let (tx, rx) = oneshot::channel();
        self.link
            .cmd_tx
            .send(Command::Tick(elapsed, tx))
            .await
            .map_err(|_| self.resolve())?;
        rx.await.map_err(|_| self.resolve())?
    }

    fn resolve(&self) -> RuntimeError {
        self.link.error_rx.borrow().as_ref().map_or_else(
            || RuntimeError::ChannelClosed,
            |err| RuntimeError::Halted(Arc::clone(err)),
        )
    }
}
