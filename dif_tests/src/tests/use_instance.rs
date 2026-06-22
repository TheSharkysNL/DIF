// These tests create the logger via the injector 
// and test if the logger is actually a valid instance of a logger 
// instead of it pointing to an invalid piece of memory

use dif::Injector;
use crate::injectables::{reset, Logger, TestLogger, WRITTEN_STRING};

#[test]
pub fn use_logger_instance() {
    // Arrange
    let mut injector = Injector::new();
    
    injector.singleton::<TestLogger>();
    
    let logger = injector.get::<TestLogger>().unwrap();
    
    let mut lock = logger.lock().unwrap();
    
    let message = "My message";
    
    reset();
    
    // Act
    lock.write(message);
    
    // Assert
    assert_eq!(WRITTEN_STRING.take(), message)
}

#[test]
pub fn use_logger_instance_dynamic() {
    // Arrange
    let mut injector = Injector::new();

    injector.singleton_dyn::<TestLogger, dyn Logger>();

    let logger = injector.get_dyn::<dyn Logger>().unwrap();

    let mut lock = logger.lock().unwrap();

    let message = "My message";

    reset();

    // Act
    lock.write(message);

    // Assert
    assert_eq!(WRITTEN_STRING.take(), message)
}