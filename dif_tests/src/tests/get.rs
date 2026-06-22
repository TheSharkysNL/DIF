use dif::Injector;
use crate::injectables::{reset, INITIALIZE_COUNT, DROP_COUNT, TestLogger, Logger, AnotherLogger, OTHER_INITIALIZE_COUNT, OTHER_DROP_COUNT};

#[test]
pub fn get_empty() {
    // Arrange
    let injector = Injector::new();
    
    // Act
    let get = injector.get::<TestLogger>();
    
    // Assert
    assert!(get.is_none());
}

#[test]
pub fn get_empty_dynamic() {
    // Arrange
    let injector = Injector::new();

    // Act
    let get = injector.get_dyn::<dyn Logger>();

    // Assert
    assert!(get.is_none());
}


#[test]
pub fn get_empty_list() {
    // Arrange
    let injector = Injector::new();

    // Act
    let get = injector.get_list::<dyn Logger>();

    // Assert
    assert!(get.is_none());
}

#[test]
pub fn get_singleton_multiple_times() {
    {   
        // Arrange
        let mut injector = Injector::new();

        injector.singleton::<TestLogger>();

        reset();

        {
            // Act
            let get = injector.get::<TestLogger>();
            let get2 = injector.get::<TestLogger>();

            // Assert
            assert!(get.is_some());
            assert!(get2.is_some());
        }
        
        assert_eq!(INITIALIZE_COUNT.get(), 1, "Singleton should only be initialized once.");
        assert_eq!(DROP_COUNT.get(), 0, "Instance should only be dropped after the Injector is dropped.");
    }
    assert_eq!(DROP_COUNT.get(), 1, "Instance should only be dropped after the Injector is dropped. After the injector is dropped.");
}

#[test]
pub fn get_transient_multiple_times() {
    {
        // Arrange
        let mut injector = Injector::new();

        injector.transient::<TestLogger>();

        reset();

        { 
            // Act
            let get = injector.get::<TestLogger>();
            let get2 = injector.get::<TestLogger>();

            // Assert
            assert!(get.is_some());
            assert!(get2.is_some());
            assert_eq!(DROP_COUNT.get(), 0, "Transient should only be dropped after the instances of get and get2 are dropped.");
        }

        assert_eq!(INITIALIZE_COUNT.get(), 2, "Transient should always be reinitialized for every get.");
        assert_eq!(DROP_COUNT.get(), 2, "Should have been dropped twice as it was created twice.");
    }
}

#[test]
pub fn get_singleton_multiple_times_dynamic() {
    {
        // Arrange
        let mut injector = Injector::new();

        injector.singleton_dyn::<TestLogger, dyn Logger>();

        reset();

        {
            // Act
            let get = injector.get_dyn::<dyn Logger>();
            let get2 = injector.get_dyn::<dyn Logger>();

            // Assert
            assert!(get.is_some());
            assert!(get2.is_some());
        }

        assert_eq!(INITIALIZE_COUNT.get(), 1, "Singleton should only be initialized once.");
        assert_eq!(DROP_COUNT.get(), 0, "Instance should only be dropped after the Injector is dropped.");
    }
    assert_eq!(DROP_COUNT.get(), 1, "Instance should only be dropped after the Injector is dropped. After the injector is dropped.");
}

#[test]
pub fn get_transient_multiple_times_dynamic() {
    // Arrange
    let mut injector = Injector::new();

    injector.transient_dyn::<TestLogger, dyn Logger>();

    reset();
    {
        // Act
        let get = injector.get_dyn::<dyn Logger>();
        let get2 = injector.get_dyn::<dyn Logger>();

        // Assert
        assert!(get.is_some());
        assert!(get2.is_some());

        assert_eq!(DROP_COUNT.get(), 0, "Transient should only be dropped after the instances of get and get2 are dropped.");
    }
    
    assert_eq!(INITIALIZE_COUNT.get(), 2, "Transient should always be reinitialized for every get.");
    assert_eq!(DROP_COUNT.get(), 2, "Should have been dropped twice as it was created twice.");
}

