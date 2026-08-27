use crate::cell::{AnyMetadata, InstanceCell};
use crate::Injector;
#[cfg(debug_assertions)]
use std::any::type_name;
use std::any::TypeId;
use std::marker::PhantomData;
use crate::container::DynInstanceCellFn;
use crate::sync::{Lock, LockBound};

/// A marker trait indicating that a type can be injected.
pub trait Injectable : 'static {
}

/// Used to create types for the injector.
pub trait FromInjector<L : Lock> : AnyMetadata<L> {
    /// Creates `Self` using the `injector`.
    fn from_injector(injector: &Injector<L>) -> Self;
}

/// A dynamic injectable. Used for dyn coercion
pub trait DynamicInjectable<T : Injectable + ?Sized, L : Lock> : FromInjector<L> + 'static {
    /// Creates a dynamic instance from `Self`.
    fn create_dynamic(s: L::Lock<Self>) -> L::Lock<T>;
}

/// The lifetime of a component: either singleton or transient.
#[derive(Debug, Copy, Clone, Default, PartialEq, Eq)]
pub enum ComponentLifetime {
    /// Creates one instance of the type and reuses it for subsequent requests.
    #[default]
    Singleton,
    /// Creates a new instance of the type for each request.
    Transient
}

pub(crate) struct DynamicComponent<L : Lock> {
    pub(crate) create_func: ComponentCreateFunction<L>,
    pub(crate) unique_id: TypeId,

    #[cfg(debug_assertions)]
    pub(crate) type_name: &'static str,
}

pub(crate) enum ComponentCreateFunction<L : Lock> {
    Pointer(fn(&Injector<L>) -> InstanceCell<L>),
    Boxed(Box<DynInstanceCellFn<L>>)
}

impl<L : Lock> ComponentCreateFunction<L> {
    pub fn call(&self, injector: &Injector<L>) -> InstanceCell<L> {
        match self {
            ComponentCreateFunction::Pointer(p) => p(injector),
            ComponentCreateFunction::Boxed(b) => b(injector)
        }
    }
}

/// A component that can be added to an [`Injector`].
pub struct Component<L : Lock> {
    pub(crate) lifetime: ComponentLifetime,
    pub(crate) create_func: ComponentCreateFunction<L>,
    pub(crate) unique_id: TypeId,

    pub(crate) dynamics: Vec<DynamicComponent<L>>,
    
    #[cfg(debug_assertions)]
    pub(crate) type_name: &'static str,
}

impl<L : Lock> Component<L> {
    /// Returns the lifetime of the component.
    pub fn lifetime(&self) -> ComponentLifetime {
        self.lifetime
    }
    
    /// Returns the unique [`TypeId`] of the component's type.
    pub fn unique_id(&self) -> TypeId {
        self.unique_id
    }
}

struct ComponentBuilderDynItem<L : Lock> {
    create_func: fn(&Injector<L>) -> InstanceCell<L>,
    unique_id: TypeId,

    #[cfg(debug_assertions)]
    type_name: &'static str,
}

/// A builder for creating the component for the type `T`. Can be initialized via the `Component`'s methods.
pub struct ComponentBuilder<T, L : Lock> {
    lifetime: ComponentLifetime,
    factory_func: Option<Box<DynInstanceCellFn<L>>>,
    
    dynamics: Vec<ComponentBuilderDynItem<L>>,
    phantom: PhantomData<T>,
}

impl<L : Lock> Component<L> {
    /// Creates a singleton component builder.
    pub fn singleton<T : FromInjector<L> + 'static>() -> ComponentBuilder<T, L>
        where L : LockBound<T>
    {
        ComponentBuilder {
            lifetime: ComponentLifetime::Singleton,
            factory_func: None,
            dynamics: Vec::new(),
            phantom: PhantomData,
        }
    }

    /// Creates a transient component builder.
    pub fn transient<T : FromInjector<L>  + 'static>() -> ComponentBuilder<T, L>
        where L : LockBound<T>
    {
        ComponentBuilder {
            lifetime: ComponentLifetime::Transient,
            factory_func: None,
            dynamics: Vec::new(),
            phantom: PhantomData,
        }
    }
}

impl<T : FromInjector<L> + 'static, L : Lock> ComponentBuilder<T, L> {
    /// Configures a custom factory function for creating the type.
    pub fn with_factory(self, factory: impl Fn(&Injector<L>) -> T + Send + Sync + 'static) -> Self {
        Self {
            lifetime: self.lifetime,
            factory_func: Some(Box::new(move |injector: &Injector<L>| {
                let mutex = L::new(factory(injector));
                InstanceCell::new(mutex)
            }) as Box<_>),
            dynamics: Vec::new(),
            phantom: PhantomData,
        }
    }

    /// Makes `T` retrievable through the injector as a dynamic type.
    /// This is useful when another crate provides the dynamic trait
    /// implementation to be injected.
    /// You can retrieve it with `injector.get::<TDyn>()` or
    /// `injector.get_list::<TDyn>()`.
    /// Multiple dynamic types can be set.
    pub fn with_dynamic<TDyn : Injectable + ?Sized  + 'static + AnyMetadata<L>>(self) -> Self
        where T : DynamicInjectable<TDyn, L>,
              L : LockBound<TDyn>
    {
        let mut dynamics = self.dynamics;
        dynamics.push(ComponentBuilderDynItem {
            create_func: |injector| {
                let value = injector.get::<T>()
                    .expect("This should not panic as T must have been into the injector");
                let value = T::create_dynamic(value.clone());
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

    /// Builds the component.
    pub fn build(self) -> Component<L> {
        Component {
            lifetime: self.lifetime,
            create_func: match self.factory_func {
                Some(x) => ComponentCreateFunction::Boxed(x),
                None => ComponentCreateFunction::Pointer(|injector: &Injector<L>| {
                    let mutex = L::new(T::from_injector(injector));
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