use std::sync::{Arc, Mutex};

use tracing::{
    field::{Field, Visit},
    span::{Attributes, Id},
    Event, Level, Span, Subscriber,
};
use tracing_subscriber::{layer::Context as LayerContext, registry::LookupSpan, Layer};

use crate::error::LogError;

pub mod target {
    pub const FIRMWARE: &str = "firmware";
    pub const RUNTIME: &str = "runtime";
    pub const HARDWARE: &str = "hardware";
}

const SPAN_NAME: &str = "handle";
const SPAN_LEVEL: Level = Level::INFO;

struct SysField(String);
struct McuField(String);

#[derive(Default)]
struct McuVisitor {
    value: String,
}

impl Visit for McuVisitor {
    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "mcu" {
            self.value = value.to_string();
        }
    }

    fn record_debug(&mut self, field: &Field, value: &dyn core::fmt::Debug) {
        if field.name() == "mcu" {
            self.value = format!("{value:?}");
        }
    }
}

#[derive(Default)]
struct MessageVisitor {
    value: String,
}

impl Visit for MessageVisitor {
    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.value = value.to_string();
        }
    }

    fn record_debug(&mut self, field: &Field, value: &dyn core::fmt::Debug) {
        if field.name() == "message" {
            self.value = format!("{value:?}");
        }
    }
}

struct RegisteredHandler {
    system: String,
    mcu: Option<String>,
    handler: Arc<dyn LogHandler>,
}

pub(crate) struct LogHandlerRegistry {
    handlers: Mutex<Vec<RegisteredHandler>>,
}

impl LogHandlerRegistry {
    pub(crate) const fn new() -> Self {
        Self {
            handlers: Mutex::new(Vec::new()),
        }
    }

    pub(crate) fn register(
        &self,
        system: impl Into<String>,
        mcu: Option<impl Into<String>>,
        handler: Arc<dyn LogHandler>,
    ) -> Result<(), LogError> {
        self.handlers
            .lock()
            .map_err(|_| LogError::LockPoisoned)?
            .push(RegisteredHandler {
                system: system.into(),
                mcu: mcu.map(Into::into),
                handler,
            });
        Ok(())
    }
}

pub(crate) struct LogRouter {
    registry: Arc<LogHandlerRegistry>,
}

impl LogRouter {
    pub(crate) const fn new(registry: Arc<LogHandlerRegistry>) -> Self {
        Self { registry }
    }

    pub(crate) fn span(system: &str, mcu: Option<&str>) -> Span {
        tracing::span!(SPAN_LEVEL, SPAN_NAME, sys = %system, mcu = %mcu.unwrap_or(""))
    }
}

impl<S: Subscriber + for<'a> LookupSpan<'a>> Layer<S> for LogRouter {
    fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: LayerContext<'_, S>) {
        if attrs.metadata().name() != SPAN_NAME {
            return;
        }

        let mut sys = SysVisitor::default();
        attrs.record(&mut sys);

        let mut mcu = McuVisitor::default();
        attrs.record(&mut mcu);

        if let Some(span) = ctx.span(id) {
            span.extensions_mut().insert(SysField(sys.value));
            span.extensions_mut().insert(McuField(mcu.value));
        }
    }

    fn on_event(&self, event: &Event<'_>, ctx: LayerContext<'_, S>) {
        let (sys, mcu) = match ctx.event_scope(event) {
            Some(scope) => {
                let mut sys = None;
                let mut mcu = None;
                for span in scope.from_root() {
                    let exts = span.extensions();
                    if let Some(s) = exts.get::<SysField>() {
                        sys = Some(s.0.clone());
                    }
                    if let Some(m) = exts.get::<McuField>() {
                        mcu = if m.0.is_empty() { None } else { Some(m.0.clone()) };
                    }
                }
                (sys, mcu)
            },
            None => return,
        };

        let Some(sys) = sys else { return };

        let mut msg = MessageVisitor::default();
        event.record(&mut msg);

        let ctx = Context {
            sys: &sys,
            mcu: mcu.as_deref(),
            target: event.metadata().target(),
        };
        let level = *event.metadata().level();

        let Ok(handlers) = self.registry.handlers.lock() else {
            return;
        };

        for entry in handlers.iter() {
            let sys_match = entry.system == sys;
            let mcu_match = entry.mcu.as_deref() == mcu.as_deref();
            if sys_match && mcu_match {
                entry.handler.log(level, &ctx, &msg.value);
            }
        }
    }
}

pub struct Context<'a> {
    pub sys: &'a str,
    pub mcu: Option<&'a str>,
    pub target: &'a str,
}

impl std::fmt::Display for Context<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.mcu {
            Some(mcu) => write!(f, "({}/{}) {}", self.sys, mcu, self.target),
            None => write!(f, "({}) {}", self.sys, self.target),
        }
    }
}

pub trait LogHandler: Send + Sync {
    fn log(&self, level: Level, ctx: &Context<'_>, msg: &str);
}

pub struct StdOutLogHandler;

impl LogHandler for StdOutLogHandler {
    fn log(&self, level: Level, ctx: &Context<'_>, msg: &str) {
        let line = format!("[{level}] {ctx}: {msg}");

        match level {
            Level::ERROR | Level::WARN => eprintln!("{line}"),
            _ => println!("{line}"),
        }
    }
}

#[derive(Default)]
struct SysVisitor {
    value: String,
}

impl Visit for SysVisitor {
    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "sys" {
            self.value = value.to_string();
        }
    }

    fn record_debug(&mut self, field: &Field, value: &dyn core::fmt::Debug) {
        if field.name() == "sys" {
            self.value = format!("{value:?}");
        }
    }
}
