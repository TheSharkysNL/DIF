mod components;
mod container;
pub mod sync;
pub mod cell;

use crate::container::DIContainer;
use crate::sync::{InstanceCellLock, Lock, LockBound};
use std::any::{TypeId};

pub use components::*;
use crate::cell::AnyMetadata;
pub use crate::container::DependencyIter;

#[cfg(feature = "globals")]
static MUTEX_INJECTOR_INSTANCE: std::sync::LazyLock<std::sync::RwLock<Injector<crate::sync::MutexMarker>>> = std::sync::LazyLock::new(|| std::sync::RwLock::new(Injector::new()));

#[cfg(feature = "globals")]
static RW_INJECTOR_INSTANCE: std::sync::LazyLock<std::sync::RwLock<Injector<crate::sync::RwLockMarker>>> = std::sync::LazyLock::new(|| std::sync::RwLock::new(Injector::new()));

#[cfg(feature = "globals")]
static mut REFCELL_INJECTOR_INSTANCE: Option<Injector<crate::sync::RefCellMarker>> = None;

#[cfg(all(feature = "globals", feature = "async"))]
static ASYNC_MUTEX_INJECTOR_INSTANCE: std::sync::LazyLock<std::sync::RwLock<Injector<crate::sync::AsyncMutexMarker>>> = std::sync::LazyLock::new(|| std::sync::RwLock::new(Injector::new()));

#[cfg(all(feature = "globals", feature = "async"))]
static ASYNC_RW_INJECTOR_INSTANCE: std::sync::LazyLock<std::sync::RwLock<Injector<crate::sync::AsyncRwLockMarker>>> = std::sync::LazyLock::new(|| std::sync::RwLock::new(Injector::new()));

// /// The global injector instance.
// #[cfg(any(feature = "async", feature = "multithreaded"))]
// static INJECTOR_INSTANCE: std::sync::LazyLock<std::sync::RwLock<Injector<crate::sync::Mutex>>> = std::sync::LazyLock::new(|| std::sync::RwLock::new(Injector { container: DIContainer::default() }));
// 
// #[cfg(not(any(feature = "async", feature = "multithreaded")))]
// static mut INJECTOR_INSTANCE: Option<Injector> = None;

/// The main injector used for dependency injection.
#[derive(Default)]
pub struct Injector<L : Lock> {
    container: DIContainer<L>,
}

impl<L : Lock> Injector<L> {
    /// Creates a new, empty injector.
    pub fn new() -> Self {
        Self {
            container: Default::default(),
        }
    }
    
