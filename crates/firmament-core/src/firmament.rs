use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::{
    logging::{LogHandler, LogHandlerRegistry, LogRouter, StdOutLogHandler},
    system::System,
};

pub struct Firmament {
    registry: Arc<LogHandlerRegistry>,
}

#[bon::bon]
impl Firmament {
    #[must_use]
    pub fn new() -> Self {
        let registry = Arc::new(LogHandlerRegistry::new());
        let router = LogRouter::new(Arc::clone(&registry));

        tracing_subscriber::registry().with(router).init();

        Self { registry }
    }

    #[builder(finish_fn = build)]
    pub fn system(
        &self,
        #[builder(start_fn)] name: impl Into<String>,
        #[builder(default = Box::new(StdOutLogHandler) as Box<dyn LogHandler>)] log_handler: Box<dyn LogHandler>,
    ) -> System {
        System::new(name, Arc::clone(&self.registry), Arc::from(log_handler))
    }
}

impl Default for Firmament {
    fn default() -> Self {
        Self::new()
    }
}
