use miette::Diagnostic;
use std::{error::Error, fmt::Display, sync::Arc};
use thiserror::Error;

/// Optional shared reference to a runtime error, used to propagate halt reasons.
pub type ErrorHandle = Option<Arc<RuntimeError>>;

/// Constructs a `Custom` variant from any displayable message.
pub trait Custom {
    /// Creates a `Custom` error variant from the given message.
    fn custom(msg: impl Display) -> Self;
}

macro_rules! impl_custom {
    ($($ty:ty),+) => {
        $(
            impl Custom for $ty {
                fn custom(msg: impl Display) -> Self {
                    Self::Custom(msg.to_string().into())
                }
            }
        )+
    };
}

/// Errors from device register reads.
#[derive(Debug, Error, Diagnostic)]
pub enum ReadError {
    #[error("read: invalid register offset {offset:#X} (range: {range_start:#X}..{range_end:#X})")]
    InvalidOffset {
        offset: u32,
        range_start: u32,
        range_end: u32,
    },

    #[error("read: register at offset {offset:#X} is write-only")]
    WriteOnly { offset: u32 },

    #[error("read: unaligned {width}-bit access at offset {offset:#X}")]
    UnalignedAccess { offset: u32, width: u8 },

    #[error("read: {0}")]
    Custom(Box<dyn Error + Send + Sync>),
}

/// Errors from device register writes.
#[derive(Debug, Error, Diagnostic)]
pub enum WriteError {
    #[error("write: invalid register offset {offset:#X} (range: {range_start:#X}..{range_end:#X})")]
    InvalidOffset {
        offset: u32,
        range_start: u32,
        range_end: u32,
    },

    #[error("write: register at offset {offset:#X} is read-only")]
    ReadOnly { offset: u32 },

    #[error("write: unaligned {width}-bit access at offset {offset:#X}")]
    UnalignedAccess { offset: u32, width: u8 },

    #[error("write: value {value:#X} exceeds {width}-bit range (max {max:#X})")]
    #[diagnostic(help("WASM passes all values as i32 — ensure your value fits the declared width"))]
    ValueOverflow { value: u32, width: u8, max: u32 },

    #[error("write: {0}")]
    Custom(Box<dyn Error + Send + Sync>),
}

/// Errors originating from a peripheral device.
#[derive(Debug, Error, Diagnostic)]
pub enum DeviceError {
    #[error(transparent)]
    Read(#[from] ReadError),

    #[error(transparent)]
    Write(#[from] WriteError),

    #[error("device: {0}")]
    Custom(Box<dyn Error + Send + Sync>),
}

/// Errors from bus-level address resolution and device dispatch.
#[derive(Debug, Error, Diagnostic)]
pub enum BusError {
    #[error("bus: no device at address {addr:#010X}")]
    UnmappedAddress { addr: u32 },

    #[error(transparent)]
    Device(#[from] DeviceError),

    #[error("bus: {0}")]
    Custom(Box<dyn Error + Send + Sync>),
}

/// Top-level MCU errors wrapping bus faults and width mismatches.
#[derive(Debug, Error, Diagnostic)]
pub enum McuError {
    #[error(transparent)]
    Bus(#[from] BusError),

    #[error("mcu: width mismatch: requested {requested}-bit but got {actual}-bit")]
    WidthMismatch { requested: u8, actual: u8 },

    #[error("mcu: {0}")]
    Custom(Box<dyn Error + Send + Sync>),
}

/// Errors from the WASM runtime, firmware execution, and executor lifecycle.
#[derive(Debug, Error, Diagnostic)]
pub enum RuntimeError {
    #[error("runtime: failed to compile WASM module")]
    CompilationError(#[source] wasmtime::Error),

    #[error("runtime: failed to link host function: {0}")]
    LinkError(String),

    #[error("runtime: WASM module does not export a `_start` entry point")]
    MissingEntryPoint,

    #[error("runtime: missing ISR handler: __isr_{irq}")]
    #[diagnostic(help("export a `#[no_mangle] fn __isr_{irq}()` from your firmware"))]
    MissingIsr { irq: u8 },

    #[error("runtime: firmware trapped")]
    Trap(#[source] wasmtime::Error),

    #[error("runtime: invalid import argument: {0}")]
    InvalidArgument(String),

    #[error("runtime: invalid state: {0}")]
    InvalidState(String),

    #[error("runtime: missing export: {0}")]
    MissingExport(String),

    #[error("runtime: command channel closed")]
    ChannelClosed,

    #[error("runtime: state lock poisoned: {0}")]
    LockPoisoned(String),

    #[error(transparent)]
    Mcu(#[from] McuError),

    #[error(transparent)]
    Halted(Arc<Self>),

    #[error("runtime: {0}")]
    Custom(Box<dyn Error + Send + Sync>),
}

impl_custom!(McuError, BusError, DeviceError, ReadError, WriteError, RuntimeError);
