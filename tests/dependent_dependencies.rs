use dil::service;
use std::cell::Cell;
use dil::sync::{Lock, Mutex};

thread_local! {
    pub static DEPENDENT_INITIALIZED: Cell<bool> = Cell::new(false);
    pub static DEPENDENCY_INITIALIZED: Cell<bool> = Cell::new(false);
}

pub struct Dependent {
    #[allow(dead_code)]
    dependency: <Mutex as Lock>::Lock<Dependency>,
}

pub struct Dependency {
    #[allow(unused)]
    s: u32
}

#[service]
impl Dependent {
    pub fn new(dependent: <Mutex as Lock>::Lock<Dependency>) -> Self {
        DEPENDENT_INITIALIZED.replace(true);
        
        Self {
            dependency: dependent,
        }
    }
}

#[service]
impl Dependency {
    pub fn new() -> Self {
        DEPENDENCY_INITIALIZED.replace(true);
        
        Self {
            s: 100,
        }
    }
}

pub fn reset() {
    DEPENDENT_INITIALIZED.replace(false);
    DEPENDENCY_INITIALIZED.replace(false);
}

#[allow(unused)]
pub struct CircularDependency {

}

#[service]
impl CircularDependency {
    #[allow(unused)]
    pub fn new(_dependency: <Mutex as Lock>::Lock<CircularDependency>) -> Self {
        Self {}
    }
}