    /// Gets the lock containing the type `T`.
    ///
    /// Returns `None` if no component for `T` has been registered.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// // create injector
    /// let mut injector = Injector::new();
    ///
    /// // register type to the injector
    /// injector.singleton::<ConsoleLogger>();
    ///
    /// // code here...
    ///
    /// // get the instance to the type
    /// let logger = injector.get::<ConsoleLogger>()
    /// .unwrap(); // unwrap here as ConsoleLogger is known to have been registered to the injector
    /// let mut logger = logger
    ///     .lock()
    ///     .await; // get lock to the logger
    ///
    /// // use the instance
    /// logger.write("It worked!");
    /// ```
    ///
    /// This can also retrieve dynamic instances. When multiple instances are
    /// registered for a dynamic type, this returns the first one registered.
    ///
    /// For example:
    ///
    /// ```
    /// // create injector
    /// let mut injector = Injector::new();
    ///
    /// // register types to the injector
    /// injector.singleton_dyn::<ConsoleLogger, dyn Logger>();
    /// injector.singleton_dyn::<FileLogger, dyn Logger>();
    ///
    /// // retrieve dynamic instance
    /// let logger = injector.get::<dyn Logger>(); 
    /// // logger here will be the `ConsoleLogger` type as that is the first instance that was added.
    /// ```
    /// 
    /// To retrieve a specific dynamic instance, use [`Self::get_by_id`].
    pub fn get<T : ?Sized + 'static>(&self) -> Option<L::Lock<T>> {
        self.container.get(self)
    }

    /// Returns an iterator over all registered dynamic instances of `T`.
    ///
    /// # Examples
    /// 
    /// ```rust
    /// // create injector
    /// let mut injector = Injector::new();
    /// 
    /// // register types to the injector
    /// injector.singleton_dyn::<ConsoleLogger, dyn Logger>();
    /// injector.singleton_dyn::<FileLogger, dyn Logger>();
    /// 
    /// // code here...
    /// 
    /// // get the instances
    /// let loggers = injector.get_list::<dyn Logger>();
    /// 
    /// for logger in loggers { // loop through all the instances
    ///     let mut logger = logger.lock()
    ///         .await; // get lock to specific instance
    ///     
    ///     // use the instance
    ///     logger.write("It worked!");
    /// }
    /// ```
    pub fn get_list<T: ?Sized + 'static>(&self) -> DependencyIter<'_, T, L> {
        self.container.get_list(self)
    }
    
    /// Gets a dynamic type using its [`TypeId`].
    /// This can retrieve a specific implementation at runtime.
    /// 
    /// # Example
    ///
    /// ```
    /// // create injector
    /// let mut injector = Injector::new();
    ///
    /// // register types to the injector
    /// injector.singleton_dyn::<ConsoleLogger, dyn Logger>();
    /// injector.singleton_dyn::<FileLogger, dyn Logger>();
    ///
    /// // retrieve dynamic instance by id
    /// let logger = injector.get_by_id::<dyn Logger>(TypeId::of::<FileLogger>()); 
    /// // logger here will be the `FileLogger` type as that was requested.
    /// ```
    pub fn get_by_id<T: ?Sized + 'static>(&self, type_id: TypeId) -> Option<L::Lock<T>> {
        self.container.get_by_id(type_id, self)
    }
    
    /// Gets an [`Any`] value by [`TypeId`] that can be downcast.
    pub fn get_any(&self, type_id: TypeId) -> Option<InstanceCellLock<L>> {
        self.container.get_instance_cell(type_id, self)
    }
    
    /// Creates a new instance of `T` using the components in the injector.
    /// This always creates a new instance, even if `T` was registered as a
    /// singleton.
    ///
    /// This method returns ownership of `T` instead of a lock containing `T`.
    /// 
    /// # Panics
    /// 
    /// If a required component is not found in the injector.
    /// 
    /// # Examples
    /// 
    /// ```
    /// #[derive(Service)]
    /// pub struct Dependent { // Dependent type which is dependent on Dependency
    ///     dependency: InjectorLock<Dependency>,
    /// }
    /// 
    /// #[derive(Service)]
    /// pub struct Dependency; // The dependency of Dependent
    /// 
    /// // Create injector
    /// let mut injector = Injector::new();
    /// 
    /// // add dependency to the injector
    /// injector.singleton::<Dependency>();
    /// 
    /// // get injector
    /// let dependent = injector.produce::<Dependent>(); 
    /// 
    /// // Use dependent below
    /// 
    /// ```
    pub fn produce<T : FromInjector<L>>(&self) -> T {
        T::from_injector(self)
    }
    
    /// Registers a singleton component with the injector.
    ///
    /// The instance is created once and reused for every call to
    /// `injector.get::<T>()`.
    /// 
    /// # Examples
    /// 
    /// ```
    /// // create injector
    /// let mut injector = Injector::new();
    ///
    /// // register type to the injector
    /// injector.singleton::<ConsoleLogger>();
    /// ```
    pub fn singleton<T : FromInjector<L> + 'static>(&mut self) 
        where L : LockBound<T>
    {
        self.component(
            Component::singleton::<T>()
                .build()
        )
    }

    /// Registers a transient component with the injector.
    ///
    /// A new instance is created for every call to `injector.get::<T>()`.
    ///
    /// # Examples
    ///
    /// ```
    /// // create injector
    /// let mut injector = Injector::new();
    ///
    /// // register type to the injector
    /// injector.transient::<ConsoleLogger>();
    /// ```
    pub fn transient<T : FromInjector<L> + 'static>(&mut self)
        where L : LockBound<T>
    {
        self.component(
            Component::transient::<T>()
                .build()
        )
    }

    /// Registers a `dyn` singleton instance to the injector.
    ///
    /// The instance is created once and reused for every call to
    /// `injector.get::<TDyn>()` or `injector.get_list::<TDyn>()`.
    /// 
    /// # Edge cases
    /// 
    /// When multiple instances of a dynamic type are registered, use
    /// `injector.get_list::<TDyn>()` to retrieve them all. The regular
    /// `injector.get::<TDyn>()` method returns the first registered instance.
    ///
    /// Registering a dynamic type does not also register the original type
    /// `T`; register it separately when needed.
    /// 
    /// # Examples
    ///
    /// ```
    /// // create injector
    /// let mut injector = Injector::new();
    ///
    /// // register type to the injector
    /// injector.singleton_dyn::<dyn Logger>();
    /// ```
    pub fn singleton_dyn<T : DynamicInjectable<TDyn, L>, TDyn : Injectable + ?Sized + AnyMetadata<L>>(&mut self)
        where L : LockBound<T>,
              L : LockBound<TDyn>
    {
        self.component(
            Component::singleton::<T>()
                .with_dynamic::<TDyn>()
                .build()
        )
    }

    /// Registers a `dyn` transient instance to the injector.
    ///
    /// A new instance is created for every call to `injector.get::<TDyn>()` or
    /// each time the iterator returned by `injector.get_list::<TDyn>()` yields
    /// an item.
    ///
    /// # Edge cases
    ///
    /// When registering multiple instances of a dynamic type. 
    /// Getting all the instances of that dynamic type can be done with the `injector.get_list::<TDyn>()`.
    /// If the regular (`injector.get::<TDyn>()`) method is used, the first instance that was registered will be resolved.
    ///
    /// Registering a dynamic type, will not also register the original type `T`. This must be done separately.
    ///
    /// # Examples
    ///
    /// ```
    /// // create injector
    /// let mut injector = Injector::new();
    ///
    /// // register type to the injector
    /// injector.transient_dyn::<dyn Logger>();
    /// ```
    pub fn transient_dyn<T : DynamicInjectable<TDyn, L>, TDyn : Injectable + ?Sized + AnyMetadata<L>>(&mut self)
        where L : LockBound<T>,
              L : LockBound<TDyn>
    {
        self.component(
            Component::transient::<T>()
                .with_dynamic::<TDyn>()
                .build()
        )
    }
    
    /// Registers a component builder to customize how an instance is created.
    /// 
    /// # Examples
    /// 
    /// Create a default singleton instance
    /// ```rust 
    ///  // create injector
    ///  let mut injector = Injector::new();
    /// 
    ///  // register component type
    ///  injector.component(
    ///     Component::singleton::<ConsoleLogger>()
    ///         .build()
    ///  );
    /// ```
    /// 
    /// Create a transient with factory
    /// ```rust
    ///  use std::sync::atomic::AtomicUsize;
    ///  static COUNT: AtomicUsize = AtomicUsize::new(0);
    /// 
    ///  // create injector
    ///  let mut injector = Injector::new();
    ///  
    ///  // register component type   
    ///  injector.component(
    ///     Component::transient::<ConsoleLogger>()
    ///         .with_factory(|injector| { // use factory to create the ConsoleLogger instance
    ///             ConsoleLogger {
    ///                 count: COUNT.fetch_add(1, Ordering::SeqCst)
    ///             }
    ///         })
    ///         .build()
    ///     );
    /// 
    ///  injector.get::<ConsoleLogger>(); // Count should not be 1
    ///  injector.get::<ConsoleLogger>(); // Count should not be 2
    /// ```
    /// 
    /// Create dynamic type 
    /// 
    /// ```rust
    ///  // create injector
    ///  let mut injector = Injector::new();
    ///
    ///  // register component type
    ///  injector.component(
    ///     Component::singleton::<ConsoleLogger>()
    ///         .with_dynamic::<dyn Logger>() // use create_dynamic to use dyn instance
    ///         .build()
    ///  );
    /// ```
    /// 
    /// Create with multiple dynamic types
    ///
    /// ```rust
    ///  // create injector
    ///  let mut injector = Injector::new();
    ///
    ///  // register component type
    ///  injector.component(
    ///     Component::singleton::<ConsoleLogger>()
    ///         .with_dynamic::<dyn Logger>() // use create_dynamic to use dyn instance
    ///         .with_dynamic::<dyn OtherDyn>()
    ///         .build()
    ///  ); // this will use one singular underlying ConsoleLogger type.
    /// ```
    /// 
    pub fn component<C : ComponentLifetimeChecker<L> + Clone + 'static>(&mut self, component: Component<L, C>) {
        self.container.register(component)
    }
}

