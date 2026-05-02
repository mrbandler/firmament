use std::sync::Arc;

use crate::{
    error::FirmamentError,
    logging::{LogHandler, LogHandlerRegistry, LogRouter},
    mcu::{Config, Handle},
    traits::Mcu,
};

pub struct System {
    name: String,
    registry: Arc<LogHandlerRegistry>,
    log_handler: Arc<dyn LogHandler>,
}

#[bon::bon]
impl System {
    pub(crate) fn new(
        name: impl Into<String>,
        registry: Arc<LogHandlerRegistry>,
        log_handler: Arc<dyn LogHandler>,
    ) -> Self {
        Self {
            name: name.into(),
            registry,
            log_handler,
        }
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[builder(finish_fn = build)]
    pub fn mcu<M: Mcu + Send + 'static>(
        &self,
        #[builder(start_fn)] name: impl Into<String>,
        image: &[u8],
        device: M,
        log_handler: Option<Box<dyn LogHandler>>,
        #[builder(default)] config: Config,
    ) -> Result<Handle, FirmamentError> {
        let name = name.into();
        let handler = log_handler.map_or_else(|| Arc::clone(&self.log_handler), Arc::from);

        self.registry.register(&self.name, Some(&name), handler)?;

        let span = LogRouter::span(&self.name, Some(&name));
        let handle = Handle::new(self.name.clone(), name, config, image, device, span)?;

        Ok(handle)
    }
}
