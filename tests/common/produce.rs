use dilian::Injector;
use dilian_core::sync::MutexMarker;
use crate::dependent_dependencies::{Dependency, Dependent, DEPENDENCY_INITIALIZED, DEPENDENT_INITIALIZED};

#[test]
#[should_panic]
pub fn produce_dependency_not_found() {
    // Arrange
    let injector = Injector::<MutexMarker>::new();
    
    // Act + Assert
    injector.produce::<Dependent>();
}

#[test]
pub fn produce_singleton() {
    // Arrange
    let mut injector = Injector::<MutexMarker>::new();
    
    injector.singleton::<Dependency>();
    
    // Act
    let _ = injector.produce::<Dependent>();
    
    // Assert
    assert!(DEPENDENCY_INITIALIZED.get());
    assert!(DEPENDENT_INITIALIZED.get());
}

#[test]
pub fn produce_transient() {
    // Arrange
    let mut injector = Injector::<MutexMarker>::new();

    injector.transient::<Dependency>();

    // Act
    let _ = injector.produce::<Dependent>();

    // Assert
    assert!(DEPENDENCY_INITIALIZED.get());
    assert!(DEPENDENT_INITIALIZED.get());
}