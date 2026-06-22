// These tests check for ordering to check that ordering doesn't matter for 
// how the user adds the instances to the injector

use dif::{Component, Injector};
use crate::dependent_dependencies::{reset, CircularDependency, Dependency, Dependent, DEPENDENCY_INITIALIZED, DEPENDENT_INITIALIZED};

#[test]
pub fn original_ordering() {
    // Arrange
    let mut injector = Injector::new();

    // as Dependent is dependent on Dependency we use the original ordering here
    injector.singleton::<Dependent>();
    injector.singleton::<Dependency>();
    
    reset();
    
    // Act
    let dependency = injector.get::<Dependent>();
    
    // Assert
    assert!(dependency.is_some());

    assert!(DEPENDENT_INITIALIZED.get());
    assert!(DEPENDENCY_INITIALIZED.get());
}

#[test]
pub fn other_ordering() {
    // Arrange
    let mut injector = Injector::new();

    // To check that Dependent will still be correctly initialized even if the dependency ordering is different
    injector.singleton::<Dependency>();
    injector.singleton::<Dependent>();

    reset();

    // Act
    let dependency = injector.get::<Dependent>();

    // Assert
    assert!(dependency.is_some());

    assert!(DEPENDENT_INITIALIZED.get());
    assert!(DEPENDENCY_INITIALIZED.get());
}

#[test]
pub fn factory_function() {
    // Arrange
    let mut injector = Injector::new();

    injector.singleton::<Dependency>();
    injector.component(Component::singleton()
        .with_factory(|injector| {
            Dependent::new(injector.get().unwrap())
        })
        .build()
    );

    reset();

    // Act
    let dependency = injector.get::<Dependent>();

    // Assert
    assert!(dependency.is_some());

    assert!(DEPENDENT_INITIALIZED.get());
    assert!(DEPENDENCY_INITIALIZED.get());
}

#[test]
pub fn factory_function_other_ordering() {
    // Arrange
    let mut injector = Injector::new();

    injector.component(Component::singleton()
        .with_factory(|injector| {
            Dependent::new(injector.get().unwrap())
        })
        .build()
    );
    injector.singleton::<Dependency>();

    reset();

    // Act
    let dependency = injector.get::<Dependent>();

    // Assert
    assert!(dependency.is_some());

    assert!(DEPENDENT_INITIALIZED.get());
    assert!(DEPENDENCY_INITIALIZED.get());
}

// should only be run in debug mode, as when it is run in release it will cause a stack error
#[cfg(debug_assertions)]
#[test]
#[should_panic]
pub fn circular_dependency() {
    // Arrange
    let mut injector = Injector::new();
    
    injector.singleton::<CircularDependency>();
    
    // Act + Assert
    injector.get::<CircularDependency>(); // circular dependency
}