#![expect(
    clippy::unused_async,
    reason = "Wasmtime requires all import functions to be async when using an async runtime."
)]
#![expect(clippy::cast_possible_truncation)]

use std::str;
use tracing::info;
use wasmtime::{Caller, Error};

use crate::{
    error::RuntimeError,
    logging::target,
    mcu::{
        mmio::{Read, Write},
        runtime::Runtime,
    },
    traits::Mcu,
    volatile_access::{Value, Width},
};

pub fn trap(err: impl Into<RuntimeError>) -> wasmtime::Error {
    wasmtime::format_err!(err.into())
}

// --- Debug ---

pub async fn log<M: Mcu + Send + 'static>(mut caller: Caller<'_, Runtime<M>>, ptr: u32, len: u32) -> Result<(), Error> {
    Runtime::<M>::meter(&mut caller).map_err(trap)?;

    let memory = caller
        .get_export("memory")
        .and_then(wasmtime::Extern::into_memory)
        .ok_or_else(|| trap(RuntimeError::MissingExport("memory".to_string())))?;

    let data = memory.data(&caller);
    let start = ptr as usize;
    let end = start + len as usize;

    let slice = data.get(start..end).ok_or_else(|| {
        trap(RuntimeError::InvalidArgument(format!(
            "debug_log: pointer {ptr:#X} + len {len} out of bounds"
        )))
    })?;

    let msg = str::from_utf8(slice).map_err(|e| trap(RuntimeError::InvalidArgument(e.to_string())))?;

    info!(target: target::FIRMWARE, message = msg);

    Ok(())
}

// --- WFI ---
//
pub async fn wfi<M: Mcu + Send + 'static>(mut caller: Caller<'_, Runtime<M>>) -> Result<(), Error> {
    Runtime::<M>::wfi(&mut caller).await.map_err(trap)
}

// --- Read Volatile ---

pub async fn read_volatile_u8<M: Mcu + Send + 'static>(
    mut caller: Caller<'_, Runtime<M>>,
    addr: u32,
) -> Result<u32, Error> {
    let op = Read::new(addr.into(), Width::U8);
    let val = Runtime::<M>::mmio(&mut caller, op).await.map_err(trap)?;

    Ok(u32::from(val))
}

pub async fn read_volatile_u16<M: Mcu + Send + 'static>(
    mut caller: Caller<'_, Runtime<M>>,
    addr: u32,
) -> Result<u32, Error> {
    let op = Read::new(addr.into(), Width::U16);
    let val = Runtime::<M>::mmio(&mut caller, op).await.map_err(trap)?;

    Ok(u32::from(val))
}

pub async fn read_volatile_u32<M: Mcu + Send + 'static>(
    mut caller: Caller<'_, Runtime<M>>,
    addr: u32,
) -> Result<u32, Error> {
    let op = Read::new(addr.into(), Width::U32);
    let val = Runtime::<M>::mmio(&mut caller, op).await.map_err(trap)?;

    Ok(u32::from(val))
}

// --- Write Volatile ---

pub async fn write_volatile_u8<M: Mcu + Send + 'static>(
    mut caller: Caller<'_, Runtime<M>>,
    addr: u32,
    val: u32,
) -> Result<(), Error> {
    if val > u32::from(u8::MAX) {
        return Err(trap(RuntimeError::InvalidArgument(format!(
            "value {val:#X} exceeds 8-bit range (max {:#X})",
            u8::MAX
        ))));
    }

    let op = Write::new(addr.into(), Value::U8(val as u8));
    Runtime::<M>::mmio(&mut caller, op).await.map_err(trap)?;

    Ok(())
}

pub async fn write_volatile_u16<M: Mcu + Send + 'static>(
    mut caller: Caller<'_, Runtime<M>>,
    addr: u32,
    val: u32,
) -> Result<(), Error> {
    if val > u32::from(u16::MAX) {
        return Err(trap(RuntimeError::InvalidArgument(format!(
            "value {val:#X} exceeds 16-bit range (max {:#X})",
            u16::MAX
        ))));
    }

    let op = Write::new(addr.into(), Value::U16(val as u16));
    Runtime::<M>::mmio(&mut caller, op).await.map_err(trap)?;

    Ok(())
}

pub async fn write_volatile_u32<M: Mcu + Send + 'static>(
    mut caller: Caller<'_, Runtime<M>>,
    addr: u32,
    val: u32,
) -> Result<(), Error> {
    let op = Write::new(addr.into(), val.into());
    Runtime::<M>::mmio(&mut caller, op).await.map_err(trap)?;

    Ok(())
}
