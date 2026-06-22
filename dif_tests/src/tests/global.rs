// Tests the global injector 

use dif::Injector;
use crate::injectables::TestLogger;

#[test]
pub fn initialize_global_injector() {
    // Arrange
    Injector::initialize(|injector| {
        injector.singleton::<TestLogger>();
    });
    
    // Act
    let item = Injector::global().get::<TestLogger>();
    
    // Assert
    assert!(item.is_some());
}

#[test]
#[should_panic]
pub fn global_injector_not_initialized() {
    // Act + Assert
    Injector::global().get::<TestLogger>();
}

#[test]
#[should_panic]
pub fn multiple_global_injectors() {
    // Arrange
    Injector::initialize(|injector| {
        injector.singleton::<TestLogger>();
    });
    
    // Act + Assert
    Injector::initialize(|injector| {
        injector.singleton::<TestLogger>();
    });
}