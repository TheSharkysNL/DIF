use std::sync::atomic::{AtomicBool, Ordering};
use dif::{Component, Injector};
use crate::injectables::{Logger, TestLogger};

#[test]
pub fn register_singleton() {
    // Arrange
    let mut injector = Injector::new();

    // Act
    injector.singleton::<TestLogger>();

    // Assert
    let get = injector.get::<TestLogger>();
    assert!(get.is_some());
}

#[test]
pub fn register_transient() {
    // Arrange
    let mut injector = Injector::new();

    // Act
    injector.transient::<TestLogger>();

    // Assert
    let get = injector.get::<TestLogger>();
    assert!(get.is_some());
}

#[test]
pub fn register_singleton_dyn() {
    // Arrange
    let mut injector = Injector::new();

    // Act
    injector.singleton_dyn::<TestLogger, dyn Logger>();

    // Assert
    let get = injector.get_dyn::<dyn Logger>();
    assert!(get.is_some());

    let get = injector.get::<TestLogger>();
    assert!(get.is_none());
}

#[test]
pub fn register_transient_dyn() {
    // Arrange
    let mut injector = Injector::new();

    // Act
    injector.transient_dyn::<TestLogger, dyn Logger>();

    // Assert
    let get = injector.get_dyn::<dyn Logger>();
    assert!(get.is_some());

    let get = injector.get::<TestLogger>();
    assert!(get.is_none());
}

#[test]
pub fn register_component() {
    // Arrange
    let mut injector = Injector::new();

    // Act
    injector.component(
        Component::singleton::<TestLogger>()
            .build()
    );

    // Assert
    let get = injector.get::<TestLogger>();
    assert!(get.is_some());
}

static CREATED: AtomicBool = AtomicBool::new(false);

#[test]
pub fn register_component_with_factory() {
    // Arrange
    let mut injector = Injector::new();
    CREATED.store(false, Ordering::SeqCst);

    // Act
    injector.component(
        Component::singleton::<TestLogger>()
            .with_factory(|_| {
                CREATED.store(true, Ordering::SeqCst);
                TestLogger {
                    
                }
            })
            .build()
    );

    // Assert
    let get = injector.get::<TestLogger>();
    assert!(get.is_some());
    assert!(CREATED.load(Ordering::SeqCst));
}

#[test]
pub fn register_component_dynamic() {
    // Arrange
    let mut injector = Injector::new();

    // Act
    injector.component(
        Component::singleton::<TestLogger>()
            .into_dynamic::<dyn Logger>()
            .build()
    );

    // Assert
    let get = injector.get_dyn::<dyn Logger>();
    assert!(get.is_some());
    
    let get = injector.get::<TestLogger>();
    assert!(get.is_none());
}

#[test]
pub fn register_component_dynamic_with_factory() {
    // Arrange
    let mut injector = Injector::new();
    CREATED.store(false, Ordering::SeqCst);
    
    // Act
    injector.component(
        Component::singleton::<TestLogger>()
            .with_factory(|_| {
                CREATED.store(true, Ordering::SeqCst);
                TestLogger {

                }
            })
            .into_dynamic::<dyn Logger>()
            .build()
    );

    // Assert
    let get = injector.get_dyn::<dyn Logger>();
    assert!(get.is_some());
    assert!(CREATED.load(Ordering::SeqCst));

    let get = injector.get::<TestLogger>();
    assert!(get.is_none());
}

#[test]
#[should_panic]
pub fn register_same_instance_multiple_times() {
    // Arrange
    let mut injector = Injector::new();
    
    // Act + Assert
    injector.singleton::<TestLogger>();
    injector.singleton::<TestLogger>();
}