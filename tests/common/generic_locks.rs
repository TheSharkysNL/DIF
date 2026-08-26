use generic_tests::define;

#[define]
mod tests {
    use dil::Injector;
    use dil::sync::{Lock, LockBound, Lockable};
    use dil::sync::{Mutex, RwLock, RefCell};
    use crate::injectables::{LoggerUser, TestLogger};
    #[test]
    fn add_and_get<L : Lock + LockBound<TestLogger> + LockBound<LoggerUser<L>> + 'static>() 
        where <L as Lock>::Lock<LoggerUser<L>>: Lockable<LoggerUser<L>>,
              <L as Lock>::Lock<TestLogger>: Lockable<TestLogger>,
    {
        // Arrange
        let mut injector = Injector::<L>::new();
        
        injector.singleton::<TestLogger>();
        injector.singleton::<LoggerUser<_>>();
        
        // Act
        let logger_user = injector.get::<LoggerUser<L>>();
        
        // Assert
        assert!(logger_user.is_some());
        
        let logger_user = logger_user
            .unwrap();
        let logger_user = logger_user.read();
        
        logger_user.write_my_logs();
    }

    #[instantiate_tests(<Mutex>)]
    mod mutex_tests {}

    #[instantiate_tests(<RwLock>)]
    mod rwlock_tests {}

    #[instantiate_tests(<RefCell>)]
    mod refcell_tests {}
}