#[test]
pub fn get_singleton_list() {
    { 
        // Arrange
        let mut injector = Injector::new();

        injector.singleton_dyn::<TestLogger, dyn Logger>();
        injector.singleton_dyn::<AnotherLogger, dyn Logger>();
        
        reset();

        {
            // Act
            let list = injector.get_list::<dyn Logger>();

            // Assert
            assert!(list.is_some());

            assert_eq!(INITIALIZE_COUNT.get(), 0, "Should only be initialized once the iterator is iterated through.");
            assert_eq!(OTHER_INITIALIZE_COUNT.get(), 0, "Should only be initialized once the iterator is iterated through.");

            let list = list.unwrap();
            let count = list.count();

            assert_eq!(count, 2, "Should have two items as two were added");
        }
        
        assert_eq!(INITIALIZE_COUNT.get(), 1);
        assert_eq!(OTHER_INITIALIZE_COUNT.get(), 1);
        
        assert_eq!(DROP_COUNT.get(), 0, "Instance should only be dropped after the Injector is dropped.");
        assert_eq!(OTHER_DROP_COUNT.get(), 0, "Instance should only be dropped after the Injector is dropped.");
    }
    assert_eq!(DROP_COUNT.get(), 1, "Instance should only be dropped after the Injector is dropped. After the injector is dropped.");
    assert_eq!(OTHER_DROP_COUNT.get(), 1, "Instance should only be dropped after the Injector is dropped. After the injector is dropped.");
}

#[test]
pub fn get_transient_list() {
    // Arrange
    let mut injector = Injector::new();

    injector.transient_dyn::<TestLogger, dyn Logger>();
    injector.transient_dyn::<AnotherLogger, dyn Logger>();

    reset();

    {
        // Act
        let list = injector.get_list::<dyn Logger>();

        // Assert
        assert_eq!(DROP_COUNT.get(), 0, "Instance should be dropped after the list is dropped.");
        assert_eq!(OTHER_DROP_COUNT.get(), 0, "Instance should be dropped after the list is dropped.");
        
        assert!(list.is_some());

        assert_eq!(INITIALIZE_COUNT.get(), 0, "Should only be initialized once the iterator is iterated through.");
        assert_eq!(OTHER_INITIALIZE_COUNT.get(), 0, "Should only be initialized once the iterator is iterated through.");

        let list = list.unwrap();
        let count = list.count();

        assert_eq!(count, 2, "Should have two items as two were added");
    }

    assert_eq!(INITIALIZE_COUNT.get(), 1);
    assert_eq!(OTHER_INITIALIZE_COUNT.get(), 1);

    assert_eq!(DROP_COUNT.get(), 1, "Instance should be dropped after the list is dropped.");
    assert_eq!(OTHER_DROP_COUNT.get(), 1, "Instance should be dropped after the list is dropped.");
}

#[test]
pub fn get_singleton_multiple_list() {
    {
        // Arrange
        let mut injector = Injector::new();

        injector.singleton_dyn::<TestLogger, dyn Logger>();
        injector.singleton_dyn::<AnotherLogger, dyn Logger>();

        reset();

        {
            // Act
            let list = injector.get_list::<dyn Logger>();
            let list2 = injector.get_list::<dyn Logger>();

            // Assert
            assert!(list.is_some());
            assert!(list2.is_some());

            assert_eq!(INITIALIZE_COUNT.get(), 0, "Should only be initialized once the iterator is iterated through.");
            assert_eq!(OTHER_INITIALIZE_COUNT.get(), 0, "Should only be initialized once the iterator is iterated through.");

            let list = list.unwrap();
            let count = list.count();
            
            let list2 = list2.unwrap();
            let count2 = list2.count();

            assert_eq!(count, 2, "Should have two items as two were added");
            assert_eq!(count2, 2, "Should have two items as two were added");
        }

        assert_eq!(INITIALIZE_COUNT.get(), 1, "Singleton should only be initialized once.");
        assert_eq!(OTHER_INITIALIZE_COUNT.get(), 1, "Singleton should only be initialized once.");

        assert_eq!(DROP_COUNT.get(), 0, "Instance should only be dropped after the Injector is dropped.");
        assert_eq!(OTHER_DROP_COUNT.get(), 0, "Instance should only be dropped after the Injector is dropped.");
    }
    assert_eq!(DROP_COUNT.get(), 1, "Instance should only be dropped after the Injector is dropped. After the injector is dropped.");
    assert_eq!(OTHER_DROP_COUNT.get(), 1, "Instance should only be dropped after the Injector is dropped. After the injector is dropped.");
}

