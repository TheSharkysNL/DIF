use crate::cell::InstanceCell;
use crate::sync::{LockOrCell, SendTrait};
use crate::Injector;
#[cfg(debug_assertions)]
use std::any::type_name;
use std::any::TypeId;
use std::marker::PhantomData;
use std::sync::Arc;
use crate::container::DynInstanceCellFn;

/// A marker trait used to show that a type can be injected.
pub trait Injectable : SendTrait + 'static {
}

/// Used to create types for the injector.
pub trait FromInjector {
    /// Create Self using the `injector`
    fn from_injector(injector: &Injector) -> Self;
}

/// A dynamic injectable. Used for dyn coercion
pub trait DynamicInjectable<T : Injectable + ?Sized> : FromInjector + SendTrait + 'static {
    /// Used to create a dynamic instance from the Self type.
    fn create_dynamic(s: Arc<LockOrCell<Self>>) -> Arc<LockOrCell<T>>;
}

/// The lifetime of a component. Can be either a singleton or transient. 
#[derive(Debug, Copy, Clone, Default, PartialEq, Eq)]
pub enum ComponentLifetime {
    /// Creates one single instance of the type.
    #[default]
    Singleton,
    /// Creates a new instance of the type each time it is retrieved.
    Transient
}

pub(crate) struct DynamicComponent {
    pub(crate) create_func: ComponentCreateFunction,
    pub(crate) unique_id: TypeId,

    #[cfg(debug_assertions)]
    pub(crate) type_name: &'static str,
}

pub(crate) enum ComponentCreateFunction {
    Pointer(fn(&Injector) -> InstanceCell),
    Boxed(Box<DynInstanceCellFn>)
}

impl ComponentCreateFunction {
    pub fn call(&self, injector: &Injector) -> InstanceCell {
        match self {
            ComponentCreateFunction::Pointer(p) => p(injector),
            ComponentCreateFunction::Boxed(b) => b(injector)
        }
    }
}

/// A component that can be added to a Injector.
pub struct Component {
    pub(crate) lifetime: ComponentLifetime,
    pub(crate) create_func: ComponentCreateFunction,
    pub(crate) unique_id: TypeId,

    pub(crate) dynamics: Vec<DynamicComponent>,
    
    #[cfg(debug_assertions)]
    pub(crate) type_name: &'static str,
}

impl Component {
    /// Returns the lifetime of the component
    pub fn lifetime(&self) -> ComponentLifetime {
        self.lifetime
    }
    
    /// Returns the unique id/TypeId of the type.
    pub fn unique_id(&self) -> TypeId {
        self.unique_id
    }
}

struct ComponentBuilderDynItem {
    create_func: fn(&Injector) -> InstanceCell,
    unique_id: TypeId,

    #[cfg(debug_assertions)]
    type_name: &'static str,
}

/// A builder for creating the component for the type `T`. Can be initialized via the `Component`'s methods.
pub struct ComponentBuilder<T> {
    lifetime: ComponentLifetime,
    factory_func: Option<Box<DynInstanceCellFn>>,
    
    dynamics: Vec<ComponentBuilderDynItem>,
    phantom: PhantomData<T>,
}

impl Component {
    /// Creates a singleton component and returns a component builder.
    pub fn singleton<T : FromInjector + 'static>() -> ComponentBuilder<T> {
        ComponentBuilder {
            lifetime: ComponentLifetime::Singleton,
            factory_func: None,
            dynamics: Vec::new(),
            phantom: PhantomData,
        }
    }

    /// Creates a transient component and returns a component builder.
    pub fn transient<T : FromInjector  + 'static>() -> ComponentBuilder<T> {
        ComponentBuilder {
            lifetime: ComponentLifetime::Transient,
            factory_func: None,
            dynamics: Vec::new(),
            phantom: PhantomData,
        }
    }
}

impl<T : FromInjector + 'static + SendTrait> ComponentBuilder<T> {
    /// Creates the type using a custom factory function
    #[cfg(any(feature = "multithreaded", feature = "async"))]
    pub fn with_factory(self, factory: impl Fn(&Injector) -> T + Send + Sync + 'static) -> Self {
        Self {
            lifetime: self.lifetime,
            factory_func: Some(Box::new(move |injector: &Injector| {
                let mutex = Arc::new(LockOrCell::new(factory(injector)));
                InstanceCell::new(mutex)
            }) as Box<dyn Fn(&Injector) -> InstanceCell + Send + Sync>),
            dynamics: Vec::new(),
            phantom: PhantomData,
        }
    }

    /// Creates the type using a custom factory function
    #[cfg(not(any(feature = "multithreaded", feature = "async")))]
    pub fn with_factory(self, factory: impl Fn(&Injector) -> T + 'static) -> Self {
        Self {
            lifetime: self.lifetime,
            factory_func: Some(Box::new(move |injector: &Injector| {
                let value = factory(injector);
                InstanceCell::new(Arc::new(LockOrCell::new(value)))
            }) as Box<_>),
            dynamics: Vec::new(),
            phantom: PhantomData,
        }
    }

    /// Lets the type T be retrieved via the injector using a dynamic type. Usefull if you want other crates to implement their own special type that will be injected.
    /// This way you can retrieve the type using `injector.get::<TDyn>()` or `injector.get_list::<TDyn>()`.
    /// Multiple dynamic types can be set.
    pub fn with_dynamic<TDyn : Injectable + ?Sized  + 'static>(self) -> Self
        where T : DynamicInjectable<TDyn>
    {
        let mut dynamics = self.dynamics;
        dynamics.push(ComponentBuilderDynItem {
            create_func: |injector| {
                let value = injector.get::<T>()
                    .expect("This should not panic as T must have been into the injector");
                let value = T::create_dynamic(value.value.clone());
                InstanceCell::new(value)
            },
            unique_id: TypeId::of::<TDyn>(),
            #[cfg(debug_assertions)]
            type_name: std::any::type_name::<TDyn>(),
        });
        Self {
            lifetime: self.lifetime,
            factory_func: self.factory_func,
            dynamics,
            phantom: PhantomData,
        }
    }

    /// Creates the component 
    pub fn build(self) -> Component {
        Component {
            lifetime: self.lifetime,
            create_func: match self.factory_func {
                Some(x) => ComponentCreateFunction::Boxed(x),
                None => ComponentCreateFunction::Pointer(|injector: &Injector| {
                    let mutex = Arc::new(LockOrCell::new(T::from_injector(injector)));
                    InstanceCell::new(mutex)
                }),
            },
            unique_id: TypeId::of::<T>(),
            dynamics: self.dynamics
                .into_iter()
                .map(|d| {
                    DynamicComponent {
                        create_func: ComponentCreateFunction::Pointer(d.create_func),
                        unique_id: d.unique_id,
                        #[cfg(debug_assertions)]
                        type_name: d.type_name
                    }
                })
                .collect::<Vec<_>>(),
            #[cfg(debug_assertions)]
            type_name: type_name::<T>(),
        }
    }
}