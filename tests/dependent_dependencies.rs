use dilian::service;
use std::cell::Cell;
use dilian::sync::{Lock, MutexMarker};

thread_local! {
    pub static DEPENDENT_INITIALIZED: Cell<bool> = Cell::new(false);
    pub static DEPENDENCY_INITIALIZED: Cell<bool> = Cell::new(false);
}

pub struct Dependent {
    #[allow(dead_code)]
    dependency: <MutexMarker as Lock>::Lock<Dependency>,
}

pub struct Dependency {
    #[allow(unused)]
    s: u32
}

#[service]
impl Dependent {
    pub fn new(dependent: <MutexMarker as Lock>::Lock<Dependency>) -> Self {
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
    pub fn new(_dependency: <MutexMarker as Lock>::Lock<CircularDependency>) -> Self {
        Self {}
    }
}