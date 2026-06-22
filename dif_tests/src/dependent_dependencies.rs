use dif::sync::InjectorLock;
use dif::service;
use std::cell::Cell;

thread_local! {
    pub static DEPENDENT_INITIALIZED: Cell<bool> = Cell::new(false);
    pub static DEPENDENCY_INITIALIZED: Cell<bool> = Cell::new(false);
}

pub struct Dependent {
    #[allow(dead_code)]
    dependency: InjectorLock<Dependency>,
}

pub struct Dependency {
    
}

#[service]
impl Dependent {
    pub fn new(dependent: InjectorLock<Dependency>) -> Self {
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
        
        Self {}
    }
}

pub fn reset() {
    DEPENDENT_INITIALIZED.replace(false);
    DEPENDENCY_INITIALIZED.replace(false);
}

pub struct CircularDependency {

}

#[service]
impl CircularDependency {
    #[allow(unused_parens)]
    pub fn new(_dependency: InjectorLock<CircularDependency>) -> Self {
        Self {}
    }
}