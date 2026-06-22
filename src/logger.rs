use std::fmt::{Arguments, Display, Formatter};
use anyhow::Error;
use dif_macros::{dynamic_service, service, Service};
use chrono::{Utc};

use dif_core as dif;
use dif_core::FromInjector;
use dif_core::sync::InjectorLockDyn;

#[derive(Clone, Copy, Ord, PartialOrd, Eq, PartialEq)]
pub enum LogLevel {
    Trace = 1,
    Debug = 2,
    Info = 3,
    Warn = 4,
    Error = 5,
    Fatal = 6,
}

impl Display for LogLevel {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLevel::Trace => f.write_str("Trace"),
            LogLevel::Debug => f.write_str("Debug"),
            LogLevel::Info => f.write_str("Info"),
            LogLevel::Warn => f.write_str("Warn"),
            LogLevel::Error => f.write_str("Error"),
            LogLevel::Fatal => f.write_str("Fatal"),
        }
    }
}

pub struct Logger {
    children: Vec<InjectorLockDyn<dyn ChildLogger>>,
}

#[service]
impl Logger {
    pub fn new(children: impl Iterator<Item = InjectorLockDyn<dyn ChildLogger>>) -> Self {
        Self {
            children: children.collect(),
        }
    }
}

impl Logger {
    pub fn log(&self, level: LogLevel, message: impl Display, error: impl Into<anyhow::Error>) {
        let error = error.into();
        let message = format_args!("{}", message);
        
        for child in self.children.iter() {
            let mut child = child.lock()
                .unwrap();
            
            if child.can_handle(level) {
                child.log(level, &message, &error);
            }
        }
    }
}

#[dynamic_service]
pub trait ChildLogger : Send + Sync {
    fn can_handle(&self, log_level: LogLevel) -> bool;
    
    fn log(&mut self, level: LogLevel, message: &Arguments<'_>, error: &anyhow::Error);
}

#[derive(Service)]
pub struct TimedLogger<T : ChildLogger + FromInjector + 'static> {
    inner: T,
}

#[service]
impl<T : ChildLogger + FromInjector + 'static> ChildLogger for TimedLogger<T> {
    fn can_handle(&self, log_level: LogLevel) -> bool {
        self.inner
            .can_handle(log_level)
    }

    fn log(&mut self, level: LogLevel, message: &Arguments<'_>, error: &Error) {
        let time = Utc::now().format("%Y-%m-%d %H:%M:%S");
        
        let message = format_args!("[{}] {}", time, message);
        self.inner
            .log(level, &message, error)
    }
}