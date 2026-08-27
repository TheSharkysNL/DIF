use generic_tests::define;

#[define]
mod tests {
    use std::any::TypeId;
    use dil::sync::{LockBound, Lockable};
    use dil::Injector;
    use dil::sync::{MutexMarker, RwLockMarker, RefCellMarker};
    use crate::injectables::{reset, INITIALIZE_COUNT, DROP_COUNT, TestLogger, Logger, AnotherLogger, OTHER_INITIALIZE_COUNT, OTHER_DROP_COUNT, WRITTEN_STRING, AnotherService};
    use dil::sync::Lock;
    use std::ops::Deref;

    #[test]
    pub fn get_empty<L : Lock  + LockBound<TestLogger> + LockBound<dyn Logger> + LockBound<AnotherLogger> + LockBound<AnotherService>>() {
        // Arrange
        let injector = Injector::<L>::new();

        // Act
        let get = injector.get::<TestLogger>();

        // Assert
        assert!(get.is_none());
    }

    #[test]
    pub fn get_empty_dynamic<L : Lock  + LockBound<TestLogger> + LockBound<dyn Logger> + LockBound<AnotherLogger> + LockBound<AnotherService>>() {
        // Arrange
        let injector = Injector::<L>::new();

        // Act
        let get = injector.get::<dyn Logger>();

        // Assert
        assert!(get.is_none());
    }


    #[test]
    pub fn get_empty_list<L : Lock  + LockBound<TestLogger> + LockBound<dyn Logger> + LockBound<AnotherLogger> + LockBound<AnotherService>>() {
        // Arrange
        let injector = Injector::<L>::new();

        // Act
        let get = injector.get_list::<dyn Logger>();

        // Assert
        assert_eq!(get.count(), 0);
    }

