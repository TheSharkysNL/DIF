mod components;
mod container;
pub mod sync;
pub mod cell;

use crate::container::DIContainer;
use crate::sync::{InjectorLock, InstanceCellLock};
pub use components::*;
use std::any::{Any, TypeId};

/// The global injector instance
static mut INJECTOR_INSTANCE: Option<Injector> = None; 

/// The main injector used for dependency injection
#[derive(Default)]
pub struct Injector {
    container: DIContainer,
}

impl Injector {
    /// Creates a new instance of the injector
    pub fn new() -> Self {
        Self {
            container: Default::default()
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
    /// .lock()
    /// .await; // get lock to the logger
    ///
    /// // use the instance
    /// logger.write("It worked!");
    /// ```
    pub fn get<T : 'static>(&self) -> Option<InjectorLock<T>> {
        self.container.get(self)
    }
    
    /// Gets a thread-safe Mutex for a `dyn` type of `T`. 
    /// 
    /// Returns `None` if the `T` instance has not been registered.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// // create injector
    /// let mut injector = Injector::new();
    ///     
    ///  // register dynamic type to the injector
    ///  injector.singleton_dyn::<ConsoleLogger, dyn Logger>(); 
    ///     
    ///  // code here...
    ///     
    ///  // get the instance to the dynamic type
    ///  let logger = injector.get_dyn::<dyn Logger>()
    ///     .unwrap(); // unwrap here as dyn Logger is known to have been registered to the injector
    ///  let mut logger = logger
    ///     .lock()
    ///     .await;
    ///     
    ///  // use the instance
    ///  logger.write("It worked!");
    /// ```
    pub fn get_dyn<T : Injectable + ?Sized + 'static>(&self) -> Option<InjectorLock<T>> {
        self.container.get_dyn(self)
    }

    /// Gets a thread-safe list of all the `dyn` instances of `T` that have been registered.
    ///
    /// Returns `None` if the `T` instances has not been registered.
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
    /// let loggers = injector.get_list::<dyn Logger>()
    ///     .unwrap(); // unwrap here as dyn Logger is known to have been registered to the injector
    /// for logger in loggers { // loop through all the instances
    ///     let mut logger = logger.lock()
    ///         .await; // get lock to specific instance
    ///     
    ///     // use the instance
    ///     logger.write("It worked!");
    /// }
    /// ```
    pub fn get_list<T : Injectable + ?Sized  + 'static>(&self) -> Option<impl Iterator<Item=InjectorLock<T>>> {
        self.container.get_list(self)
    }
    
    pub fn get_any(&self, type_id: TypeId) -> Option<InstanceCellLock> {
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
    pub fn produce<T : FromInjector>(&self) -> T {
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
    pub fn singleton<T : FromInjector + 'static>(&mut self) {
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
    pub fn transient<T : FromInjector + 'static>(&mut self) {
        self.component(
            Component::transient::<T>()
                .build()
        )
    }

    /// Registers a `dyn` singleton instance to the injector.
    ///
    /// A singleton meaning that the instance is created once 
    /// and then reused for every call to `injector.get_dyn::<TDyn>()` or `injector.get_list::<TDyn>()`.
    /// 
    /// # Edge cases
    /// 
    /// When registering multiple instances of a dynamic type. 
    /// Getting all the instances of that dynamic type can be done with the `injector.get_list::<TDyn>()`.
    /// If the regular (`injector.get_dyn::<TDyn>()`) method is used, the first instance that was registered will be resolved.
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
    pub fn singleton_dyn<T : DynamicInjectable<TDyn> + 'static, TDyn : Sync + Send + Injectable + ?Sized + 'static>(&mut self) {
        self.component(
            Component::with_no_id::<T>(ComponentLifetime::Singleton)
                .into_dynamic::<TDyn>()
                .build()
        )
    }

    /// Registers a `dyn` transient instance to the injector.
    ///
    /// A transient meaning that the instance is created 
    /// for every call to `injector.get_dyn::<TDyn>()` 
    /// or every time the iterator resolved from `injector.get_list::<TDyn>()` is iterated through.
    ///
    /// # Edge cases
    ///
    /// When registering multiple instances of a dynamic type. 
    /// Getting all the instances of that dynamic type can be done with the `injector.get_list::<TDyn>()`.
    /// If the regular (`injector.get_dyn::<TDyn>()`) method is used, the first instance that was registered will be resolved.
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
    pub fn transient_dyn<T : DynamicInjectable<TDyn> + 'static, TDyn : Sync + Send + Injectable + ?Sized + 'static>(&mut self) {
        self.component(
            Component::with_no_id::<T>(ComponentLifetime::Transient)
                .into_dynamic::<TDyn>()
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
    ///                 count: COUNT.fetch_add(0, Ordering::SeqCst)
    ///             }
    ///         })
    ///         .build()
    ///     );
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
    ///         .into_dynamic::<dyn Logger>() // use into_dynamic to use dyn instance
    ///         .build()
    ///  );
    /// ```
    /// 
    pub fn component(&mut self, component: Component) {
        self.container.register(component)
    }

    /// Gets a `'static` reference to the global injector. 
    #[allow(static_mut_refs)]
    pub fn global() -> &'static Self {
        // Safety: The global injector instance can only be created once.
        // This means that returning 'static references here is safe 
        // as this function will only return a reference to the instance once it is created
        // and after that the global instance never changes so there will be no dangling pointers
        unsafe {
            INJECTOR_INSTANCE
                .as_ref()
                .expect("Injector must be initialized first before a handle can be taken for it. Use the Injector::initialize method to initialize the injector.")
        }
    }

    /// Initializes the global injector. 
    /// 
    /// The `init_method` initializes the injector with the instances you want to register.
    /// 
    /// # Panics
    /// This method will panic if it is used more than once. 
    /// The global injector can only be initialized once and after that it can be retrieved via the Injector::global method.
    /// 
    /// # Examples
    /// 
    /// ```rust
    ///  Injector::initialize(|injector| { // use method to register instances
    ///     injector.singleton::<ConsoleLogger>();
    ///  });
    ///  
    ///  // get global injector
    ///  let value = Injector::global()
    ///     .get::<ConsoleLogger>(); // get the instance
    ///  assert!(value.is_some()); // This passes as the Injector is fully initialized
    /// ```
    /// 
    #[allow(static_mut_refs)]
    pub fn initialize<F : FnOnce(&mut Injector)>(init_method: F) {
        // Safety: The global injector instance can only be created once.
        // After it is created it will never be mutated.
        // This way it ensures that there are no race conditions
        unsafe {
            if INJECTOR_INSTANCE.is_some() {
                panic!("Another global instance of a injector is already active. Cannot create a global instance of a injector twice.");
            }
            
            let mut injector = Injector::new();
            
            init_method(&mut injector);
        
            INJECTOR_INSTANCE = Some(injector);
        }
    }
}