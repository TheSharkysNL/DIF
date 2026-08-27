use generic_tests::define;

#[define]
mod tests {
    use std::sync::atomic::{AtomicBool, Ordering};
    use dil::{Component, Injector};
    use dil::sync::{MutexMarker, RwLockMarker, RefCellMarker};
    #[cfg(feature = "async")]
    use dil::sync::{AsyncMutexMarker, AsyncRwLockMarker};
    use dil::ComponentLifetime;
    use dil::sync::{Lock, LockBound};
    use crate::injectables::{AnotherLogger, AnotherService, Logger, TestLogger};
    use std::any::TypeId;

    #[test]
    pub fn register_singleton<L : Lock + LockBound<TestLogger>>() {
        // Arrange
        let mut injector = Injector::<L>::new();

        // Act
        injector.singleton::<TestLogger>();

        // Assert
        let get = injector.get::<TestLogger>();
        assert!(get.is_some());
    }

    #[test]
    pub fn register_transient<L : Lock + LockBound<TestLogger>>() {
        // Arrange
        let mut injector = Injector::<L>::new();

        // Act
        injector.transient::<TestLogger>();

        // Assert
        let get = injector.get::<TestLogger>();
        assert!(get.is_some());
    }

    #[test]
    pub fn register_singleton_dyn<L : Lock + LockBound<TestLogger> + LockBound<dyn Logger>>() {
        // Arrange
        let mut injector = Injector::<L>::new();

        // Act
        injector.singleton_dyn::<TestLogger, dyn Logger>();

        // Assert
        let get = injector.get::<dyn Logger>();
        assert!(get.is_some());
    }

    #[test]
    pub fn register_transient_dyn<L : Lock + LockBound<TestLogger> + LockBound<dyn Logger>>() {
        // Arrange
        let mut injector = Injector::<L>::new();

        // Act
        injector.transient_dyn::<TestLogger, dyn Logger>();

        // Assert
        let get = injector.get::<dyn Logger>();
        assert!(get.is_some());
    }

    #[test]
    pub fn register_component<L : Lock + LockBound<TestLogger> + LockBound<dyn Logger>>() {
        // Arrange
        let mut injector = Injector::<L>::new();

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
    pub fn register_component_with_factory<L : Lock + LockBound<TestLogger> + LockBound<dyn Logger>>() {
        // Arrange
        let mut injector = Injector::<L>::new();
        CREATED.store(false, Ordering::SeqCst);

        // Act
        injector.component(
            Component::singleton::<TestLogger>()
                .with_factory(|_| {
                    CREATED.store(true, Ordering::SeqCst);
                    TestLogger {}
                })
                .build()
        );

        // Assert
        let get = injector.get::<TestLogger>();
        assert!(get.is_some());
        assert!(CREATED.load(Ordering::SeqCst));
    }

    #[test]
    pub fn register_component_dynamic<L : Lock + LockBound<TestLogger> + LockBound<dyn Logger>>() {
        // Arrange
        let mut injector = Injector::<L>::new();

        // Act
        injector.component(
            Component::singleton::<TestLogger>()
                .with_dynamic::<dyn Logger>()
                .build()
        );

        // Assert
        let get = injector.get::<dyn Logger>();
        assert!(get.is_some());
    }

    #[test]
    pub fn register_component_dynamic_with_factory<L : Lock + LockBound<TestLogger> + LockBound<dyn Logger>>() {
        // Arrange
        let mut injector = Injector::<L>::new();
        CREATED.store(false, Ordering::SeqCst);

        // Act
        injector.component(
            Component::singleton::<TestLogger>()
                .with_factory(|_| {
                    CREATED.store(true, Ordering::SeqCst);
                    TestLogger {}
                })
                .with_dynamic::<dyn Logger>()
                .build()
        );

        // Assert
        let get = injector.get::<dyn Logger>();
        assert!(get.is_some());
        assert!(CREATED.load(Ordering::SeqCst));
    }

    #[test]
    #[should_panic]
    pub fn register_same_instance_multiple_times<L : Lock + LockBound<TestLogger> + LockBound<dyn Logger>>() {
        // Arrange
        let mut injector = Injector::<L>::new();

        // Act + Assert
        injector.singleton::<TestLogger>();
        injector.singleton::<TestLogger>();
    }

    #[test]
    pub fn register_instance_three_times<L : Lock + LockBound<TestLogger> + LockBound<dyn Logger> + LockBound<AnotherLogger> + LockBound<AnotherService>>() {
        // Arrange
        let mut injector = Injector::<L>::new();

        // Act
        injector.singleton_dyn::<TestLogger, dyn Logger>();
        injector.singleton_dyn::<AnotherLogger, dyn Logger>();
        injector.singleton_dyn::<AnotherService, dyn Logger>();

        // Assert
        let get = injector.get_list::<dyn Logger>();
        assert_eq!(get.count(), 3);
    }

    #[test]
    pub fn component_lifetime_singleton<L : Lock + LockBound<TestLogger> + LockBound<dyn Logger>>() {
        // Arrange
        let component: Component<L> = Component::singleton::<TestLogger>()
            .build();

        // Act
        let lifetime = component.lifetime();

        // Assert
        assert_eq!(lifetime, ComponentLifetime::Singleton)
    }

    #[test]
    pub fn component_lifetime_transient<L : Lock + LockBound<TestLogger> + LockBound<dyn Logger> + LockBound<AnotherLogger> + LockBound<AnotherService>>() {
        // Arrange
        let component: Component<L> = Component::transient::<TestLogger>()
            .build();

        // Act
        let lifetime = component.lifetime();

        // Assert
        assert_eq!(lifetime, ComponentLifetime::Transient)
    }

    #[test]
    pub fn component_singleton_unique_id<L : Lock + LockBound<TestLogger> + LockBound<dyn Logger> + LockBound<AnotherLogger> + LockBound<AnotherService>>() {
        // Arrange
        let component: Component<L> = Component::singleton::<TestLogger>()
            .build();

        // Act
        let lifetime = component.unique_id();

        // Assert
        assert_eq!(lifetime, TypeId::of::<TestLogger>());
    }

    #[test]
    pub fn component_transient_unique_id<L : Lock + LockBound<TestLogger> + LockBound<dyn Logger> + LockBound<AnotherLogger> + LockBound<AnotherService>>() {
        // Arrange
        let component: Component<L> = Component::transient::<TestLogger>()
            .build();

        // Act
        let lifetime = component.unique_id();

        // Assert
        assert_eq!(lifetime, TypeId::of::<TestLogger>());
    }

    #[instantiate_tests(<MutexMarker>)]
    mod mutex_tests {}

    #[instantiate_tests(<RwLockMarker>)]
    mod rwlock_tests {}

    #[instantiate_tests(<RefCellMarker>)]
    mod refcell_tests {}

    #[cfg(feature = "async")]
    #[instantiate_tests(<AsyncMutexMarker>)]
    mod async_mutex_tests {}

    #[cfg(feature = "async")]
    #[instantiate_tests(<AsyncRwLockMarker>)]
    mod async_rwlock_tests {}
}