    #[test]
    pub fn get_singleton_multiple_times<L : Lock  + LockBound<TestLogger> + LockBound<dyn Logger> + LockBound<AnotherLogger> + LockBound<AnotherService>>() {
        {
            // Arrange
            let mut injector = Injector::<L>::new();

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
    pub fn get_transient_multiple_times<L : Lock + LockBound<TestLogger> + LockBound<dyn Logger> + LockBound<AnotherLogger> + LockBound<AnotherService>>() {
        {
            // Arrange
            let mut injector = Injector::<L>::new();

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
    pub fn get_singleton_multiple_times_dynamic<L : Lock  + LockBound<TestLogger> + LockBound<dyn Logger> + LockBound<AnotherLogger> + LockBound<AnotherService>>() {
        {
            // Arrange
            let mut injector = Injector::<L>::new();

            injector.singleton_dyn::<TestLogger, dyn Logger>();

            reset();

            {
                // Act
                let get = injector.get::<dyn Logger>();
                let get2 = injector.get::<dyn Logger>();

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
    pub fn get_transient_multiple_times_dynamic<L : Lock  + LockBound<TestLogger> + LockBound<dyn Logger> + LockBound<AnotherLogger> + LockBound<AnotherService>>() {
        // Arrange
        let mut injector = Injector::<L>::new();

        injector.transient_dyn::<TestLogger, dyn Logger>();

        reset();
        {
            // Act
            let get = injector.get::<dyn Logger>();
            let get2 = injector.get::<dyn Logger>();

            // Assert
            assert!(get.is_some());
            assert!(get2.is_some());

            assert_eq!(DROP_COUNT.get(), 0, "Transient should only be dropped after the instances of get and get2 are dropped.");
        }

        assert_eq!(INITIALIZE_COUNT.get(), 2, "Transient should always be reinitialized for every get.");
        assert_eq!(DROP_COUNT.get(), 2, "Should have been dropped twice as it was created twice.");
    }

    #[test]
    pub fn get_singleton_single_list<L : Lock  + LockBound<TestLogger> + LockBound<dyn Logger> + LockBound<AnotherLogger> + LockBound<AnotherService>>() {
        {
            // Arrange
            let mut injector = Injector::<L>::new();

            injector.singleton_dyn::<TestLogger, dyn Logger>();

            reset();

            {
                // Act
                let list = injector.get_list::<dyn Logger>();

                // Assert
                assert_eq!(INITIALIZE_COUNT.get(), 0, "Should only be initialized once the iterator is iterated through.");

                let count = list.count();

                assert_eq!(count, 1, "Should have one item");
            }

            assert_eq!(INITIALIZE_COUNT.get(), 1);

            assert_eq!(DROP_COUNT.get(), 0, "Instance should only be dropped after the Injector is dropped.");
        }
        assert_eq!(DROP_COUNT.get(), 1, "Instance should only be dropped after the Injector is dropped. After the injector is dropped.");
    }

    #[test]
    pub fn get_transient_single_list<L : Lock  + LockBound<TestLogger> + LockBound<dyn Logger> + LockBound<AnotherLogger> + LockBound<AnotherService>>() {
        // Arrange
        let mut injector = Injector::<L>::new();

        injector.transient_dyn::<TestLogger, dyn Logger>();

        reset();

        {
            // Act
            let list = injector.get_list::<dyn Logger>();

            // Assert
            assert_eq!(DROP_COUNT.get(), 0, "Instance should be dropped after the list is dropped.");
            
            assert_eq!(INITIALIZE_COUNT.get(), 0, "Should only be initialized once the iterator is iterated through.");

            let count = list.count();

            assert_eq!(count, 1, "Should have one item");
        }

        assert_eq!(INITIALIZE_COUNT.get(), 1);

        assert_eq!(DROP_COUNT.get(), 1, "Instance should be dropped after the list is dropped.");
    }

    #[test]
    pub fn get_singleton_list<L : Lock  + LockBound<TestLogger> + LockBound<dyn Logger> + LockBound<AnotherLogger> + LockBound<AnotherService>>() {
        {
            // Arrange
            let mut injector = Injector::<L>::new();

            injector.singleton_dyn::<TestLogger, dyn Logger>();
            injector.singleton_dyn::<AnotherLogger, dyn Logger>();

            reset();

            {
                // Act
                let list = injector.get_list::<dyn Logger>();

                // Assert
                assert_eq!(INITIALIZE_COUNT.get(), 0, "Should only be initialized once the iterator is iterated through.");
                assert_eq!(OTHER_INITIALIZE_COUNT.get(), 0, "Should only be initialized once the iterator is iterated through.");

                let count = list.count();

                assert_eq!(count, 2, "Should have two items as two were added");
            }

            assert_eq!(INITIALIZE_COUNT.get(), 1);
            assert_eq!(OTHER_INITIALIZE_COUNT.get(), 1);

            assert_eq!(DROP_COUNT.get(), 0, "Instance should only be dropped after the Injector is dropped.");
            assert_eq!(OTHER_DROP_COUNT.get(), 0, "Instance should only be dropped after the Injector is dropped.");
        }
        assert_eq!(DROP_COUNT.get(), 1, "Instance should only be dropped after the Injector is dropped.");
        assert_eq!(OTHER_DROP_COUNT.get(), 1, "Instance should only be dropped after the Injector is dropped.");
    }

    #[test]
    pub fn get_transient_list<L : Lock  + LockBound<TestLogger> + LockBound<dyn Logger> + LockBound<AnotherLogger> + LockBound<AnotherService>>() {
        // Arrange
        let mut injector = Injector::<L>::new();

        injector.transient_dyn::<TestLogger, dyn Logger>();
        injector.transient_dyn::<AnotherLogger, dyn Logger>();

        reset();

        {
            // Act
            let list = injector.get_list::<dyn Logger>();

            // Assert
            assert_eq!(DROP_COUNT.get(), 0, "Instance should be dropped after the list is dropped.");
            assert_eq!(OTHER_DROP_COUNT.get(), 0, "Instance should be dropped after the list is dropped.");

            assert_eq!(INITIALIZE_COUNT.get(), 0, "Should only be initialized once the iterator is iterated through.");
            assert_eq!(OTHER_INITIALIZE_COUNT.get(), 0, "Should only be initialized once the iterator is iterated through.");

            let count = list.count();

            assert_eq!(count, 2, "Should have two items as two were added");
        }

        assert_eq!(INITIALIZE_COUNT.get(), 1);
        assert_eq!(OTHER_INITIALIZE_COUNT.get(), 1);

        assert_eq!(DROP_COUNT.get(), 1, "Instance should be dropped after the list is dropped.");
        assert_eq!(OTHER_DROP_COUNT.get(), 1, "Instance should be dropped after the list is dropped.");
    }

    #[test]
    pub fn get_singleton_multiple_list<L : Lock  + LockBound<TestLogger> + LockBound<dyn Logger> + LockBound<AnotherLogger> + LockBound<AnotherService>>() {
        {
            // Arrange
            let mut injector = Injector::<L>::new();

            injector.singleton_dyn::<TestLogger, dyn Logger>();
            injector.singleton_dyn::<AnotherLogger, dyn Logger>();

            reset();

            {
                // Act
                let list = injector.get_list::<dyn Logger>();
                let list2 = injector.get_list::<dyn Logger>();

                // Assert
                assert_eq!(INITIALIZE_COUNT.get(), 0, "Should only be initialized once the iterator is iterated through.");
                assert_eq!(OTHER_INITIALIZE_COUNT.get(), 0, "Should only be initialized once the iterator is iterated through.");

                let count = list.count();

                let count2 = list2.count();

                assert_eq!(count, 2, "Should have two items as two were added");
                assert_eq!(count2, 2, "Should have two items as two were added");
            }

            assert_eq!(INITIALIZE_COUNT.get(), 1, "Singleton should only be initialized once.");
            assert_eq!(OTHER_INITIALIZE_COUNT.get(), 1, "Singleton should only be initialized once.");

            assert_eq!(DROP_COUNT.get(), 0, "Instance should only be dropped after the Injector is dropped.");
            assert_eq!(OTHER_DROP_COUNT.get(), 0, "Instance should only be dropped after the Injector is dropped.");
        }
        assert_eq!(DROP_COUNT.get(), 1, "Instance should only be dropped after the Injector is dropped.");
        assert_eq!(OTHER_DROP_COUNT.get(), 1, "Instance should only be dropped after the Injector is dropped.");
    }

    #[test]
    pub fn get_transient_multiple_list<L : Lock  + LockBound<TestLogger> + LockBound<dyn Logger> + LockBound<AnotherLogger> + LockBound<AnotherService>>() {
        // Arrange
        let mut injector = Injector::<L>::new();

        injector.transient_dyn::<TestLogger, dyn Logger>();
        injector.transient_dyn::<AnotherLogger, dyn Logger>();

        reset();

        {
            // Act
            let list = injector.get_list::<dyn Logger>();
            let list2 = injector.get_list::<dyn Logger>();

            // Assert
            assert_eq!(INITIALIZE_COUNT.get(), 0, "Should only be initialized once the iterator is iterated through.");
            assert_eq!(OTHER_INITIALIZE_COUNT.get(), 0, "Should only be initialized once the iterator is iterated through.");

            let count = list.count();

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
    pub fn get_transient_and_singleton_multiple_list<L : Lock  + LockBound<TestLogger> + LockBound<dyn Logger> + LockBound<AnotherLogger> + LockBound<AnotherService>>() {
        {
            // Arrange
            let mut injector = Injector::<L>::new();

            injector.singleton_dyn::<TestLogger, dyn Logger>();
            injector.transient_dyn::<AnotherLogger, dyn Logger>();

            reset();

            {
                // Act
                let list = injector.get_list::<dyn Logger>();
                let list2 = injector.get_list::<dyn Logger>();

                // Assert

                assert_eq!(INITIALIZE_COUNT.get(), 0, "Should only be initialized once the iterator is iterated through.");
                assert_eq!(OTHER_INITIALIZE_COUNT.get(), 0, "Should only be initialized once the iterator is iterated through.");

                let count = list.count();

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

    #[test]
    pub fn get_by_id_test_logger<L : Lock  + LockBound<TestLogger> + LockBound<dyn Logger> + LockBound<AnotherLogger> + LockBound<AnotherService>>() 
        where <L as Lock>::Lock<dyn Logger> : Lockable<dyn Logger>
    {
        // Arrange
        let mut injector = Injector::<L>::new();

        injector.singleton_dyn::<TestLogger, dyn Logger>();
        injector.transient_dyn::<AnotherLogger, dyn Logger>();

        reset();

        // Act
        let logger = injector.get_by_id::<dyn Logger>(TypeId::of::<TestLogger>());

        // Assert
        assert!(logger.is_some());
        assert_eq!(INITIALIZE_COUNT.get(), 1, "Should have gotten the TestLogger.");
        assert_eq!(OTHER_INITIALIZE_COUNT.get(), 0, "Should have gotten the TestLogger.");

        let logger = logger.unwrap();
        let mut logger = logger.write();

        logger.write("Test");

        assert_eq!(WRITTEN_STRING.take(), "Test");
    }

    #[test]
    pub fn get_by_id_another_logger<L : Lock  + LockBound<TestLogger> + LockBound<dyn Logger> + LockBound<AnotherLogger> + LockBound<AnotherService>>()
        where <L as Lock>::Lock<dyn Logger> : Lockable<dyn Logger>
    {
        // Arrange
        let mut injector = Injector::<L>::new();

        injector.singleton_dyn::<TestLogger, dyn Logger>();
        injector.transient_dyn::<AnotherLogger, dyn Logger>();

        reset();

        // Act
        let logger = injector.get_by_id::<dyn Logger>(TypeId::of::<AnotherLogger>());

        // Assert
        assert!(logger.is_some());
        assert_eq!(INITIALIZE_COUNT.get(), 0, "Should have gotten the AnotherLogger.");
        assert_eq!(OTHER_INITIALIZE_COUNT.get(), 1, "Should have gotten the AnotherLogger.");

        let logger = logger.unwrap();
        let mut logger = logger.write();

        logger.write("Test");

        assert_eq!(WRITTEN_STRING.take(), "");
    }

    #[test]
    pub fn get_by_id_single_logger<L : Lock  + LockBound<TestLogger> + LockBound<dyn Logger> + LockBound<AnotherLogger> + LockBound<AnotherService>>()
        where <L as Lock>::Lock<dyn Logger> : Lockable<dyn Logger>
    {
        // Arrange
        let mut injector = Injector::<L>::new();

        injector.singleton_dyn::<TestLogger, dyn Logger>();

        reset();

        // Act
        let logger = injector.get_by_id::<dyn Logger>(TypeId::of::<TestLogger>());

        // Assert
        assert!(logger.is_some());
        assert_eq!(INITIALIZE_COUNT.get(), 1, "Should have gotten the TestLogger.");
        assert_eq!(OTHER_INITIALIZE_COUNT.get(), 0, "Should have gotten the TestLogger.");

        let logger = logger.unwrap();
        let mut logger = logger.write();

        logger.write("Test");

        assert_eq!(WRITTEN_STRING.take(), "Test");
    }

    #[test]
    pub fn get_by_id_not_found<L : Lock  + LockBound<TestLogger> + LockBound<dyn Logger> + LockBound<AnotherLogger> + LockBound<AnotherService>>() {
        // Arrange
        let mut injector = Injector::<L>::new();

        injector.singleton_dyn::<TestLogger, dyn Logger>();

        reset();

        // Act
        let logger = injector.get_by_id::<dyn Logger>(TypeId::of::<AnotherLogger>());

        // Assert
        assert!(logger.is_none());
    }

    #[test]
    pub fn get_any_singleton<L : Lock  + LockBound<TestLogger> + LockBound<dyn Logger> + LockBound<AnotherLogger> + LockBound<AnotherService>>()
        where <L as Lock>::Lock<TestLogger> : Lockable<TestLogger>
    {
        // Arrange
        let mut injector = Injector::<L>::new();

        injector.singleton_dyn::<TestLogger, dyn Logger>();

        reset();

        // Act
        let logger = injector.get_any(TypeId::of::<TestLogger>());

        // Assert
        assert!(logger.is_some());

        let logger = logger.unwrap();
        assert!(logger.is::<TestLogger>());
        
        let logger = logger.get::<TestLogger>();
        assert!(logger.is_some());
        
        logger.unwrap()
            .write()
            .write("any_test");
        
        assert_eq!(WRITTEN_STRING.take(), "any_test");
    }
    
    #[test]
    pub fn get_any_transient<L : Lock  + LockBound<TestLogger> + LockBound<dyn Logger> + LockBound<AnotherLogger> + LockBound<AnotherService>>() 
        where <L as Lock>::Lock<TestLogger> : Lockable<TestLogger>
    {
        // Arrange
        let mut injector = Injector::<L>::new();

        injector.transient_dyn::<TestLogger, dyn Logger>();

        reset();

        // Act
        let logger = injector.get_any(TypeId::of::<TestLogger>());

        // Assert
        assert!(logger.is_some());

        let logger = logger.unwrap();
        assert!(logger.is::<TestLogger>());

        let logger = logger.get::<TestLogger>();
        assert!(logger.is_some());

        logger.unwrap()
            .write()
            .write("any_test");

        assert_eq!(WRITTEN_STRING.take(), "any_test");
    }

    #[test]
    pub fn get_any_invalid_singleton<L : Lock  + LockBound<TestLogger> + LockBound<dyn Logger> + LockBound<AnotherLogger> + LockBound<AnotherService>>() {
        // Arrange
        let mut injector = Injector::<L>::new();

        injector.singleton_dyn::<AnotherLogger, dyn Logger>();

        reset();

        // Act
        let logger = injector.get_any(TypeId::of::<TestLogger>());

        // Assert
        assert!(logger.is_none());
    }

    #[test]
    pub fn get_any_invalid_transient<L : Lock  + LockBound<TestLogger> + LockBound<dyn Logger> + LockBound<AnotherLogger> + LockBound<AnotherService>>() {
        // Arrange
        let mut injector = Injector::<L>::new();

        injector.transient_dyn::<AnotherLogger, dyn Logger>();

        reset();

        // Act
        let logger = injector.get_any(TypeId::of::<TestLogger>());

        // Assert
        assert!(logger.is_none());
    }

    #[test]
    pub fn get_invalid_get_singleton<L : Lock  + LockBound<TestLogger> + LockBound<dyn Logger> + LockBound<AnotherLogger> + LockBound<AnotherService>>() {
        // Arrange
        let mut injector = Injector::<L>::new();

        injector.singleton_dyn::<TestLogger, dyn Logger>();

        reset();

        // Act
        let logger = injector.get_any(TypeId::of::<TestLogger>());

        // Assert
        assert!(logger.is_some());

        let logger = logger.unwrap();
        assert!(logger.is::<TestLogger>());

        let logger = logger.get::<AnotherLogger>();
        assert!(logger.is_none());
    }

    #[test]
    pub fn get_invalid_get_transient<L : Lock  + LockBound<TestLogger> + LockBound<dyn Logger> + LockBound<AnotherLogger> + LockBound<AnotherService>>() {
        // Arrange
        let mut injector = Injector::<L>::new();

        injector.transient_dyn::<TestLogger, dyn Logger>();

        reset();

        // Act
        let logger = injector.get_any(TypeId::of::<TestLogger>());

        // Assert
        assert!(logger.is_some());

        let logger = logger.unwrap();
        assert!(logger.is::<TestLogger>());
        
        let logger = logger.get::<AnotherLogger>();
        assert!(logger.is_none());
    }

    #[test]
    pub fn first_with_multiple_dyns_singleton<L : Lock  + LockBound<TestLogger> + LockBound<dyn Logger> + LockBound<AnotherLogger> + LockBound<AnotherService> + 'static>()
        where <L as Lock>::Lock<dyn Logger> : Lockable<dyn Logger>,
    {
        // Arrange
        let mut injector = Injector::<L>::new();

        injector.singleton_dyn::<TestLogger, dyn Logger>();
        injector.singleton_dyn::<AnotherLogger, dyn Logger>();

        reset();

        // Act
        let logger = injector.get::<dyn Logger>();

        // Assert
        assert!(logger.is_some());

        let logger = logger.unwrap();
        let logger = logger.read();
        
        assert_eq!(Logger::type_id(logger.deref()), TypeId::of::<TestLogger>());
    }

    #[test]
    pub fn first_with_multiple_dyns_transient<L : Lock  + LockBound<TestLogger> + LockBound<dyn Logger> + LockBound<AnotherLogger> + LockBound<AnotherService> + 'static>()
        where <L as Lock>::Lock<dyn Logger> : Lockable<dyn Logger>,
    {
        // Arrange
        let mut injector = Injector::<L>::new();

        injector.transient_dyn::<TestLogger, dyn Logger>();
        injector.transient_dyn::<AnotherLogger, dyn Logger>();

        reset();

        // Act
        let logger = injector.get::<dyn Logger>();

        // Assert
        assert!(logger.is_some());

        let logger = logger.unwrap();
        let logger = logger.read();

        assert_eq!(Logger::type_id(logger.deref()), TypeId::of::<TestLogger>());
    }

    #[instantiate_tests(<MutexMarker>)]
    mod mutex_tests {}

    #[instantiate_tests(<RwLockMarker>)]
    mod rwlock_tests {}

    #[instantiate_tests(<RefCellMarker>)]
    mod refcell_tests {}
}