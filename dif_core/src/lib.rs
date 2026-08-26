#![cfg_attr(doc, feature(doc_cfg))]

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
static MUTEX_INJECTOR_INSTANCE: std::sync::LazyLock<std::sync::RwLock<Injector<crate::sync::Mutex>>> = std::sync::LazyLock::new(|| std::sync::RwLock::new(Injector::new()));

#[cfg(feature = "globals")]
static RW_INJECTOR_INSTANCE: std::sync::LazyLock<std::sync::RwLock<Injector<crate::sync::RwLock>>> = std::sync::LazyLock::new(|| std::sync::RwLock::new(Injector::new()));

#[cfg(feature = "globals")]
static mut REFCELL_INJECTOR_INSTANCE: Option<Injector<crate::sync::RefCell>> = None;

#[cfg(all(feature = "globals", feature = "async"))]
static ASYNC_MUTEX_INJECTOR_INSTANCE: std::sync::LazyLock<std::sync::RwLock<Injector<crate::sync::AsyncMutex>>> = std::sync::LazyLock::new(|| std::sync::RwLock::new(Injector::new()));

#[cfg(all(feature = "globals", feature = "async"))]
static ASYNC_RW_INJECTOR_INSTANCE: std::sync::LazyLock<std::sync::RwLock<Injector<crate::sync::AsyncRwLock>>> = std::sync::LazyLock::new(|| std::sync::RwLock::new(Injector::new()));

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
    /// Creates a new instance of the injector
    pub fn new() -> Self {
        Self {
            container: Default::default(),
        }
    }
    
    /// Gets a thread-safe Mutex for the type `T`. 
    /// 
    /// Returns `None` if the `T` instance has not been registered.
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
    /// Can also be used to retrieve dynamic instances. This will get the first instance that was added to the injector.
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
    /// If you want to get a specific instance of the dynamic type. You can use injector.get_by_id.
    pub fn get<T : ?Sized + 'static>(&self) -> Option<L::Lock<T>> {
        self.container.get(self)
    }

    /// Gets a thread-safe list of all the `dyn` instances of `T` that have been registered.
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
    
    /// Gets a dynamic type using its type id. 
    /// This can be used to retrieve a specific type at runtime.
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
    
    /// Gets an Any type based on the given TypeId that can be downcast.
    pub fn get_any(&self, type_id: TypeId) -> Option<InstanceCellLock<L>> {
        self.container.get_instance_cell(type_id, self)
    }
    
    /// Creates a new instance of the type `T` by using the instance components within the injector.
    /// Be weary as this method will always create a new instance even if it was registered as singleton.
    /// 
    /// This method can be used to get ownership of a type instead of a `Mutex` type
    /// 
    /// # Panics
    /// 
    /// If a component is not found within the injector
    /// it will panic
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
    
    /// Registers a singleton instance to the injector.
    /// 
    /// A singleton meaning that the instance is created once 
    /// and then reused for every call to `injector.get::<T>()`.
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

    /// Registers a transient instance to the injector.
    ///
    /// A transient meaning that the instance is created 
    /// for every call to `injector.get::<T>()`.
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
    /// A singleton meaning that the instance is created once 
    /// and then reused for every call to `injector.get::<TDyn>()` or `injector.get_list::<TDyn>()`.
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
    /// A transient meaning that the instance is created 
    /// for every call to `injector.get::<TDyn>()` 
    /// or every time the iterator resolved from `injector.get_list::<TDyn>()` is iterated through.
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
    
    /// Registers a component type to further customize the instance registered
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
    pub fn component(&mut self, component: Component<L>) {
        self.container.register(component)
    }

    /// Gets a global reference to the [`Injector`]. This injector only works with [`std::sync::Arc<Mutex<T>>`].
    /// Can be used to retrieve services from the [`Injector`]. 
    /// 
    /// To add new services use [`Self::global_mutex_mut`].
    /// 
    /// # Example:
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
    pub fn global_mutex() -> std::sync::RwLockReadGuard<'static, Injector<crate::sync::Mutex>> {
        MUTEX_INJECTOR_INSTANCE.read()
            .unwrap()
    }

    /// Gets a global mutable reference to the [`Injector`]. This injector only works with [`std::sync::Arc<Mutex<T>>`].
    /// Can be used to retrieve and add new services into the [`Injector`]. 
    ///
    /// To get a non-mutable reference use [`Self::global_mutex`].
    ///
    /// # Example:
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
    pub fn global_mutex_mut() -> std::sync::RwLockWriteGuard<'static, Injector<crate::sync::Mutex>> {
        MUTEX_INJECTOR_INSTANCE.write()
            .unwrap()
    }

    /// Gets a global reference to the [`Injector`]. This injector only works with [`std::sync::Arc<RwLock<T>>`].
    /// Can be used to retrieve services from the [`Injector`]. 
    ///
    /// To add new services use [`Self::global_rw_mut`].
    ///
    /// # Example:
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
    pub fn global_rw() -> std::sync::RwLockReadGuard<'static, Injector<crate::sync::RwLock>> {
        RW_INJECTOR_INSTANCE.read()
            .unwrap()
    }

    /// Gets a global mutable reference to the [`Injector`]. This injector only works with [`std::sync::Arc<RwLock<T>>`].
    /// Can be used to retrieve and add new services into the [`Injector`]. 
    ///
    /// To get a non-mutable reference use [`Self::global_rw`].
    ///
    /// # Example:
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
    pub fn global_rw_mut() -> std::sync::RwLockWriteGuard<'static, Injector<crate::sync::RwLock>> {
        RW_INJECTOR_INSTANCE.write()
            .unwrap()
    }
    
    /// Used to initialize the services for use with the [`Injector`]. 
    /// This [`Injector`] only works with [`std::rc::Rc<RefCell<T>>`].
    /// 
    /// This function is used to add the services for them to be retrieved later via the [`Self::global_ref_cell`] function.
    /// You can pass an `init_func` which gets a mutable reference to the [`Injector`]. 
    /// After this you can use the singleton or transient functions to add new services.
    /// 
    /// # Example:
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
    pub fn initialize_ref_cell<F : FnOnce(&mut Injector<crate::sync::RefCell>)>(init_func: F) {
        // Safety: The injector instance can only be initialized once and then never be mutated again.
        // After this you can only get an injector reference to retrieve its services.
        // Meaning that it cannot be mutated when someone has a reference to it.
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

    /// Used to retrieve the services from the [`Injector`]. 
    /// This [`Injector`] only works with [`std::rc::Rc<RefCell<T>>`].
    /// 
    /// This function will panic if the [`Injector`] was not initialized. 
    /// You can initialize the [`Injector`] using the [`Self::initialize_ref_cell`] function.
    ///
    /// # Example:
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
    pub fn global_ref_cell() -> &'static Injector<crate::sync::RefCell> {
        // Safety: The injector instance can only be initialized once and then never be mutated again.
        // After this you can only get an injector reference to retrieve its services.
        // Meaning that it cannot be mutated when someone has a reference to it.
        unsafe {
            match &REFCELL_INJECTOR_INSTANCE {
                Some(injector) => injector,
                None => panic!("The injector must first be initialized. Use the initialize_ref_cell function first."),
            }
        }
    }

    /// Gets a global reference to the [`Injector`]. 
    /// This injector only works with [`std::sync::Arc<tokio::sync::Mutex<T>>`].
    /// Can be used to retrieve services from the [`Injector`]. 
    /// 
    /// This [`Injector`] can be used for async applications.
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
    pub fn global_async_mutex() -> std::sync::RwLockReadGuard<'static, Injector<crate::sync::AsyncMutex>> {
        ASYNC_MUTEX_INJECTOR_INSTANCE.read()
            .unwrap()
    }

    /// Gets a global mutable reference to the [`Injector`]. This injector only works with [`std::sync::Arc<tokio::sync::Mutex<T>>`].
    /// Can be used to retrieve and add new services into the [`Injector`]. 
    /// 
    /// This [`Injector`] can be used for async applications.
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
    pub fn global_async_mutex_mut() -> std::sync::RwLockWriteGuard<'static, Injector<crate::sync::AsyncMutex>> {
        ASYNC_MUTEX_INJECTOR_INSTANCE.write()
            .unwrap()
    }

    /// Gets a global reference to the [`Injector`]. 
    /// This injector only works with [`std::sync::Arc<tokio::sync::RwLock<T>>`].
    /// Can be used to retrieve services from the [`Injector`]. 
    /// 
    /// This [`Injector`] can be used for async applications.
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
    pub fn global_async_rw() -> std::sync::RwLockReadGuard<'static, Injector<crate::sync::AsyncRwLock>> {
        ASYNC_RW_INJECTOR_INSTANCE.read()
            .unwrap()
    }

    /// Gets a global mutable reference to the [`Injector`]. 
    /// This injector only works with [`std::sync::Arc<tokio::sync::RwLock<T>>`].
    /// Can be used to retrieve and add new services into the [`Injector`]. 
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
    pub fn global_async_rw_mut() -> std::sync::RwLockWriteGuard<'static, Injector<crate::sync::AsyncRwLock>> {
        ASYNC_RW_INJECTOR_INSTANCE.write()
            .unwrap()
    }
}