#[test]
pub fn get_transient_multiple_list() {
    // Arrange
    let mut injector = Injector::new();

    injector.transient_dyn::<TestLogger, dyn Logger>();
    injector.transient_dyn::<AnotherLogger, dyn Logger>();

    reset();

    {
        // Act
        let list = injector.get_list::<dyn Logger>();
        let list2 = injector.get_list::<dyn Logger>();

        // Assert
        assert!(list.is_some());
        assert!(list2.is_some());

        assert_eq!(INITIALIZE_COUNT.get(), 0, "Should only be initialized once the iterator is iterated through.");
        assert_eq!(OTHER_INITIALIZE_COUNT.get(), 0, "Should only be initialized once the iterator is iterated through.");

        let list = list.unwrap();
        let count = list.count();

        let list2 = list2.unwrap();
        let count2 = list2.count();

        assert_eq!(count, 2, "Should have two items as two were added");
        assert_eq!(count2, 2, "Should have two items as two were added");
    }

    assert_eq!(INITIALIZE_COUNT.get(), 2, "Transient should have a new instance every time.");
    assert_eq!(OTHER_INITIALIZE_COUNT.get(), 2, "Transient should have a new instance every time.");

    assert_eq!(DROP_COUNT.get(), 2, "Instance should be dropped after the list is dropped.");
    assert_eq!(OTHER_DROP_COUNT.get(), 2, "Instance should be dropped after the list is dropped.");
}

#[test]
pub fn get_transient_and_singleton_multiple_list() {
    {
        // Arrange
        let mut injector = Injector::new();

        injector.singleton_dyn::<TestLogger, dyn Logger>();
        injector.transient_dyn::<AnotherLogger, dyn Logger>();

        reset();

        {
            // Act
            let list = injector.get_list::<dyn Logger>();
            let list2 = injector.get_list::<dyn Logger>();

            // Assert
            assert!(list.is_some());
            assert!(list2.is_some());

            assert_eq!(INITIALIZE_COUNT.get(), 0, "Should only be initialized once the iterator is iterated through.");
            assert_eq!(OTHER_INITIALIZE_COUNT.get(), 0, "Should only be initialized once the iterator is iterated through.");

            let list = list.unwrap();
            let count = list.count();

            let list2 = list2.unwrap();
            let count2 = list2.count();

            assert_eq!(count, 2, "Should have two items as two were added");
            assert_eq!(count2, 2, "Should have two items as two were added");
        }

        assert_eq!(INITIALIZE_COUNT.get(), 1, "Singleton should only have one instance.");
        assert_eq!(OTHER_INITIALIZE_COUNT.get(), 2, "Transient should have a new instance every time.");

        assert_eq!(DROP_COUNT.get(), 0, "Singleton should only be dropped after the Injector is dropped.");
        assert_eq!(OTHER_DROP_COUNT.get(), 2, "Instance should be dropped after the list is dropped.");
    }

    assert_eq!(DROP_COUNT.get(), 1, "Singleton should only be dropped after the Injector is dropped.");
    assert_eq!(OTHER_DROP_COUNT.get(), 2, "Instance should be dropped after the list is dropped.");
}