impl Injector<crate::sync::MutexMarker> {
    /// Gets a read guard for the global [`Injector`], which uses
    /// [`std::sync::Arc<Mutex<T>>`] for registered values.
    ///
    /// Use [`Self::global_mutex_mut`] to register additional services.
    ///
    /// # Example
    ///
    /// ```rust
    /// // get mutable injector for adding services.
    /// let injector = Injector::global_mutex_mut();
    ///
    /// // add singleton
    /// injector.singleton::<ConsoleLogger>();
    ///
    /// // code here...
    ///
    /// // retrieve service here
    /// let injector = Injector::global_mutex();
    /// let logger = injector.get::<ConsoleLogger>();
    ///
    /// // use service here...
    /// ```
    #[cfg(feature = "globals")]
    pub fn global_mutex() -> std::sync::RwLockReadGuard<'static, Injector<crate::sync::MutexMarker>> {
        MUTEX_INJECTOR_INSTANCE.read()
            .unwrap()
    }

    /// Gets a write guard for the global [`Injector`], which uses
    /// [`std::sync::Arc<Mutex<T>>`] for registered values.
    ///
    /// Use this guard to retrieve services or register new ones.
    ///
    /// To get a non-mutable reference use [`Self::global_mutex`].
    ///
    /// # Example
    ///
    /// ```rust
    /// // get mutable injector for adding services.
    /// let injector = Injector::global_mutex_mut();
    ///
    /// // add singleton
    /// injector.singleton::<ConsoleLogger>();
    ///
    /// // code here...
    ///
    /// // retrieve service here
    /// let injector = Injector::global_mutex();
    /// let logger = injector.get::<ConsoleLogger>();
    ///
    /// // use service here...
    /// ```
    #[cfg(feature = "globals")]
    pub fn global_mutex_mut() -> std::sync::RwLockWriteGuard<'static, Injector<crate::sync::MutexMarker>> {
        MUTEX_INJECTOR_INSTANCE.write()
            .unwrap()
    }

    /// Gets a read guard for the global [`Injector`], which uses
    /// [`std::sync::Arc<RwLock<T>>`] for registered values.
    ///
    /// To add new services use [`Self::global_rw_mut`].
    ///
    /// # Example
    ///
    /// ```rust
    /// // get mutable injector for adding services.
    /// let injector = Injector::global_rw_mut();
    ///
    /// // add singleton
    /// injector.singleton::<ConsoleLogger>();
    ///
    /// // code here...
    ///
    /// // retrieve service here
    /// let injector = Injector::global_rw();
    /// let logger = injector.get::<ConsoleLogger>();
    ///
    /// // use service here...
    /// ```
    #[cfg(feature = "globals")]
    pub fn global_rw() -> std::sync::RwLockReadGuard<'static, Injector<crate::sync::RwLockMarker>> {
        RW_INJECTOR_INSTANCE.read()
            .unwrap()
    }

    /// Gets a write guard for the global [`Injector`], which uses
    /// [`std::sync::Arc<RwLock<T>>`] for registered values.
    ///
    /// Use this guard to retrieve services or register new ones.
    ///
    /// To get a non-mutable reference use [`Self::global_rw`].
    ///
    /// # Example
    ///
    /// ```rust
    /// // get mutable injector for adding services.
    /// let injector = Injector::global_rw_mut();
    ///
    /// // add singleton
    /// injector.singleton::<ConsoleLogger>();
    ///
    /// // code here...
    ///
    /// // retrieve service here
    /// let injector = Injector::global_rw();
    /// let logger = injector.get::<ConsoleLogger>();
    ///
    /// // use service here...
    /// ```
    #[cfg(feature = "globals")]
    pub fn global_rw_mut() -> std::sync::RwLockWriteGuard<'static, Injector<crate::sync::RwLockMarker>> {
        RW_INJECTOR_INSTANCE.write()
            .unwrap()
    }

    /// Initializes the global [`Injector`] backed by
    /// [`std::rc::Rc<RefCell<T>>`].
    ///
    /// The `init_func` receives a mutable injector for registering services.
    /// Initialization may be performed only once; afterward, use
    /// [`Self::global_ref_cell`] to retrieve services.
    ///
    /// # Panics
    ///
    /// Panics if the global injector has already been initialized.
    ///
    /// # Example
    ///
    /// ```rust
    /// // initialize injector
    /// Injector::initialize_ref_cell(|injector| {
    ///     // add ConsoleLogger singleton
    ///     injector.singleton::<ConsoleLogger>();
    /// });
    ///
    /// // retrieve injector and get the logger.
    /// let injector = Injector::global_ref_cell();
    /// let logger = injector.get::<ConsoleLogger>();
    /// ```
    #[cfg(feature = "globals")]
    #[allow(static_mut_refs)]
    pub fn initialize_ref_cell<F : FnOnce(&mut Injector<crate::sync::RefCellMarker>)>(init_func: F) {
        // Safety: Initialization is allowed only once. After initialization,
        // this static is accessed only through shared references, so its value
        // cannot be mutated while a returned reference is in use.
        unsafe {
            match &mut REFCELL_INJECTOR_INSTANCE {
                Some(_) => panic!("The injector cannot be initialized more than once."),
                None => {
                    let mut new_injector = Injector::new();
                    init_func(&mut new_injector);

                    REFCELL_INJECTOR_INSTANCE = Some(new_injector);
                }
            }
        }
    }

    /// Gets the global [`Injector`] backed by [`std::rc::Rc<RefCell<T>>`].
    ///
    /// # Panics
    ///
    /// Panics if [`Self::initialize_ref_cell`] has not been called first.
    ///
    /// # Example
    ///
    /// ```rust
    /// // initialize injector
    /// Injector::initialize_ref_cell(|injector| {
    ///     // add ConsoleLogger singleton
    ///     injector.singleton::<ConsoleLogger>();
    /// });
    ///
    /// // retrieve injector and get the logger.
    /// let injector = Injector::global_ref_cell();
    /// let logger = injector.get::<ConsoleLogger>();
    /// ```
    #[cfg(feature = "globals")]
    #[allow(static_mut_refs)]
    pub fn global_ref_cell() -> &'static Injector<crate::sync::RefCellMarker> {
        // Safety: Initialization is allowed only once. After initialization,
        // this static is accessed only through shared references, so its value
        // cannot be mutated while the returned reference is in use.
        unsafe {
            match &REFCELL_INJECTOR_INSTANCE {
                Some(injector) => injector,
                None => panic!("The injector must first be initialized. Use the initialize_ref_cell function first."),
            }
        }
    }

    /// Gets a read guard for the global [`Injector`], which uses
    /// [`std::sync::Arc<tokio::sync::Mutex<T>>`] for registered values.
    /// This injector is intended for asynchronous applications.
    ///
    /// To add new services use [`Self::global_async_mutex_mut`].
    ///
    /// # Example
    ///
    /// ```rust
    /// // get mutable injector for adding services.
    /// let injector = Injector::global_async_mutex_mut();
    ///
    /// // add singleton
    /// injector.singleton::<ConsoleLogger>();
    ///
    /// // code here...
    ///
    /// // retrieve service here
    /// let injector = Injector::global_async_mutex();
    /// let logger = injector.get::<ConsoleLogger>();
    ///
    /// // use service here...
    /// ```
    #[cfg(all(feature = "globals", feature = "async"))]
    pub fn global_async_mutex() -> std::sync::RwLockReadGuard<'static, Injector<crate::sync::AsyncMutexMarker>> {
        ASYNC_MUTEX_INJECTOR_INSTANCE.read()
            .unwrap()
    }

    /// Gets a write guard for the global [`Injector`], which uses
    /// [`std::sync::Arc<tokio::sync::Mutex<T>>`] for registered values.
    /// This injector is intended for asynchronous applications.
    ///
    /// To get a non-mutable reference use [`Self::global_async_mutex`].
    ///
    /// # Example
    ///
    /// ```rust
    /// // get mutable injector for adding services.
    /// let injector = Injector::global_async_mutex_mut();
    ///
    /// // add singleton
    /// injector.singleton::<ConsoleLogger>();
    ///
    /// // code here...
    ///
    /// // retrieve service here
    /// let injector = Injector::global_async_mutex();
    /// let logger = injector.get::<ConsoleLogger>();
    ///
    /// // use service here...
    /// ```
    #[cfg(all(feature = "globals", feature = "async"))]
    pub fn global_async_mutex_mut() -> std::sync::RwLockWriteGuard<'static, Injector<crate::sync::AsyncMutexMarker>> {
        ASYNC_MUTEX_INJECTOR_INSTANCE.write()
            .unwrap()
    }

    /// Gets a read guard for the global [`Injector`], which uses
    /// [`std::sync::Arc<tokio::sync::RwLock<T>>`] for registered values.
    /// This injector is intended for asynchronous applications.
    ///
    /// To add new services use [`Self::global_async_rw_mut`].
    ///
    /// # Example
    ///
    /// ```rust
    /// // get mutable injector for adding services.
    /// let injector = Injector::global_async_rw_mut();
    ///
    /// // add singleton
    /// injector.singleton::<ConsoleLogger>();
    ///
    /// // code here...
    ///
    /// // retrieve service here
    /// let injector = Injector::global_async_rw();
    /// let logger = injector.get::<ConsoleLogger>();
    ///
    /// // use service here...
    /// ```
    #[cfg(all(feature = "globals", feature = "async"))]
    pub fn global_async_rw() -> std::sync::RwLockReadGuard<'static, Injector<crate::sync::AsyncRwLockMarker>> {
        ASYNC_RW_INJECTOR_INSTANCE.read()
            .unwrap()
    }

    /// Gets a write guard for the global [`Injector`], which uses
    /// [`std::sync::Arc<tokio::sync::RwLock<T>>`] for registered values.
    /// This injector is intended for asynchronous applications.
    ///
    /// To get a non-mutable reference use [`Self::global_async_rw`].
    ///
    /// # Example
    ///
    /// ```rust
    /// // get mutable injector for adding services.
    /// let injector = Injector::global_async_rw_mut();
    ///
    /// // add singleton
    /// injector.singleton::<ConsoleLogger>();
    ///
    /// // code here...
    ///
    /// // retrieve service here
    /// let injector = Injector::global_async_rw();
    /// let logger = injector.get::<ConsoleLogger>();
    ///
    /// // use service here...
    /// ```
    #[cfg(all(feature = "globals", feature = "async"))]
    pub fn global_async_rw_mut() -> std::sync::RwLockWriteGuard<'static, Injector<crate::sync::AsyncRwLockMarker>> {
        ASYNC_RW_INJECTOR_INSTANCE.write()
            .unwrap()
    }
}