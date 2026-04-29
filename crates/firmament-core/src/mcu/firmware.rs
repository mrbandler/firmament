use std::{marker::PhantomData, sync::Arc};
use wasmtime::{Config, Engine, Instance, Linker, Module, Store};

use crate::{
    error::RuntimeError,
    mcu::{imports, runtime::Runtime},
    traits::Mcu,
};

/// Pre-compiled WASM firmware image with its Wasmtime engine.
#[derive(Debug, Clone)]
pub struct Image {
    /// Wasmtime engine configured for fuel metering.
    engine: Engine,
    /// Compiled WASM module.
    module: Module,
    /// Fuel between async yield points.
    yield_interval: u64,
}

impl Image {
    /// Compiles WASM bytes into a reusable firmware image.
    pub fn new(bytes: &[u8], yield_interval: u64) -> Result<Self, RuntimeError> {
        let mut config = Config::default();
        config.consume_fuel(true);

        let engine = Engine::new(&config).map_err(RuntimeError::CompilationError)?;
        let module = Module::new(&engine, bytes).map_err(RuntimeError::CompilationError)?;

        Ok(Self {
            engine,
            module,
            yield_interval,
        })
    }
}

/// Typestate: firmware is compiled but not yet linked.
pub struct Compiled;

/// Typestate: firmware is linked with a WASM instance and store, ready to boot.
pub struct Linked<M: Mcu + Send + 'static> {
    instance: Instance,
    store: Store<Runtime<M>>,
}

/// Typestate-driven firmware lifecycle: `Compiled` -> `Linked` -> booted.
pub struct Firmware<M: Mcu + Send + 'static, S = Compiled> {
    image: Arc<Image>,
    phase: S,
    _mcu: PhantomData<M>,
}

impl<M: Mcu + Send + 'static> Firmware<M, Compiled> {
    /// Creates compiled firmware from a shared image.
    pub const fn new(image: Arc<Image>) -> Self {
        Self {
            image,
            phase: Compiled,
            _mcu: PhantomData,
        }
    }

    /// Links host imports and instantiates the WASM module, producing a bootable firmware.
    pub async fn link(self, runtime: Runtime<M>) -> Result<Firmware<M, Linked<M>>, RuntimeError> {
        let mut store = Store::new(&self.image.engine, runtime);
        let mut linker = Linker::new(&self.image.engine);

        store
            .fuel_async_yield_interval(Some(self.image.yield_interval))
            .map_err(RuntimeError::Trap)?;
        store.set_fuel(u64::MAX).map_err(RuntimeError::Trap)?;

        macro_rules! link_func {
            ($name:expr, $func:expr) => {
                linker
                    .func_wrap_async("fm", $name, |caller, ()| {
                        Box::new($func(caller))
                    })
                    .map_err(|_| RuntimeError::LinkError($name.to_string()))?
            };
            ($name:expr, $func:expr, $($param:ident : $ty:ty),+) => {
                linker
                    .func_wrap_async("fm", $name, |caller, ($($param,)+): ($($ty,)+)| {
                        Box::new($func(caller, $($param),+))
                    })
                    .map_err(|_| RuntimeError::LinkError($name.to_string()))?
            };
        }

        link_func!("debug_log", imports::debug_log::<M>, ptr: u32, len: u32);
        link_func!("wfi", imports::wfi::<M>);
        link_func!("read_volatile_u8", imports::read_volatile_u8::<M>, addr: u32);
        link_func!("read_volatile_u16", imports::read_volatile_u16::<M>, addr: u32);
        link_func!("read_volatile_u32", imports::read_volatile_u32::<M>, addr: u32);
        link_func!("write_volatile_u8", imports::write_volatile_u8::<M>, addr: u32, val: u32);
        link_func!("write_volatile_u16", imports::write_volatile_u16::<M>, addr: u32, val: u32);
        link_func!("write_volatile_u32", imports::write_volatile_u32::<M>, addr: u32, val: u32);

        let instance = linker
            .instantiate_async(&mut store, &self.image.module)
            .await
            .map_err(RuntimeError::CompilationError)?;

        Ok(Firmware::<M, Linked<M>> {
            image: self.image,
            phase: Linked { instance, store },
            _mcu: self._mcu,
        })
    }
}

impl<M: Mcu + Send + 'static> Firmware<M, Linked<M>> {
    /// Calls the firmware's `_start` entry point.
    pub async fn boot(&mut self) -> Result<(), RuntimeError> {
        let start_fn = self
            .phase
            .instance
            .get_typed_func::<(), ()>(&mut self.phase.store, "_start")
            .map_err(|_| RuntimeError::MissingEntryPoint)?;

        start_fn
            .call_async(&mut self.phase.store, ())
            .await
            .map_err(RuntimeError::Trap)
    }
}
