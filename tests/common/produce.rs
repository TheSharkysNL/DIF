use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use dilian::Injector;
use dilian_core::{Component, FromInjector};
use dilian_core::sync::MutexMarker;
use crate::dependent_dependencies::{Dependency, Dependent, DEPENDENCY_INITIALIZED, DEPENDENT_INITIALIZED};

#[test]
pub fn produce_dependency_not_found() {
    // Arrange
    let injector = Injector::<MutexMarker>::new();
    
    // Act
    let v = injector.produce::<Dependent>();
    
    // Assert
    assert!(v.is_none());
}

#[test]
pub fn produce_singleton() {
    // Arrange
    let mut injector = Injector::<MutexMarker>::new();
    
    injector.singleton::<Dependency>();
    injector.singleton::<Dependent>();
    
    // Act
    let v = injector.produce::<Dependent>();
    
    // Assert
    assert!(v.is_some());
    assert!(DEPENDENCY_INITIALIZED.get());
    assert!(DEPENDENT_INITIALIZED.get());
}

#[test]
pub fn produce_transient() {
    // Arrange
    let mut injector = Injector::<MutexMarker>::new();

    injector.transient::<Dependency>();
    injector.transient::<Dependent>();

    // Act
    let v = injector.produce::<Dependent>();

    // Assert
    assert!(v.is_some());
    assert!(DEPENDENCY_INITIALIZED.get());
    assert!(DEPENDENT_INITIALIZED.get());
}

#[test]
pub fn produce_singleton_with_factory() {
    // Arrange
    let mut injector = Injector::<MutexMarker>::new();
    
    let factory_called = Arc::new(AtomicBool::new(false));
    let factory_called_cloned = factory_called.clone();

    injector.singleton::<Dependency>();
    injector.component(Component::singleton::<Dependent>()
        .with_factory(move |i| {
            factory_called_cloned.store(true, Ordering::SeqCst);
            
            Dependent::from_injector(i)
        })
        .build_with_factory()
    );

    // Act
    let v = injector.produce::<Dependent>();

    // Assert
    assert!(v.is_some());
    assert!(DEPENDENCY_INITIALIZED.get());
    assert!(DEPENDENT_INITIALIZED.get());
    
    assert!(factory_called.load(Ordering::SeqCst));
}

#[test]
pub fn produce_transient_with_factory() {
    // Arrange
    let mut injector = Injector::<MutexMarker>::new();

    let factory_called = Arc::new(AtomicBool::new(false));
    let factory_called_cloned = factory_called.clone();

    injector.transient::<Dependency>();
    injector.component(Component::transient::<Dependent>()
        .with_factory(move |i| {
            factory_called_cloned.store(true, Ordering::SeqCst);

            Dependent::from_injector(i)
        })
        .build_with_factory()
    );

    // Act
    let v = injector.produce::<Dependent>();

    // Assert
    assert!(v.is_some());
    assert!(DEPENDENCY_INITIALIZED.get());
    assert!(DEPENDENT_INITIALIZED.get());

    assert!(factory_called.load(Ordering::SeqCst));
}