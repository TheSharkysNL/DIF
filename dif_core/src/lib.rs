mod components;
mod container;
pub mod sync;
mod cell;

use crate::container::DIContainer;
use crate::sync::{InstanceCellLock, Lock};
use std::any::{TypeId};

pub use components::*;
pub use crate::container::DependencyIter;

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
    pub fn singleton<T : FromInjector<L> + 'static>(&mut self) {
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
    pub fn transient<T : FromInjector<L> + 'static>(&mut self) {
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
    pub fn singleton_dyn<T : DynamicInjectable<TDyn, L>, TDyn : Injectable + ?Sized>(&mut self) {
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
    pub fn transient_dyn<T : DynamicInjectable<TDyn, L>, TDyn : Injectable + ?Sized>(&mut self) {
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
    
    // /// Gets the global injector so services can be retrieved. Can panic if the underlying rwlock was poisoned.
    // #[cfg(any(feature = "async", feature = "multithreaded"))]
    // pub fn global() -> std::sync::RwLockReadGuard<'static, Injector<crate::sync::Mutex>> {
    //     INJECTOR_INSTANCE.read()
    //         .unwrap()
    // }
    // 
    // /// Gets the global injector to be mutated. Can panic if the underlying rwlock was poisoned.
    // #[cfg(any(feature = "async", feature = "multithreaded"))]
    // pub fn global_mut() -> std::sync::RwLockWriteGuard<'static, Injector<crate::sync::Mutex>> {
    //     INJECTOR_INSTANCE.write()
    //         .unwrap()
    // }

    // /// Initially mutates the injector for later use.
    // #[cfg(not(any(feature = "async", feature = "multithreaded")))]
    // #[allow(static_mut_refs)]
    // pub fn initialize<F : FnOnce(&mut Injector)>(init_func: F) {
    //     // Safety: The injector instance can only be initialized once and then never be mutated again.
    //     // After this you can only get an injector reference to retrieve its services.
    //     // Meaning that it cannot be mutated when someone has a reference to it.
    //     unsafe {
    //         match &mut INJECTOR_INSTANCE {
    //             Some(_) => panic!("The injector cannot be initialized more than once."),
    //             None => {
    //                 let mut new_injector = Injector::new();
    //                 init_func(&mut new_injector);
    //                 
    //                 INJECTOR_INSTANCE = Some(new_injector);
    //             }
    //         }
    //     }
    // }
    // 
    // /// Gets the global injector so services can be retrieved.
    // #[cfg(not(any(feature = "async", feature = "multithreaded")))]
    // #[allow(static_mut_refs)]
    // pub fn global() -> &'static Injector {
    //     // Safety: The injector instance can only be initialized once and then never be mutated again.
    //     // After this you can only get an injector reference to retrieve its services.
    //     // Meaning that it cannot be mutated when someone has a reference to it.
    //     unsafe {
    //         match &INJECTOR_INSTANCE {
    //             Some(instance) => instance,
    //             None => panic!("Injector has not been initialized. Please us the static initialize function to initialize the injector."),
    //         }
    //     }
    // }
    
}