use std::sync::{Arc, Mutex};
use tokio::sync::{mpsc, watch, Notify};

use crate::{
    error::ErrorHandle,
    mcu::{
        config::Config,
        executor::Status,
        handle::{Command, Write},
    },
};

/// Channel endpoints owned by the [`Handle`](super::Handle).
#[derive(Debug, Clone)]
pub struct HandleLink {
    pub write_tx: mpsc::Sender<Write>,
    pub cmd_tx: mpsc::Sender<Command>,
    pub error_rx: watch::Receiver<ErrorHandle>,
    pub status_rx: watch::Receiver<Status>,
}

/// Channel endpoints owned by the [`Executor`](super::executor::Executor).
#[derive(Debug)]
pub struct ExecutorLink {
    pub cmd_rx: mpsc::Receiver<Command>,
    pub error_tx: watch::Sender<ErrorHandle>,
    pub status_tx: watch::Sender<Status>,
}

/// Channel endpoints owned by the [`Runtime`](super::runtime::Runtime).
#[derive(Debug, Clone)]
pub struct RuntimeLink {
    pub write_rx: Arc<Mutex<mpsc::Receiver<Write>>>,
    pub tick_notify: Arc<Notify>,
}

/// All channel links needed to wire up the Handle, Executor, and Runtime.
#[derive(Debug)]
pub struct Channels {
    /// Endpoints for the [`Handle`](super::Handle).
    pub handle: HandleLink,
    /// Endpoints for the [`Runtime`](super::runtime::Runtime).
    pub runtime: RuntimeLink,
    /// Endpoints for the [`Executor`](super::executor::Executor).
    pub executor: ExecutorLink,
}

impl Channels {
    /// Creates all channels sized according to the given config.
    pub fn new(config: &Config) -> Self {
        let tick_notify = Arc::new(Notify::new());
        let (write_tx, write_rx) = mpsc::channel(config.write_buffer);
        let (cmd_tx, cmd_rx) = mpsc::channel(config.cmd_buffer);
        let (error_tx, error_rx) = watch::channel(None);
        let (status_tx, status_rx) = watch::channel(Status::Off);

        Self {
            handle: HandleLink {
                write_tx,
                cmd_tx,
                error_rx,
                status_rx,
            },
            runtime: RuntimeLink {
                write_rx: Arc::new(Mutex::new(write_rx)),
                tick_notify: Arc::clone(&tick_notify),
            },
            executor: ExecutorLink {
                cmd_rx,
                error_tx,
                status_tx,
            },
        }
    }
}
