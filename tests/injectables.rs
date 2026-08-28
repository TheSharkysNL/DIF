use std::any::{TypeId};
pub use dilian::service;
use dilian::{dynamic_service};
use std::cell::{Cell, RefCell};
use dilian::sync::{Lock, Lockable};
use dilian::Service;

thread_local! {
    pub static INITIALIZE_COUNT: Cell<u32> = Cell::new(0);
    pub static DROP_COUNT: Cell<u32> = Cell::new(0);
    pub static WRITTEN_STRING: RefCell<String> = RefCell::new(String::new());
}

thread_local! {
    pub static OTHER_INITIALIZE_COUNT: Cell<u32> = Cell::new(0);
    pub static OTHER_DROP_COUNT: Cell<u32> = Cell::new(0);
}

pub struct TestLogger {
}

#[service]
impl TestLogger {
    pub fn new() -> Self {
        let count = INITIALIZE_COUNT.get();
        INITIALIZE_COUNT.set(count + 1);
        Self {
        }
    }

    pub fn write(&mut self, message: &str) {
        WRITTEN_STRING.with_borrow_mut(|x| {
            x.push_str(message);
        });
    }
}

impl Drop for TestLogger {
    fn drop(&mut self) {
        let count = DROP_COUNT.get();
        DROP_COUNT.set(count + 1);
    }
}

pub fn reset() {
    INITIALIZE_COUNT.replace(0);
    DROP_COUNT.replace(0);
    WRITTEN_STRING.replace(String::new());
    
    OTHER_DROP_COUNT.replace(0);
    OTHER_INITIALIZE_COUNT.replace(0);
}

pub struct AnotherLogger {
}

#[service]
impl AnotherLogger {
    pub fn new() -> Self {
        let count = OTHER_INITIALIZE_COUNT.get();
        OTHER_INITIALIZE_COUNT.set(count + 1);
        
        Self { }
    }
}

impl Drop for AnotherLogger {
    fn drop(&mut self) {
        let count = OTHER_DROP_COUNT.get();
        OTHER_DROP_COUNT.set(count + 1);
    }
}

#[dynamic_service]
pub trait Logger : Send + Sync {
    fn write(&mut self, message: &str);
    
    fn type_id(&self) -> TypeId {
        TypeId::of::<Self>()
    }
}

#[service]
impl Logger for AnotherLogger {
    fn write(&mut self, _: &str) {
        // do nothing
    }
}

#[service]
impl Logger for TestLogger {
    fn write(&mut self, message: &str) {
        self.write(message); // call original write
    }
}

#[derive(Service)]
pub struct AnotherService {
    
}

#[service]
impl Logger for AnotherService {
    fn write(&mut self, _: &str) {
        // do nothing again
    }
}

pub struct LoggerUser<L : Lock> {
    logger: L::Lock<TestLogger>,
}

#[service]
impl<L : Lock + 'static> LoggerUser<L> {
    pub fn new(logger: L::Lock<TestLogger>) -> Self {
        Self {
            logger,
        }
    }
    
    pub fn write_my_logs(&self)
        where <L as Lock>::Lock<TestLogger> : Lockable<TestLogger>
    {
        let mut logger = self.logger.write();
        logger.write("my message");
    }
}