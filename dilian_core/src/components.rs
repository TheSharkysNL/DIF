use crate::cell::{AnyMetadata, InstanceCell};
use crate::Injector;
#[cfg(debug_assertions)]
use std::any::type_name;
use std::any::TypeId;
use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;
use crate::container::{DynInstanceCellFn};
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
#[derive(Default)]
pub enum ComponentLifetime<C> {
    /// Creates one instance of the type and reuses it for subsequent requests.
    #[default]
    Singleton,
    /// Creates a new instance of the type for each request.
    Transient,
    /// A custom component lifetime.
    Custom(C),
}

impl<C : Clone> Clone for ComponentLifetime<C> {
    fn clone(&self) -> Self {
        match self {
            ComponentLifetime::Singleton => ComponentLifetime::Singleton,
            ComponentLifetime::Transient => ComponentLifetime::Transient,
            ComponentLifetime::Custom(checker) => ComponentLifetime::Custom(checker.clone()),
        }
    }
}

impl<L> PartialEq for ComponentLifetime<L> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (ComponentLifetime::Singleton, ComponentLifetime::Singleton) => true,
            (ComponentLifetime::Transient, ComponentLifetime::Transient) => true,
            (ComponentLifetime::Custom(_), ComponentLifetime::Custom(_)) => true,
            _ => false,
        }
    }
}

impl<L> Debug for ComponentLifetime<L> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ComponentLifetime::Singleton => write!(f, "Singleton"),
            ComponentLifetime::Transient => write!(f, "Transient"),
            ComponentLifetime::Custom(_) => write!(f, "Custom"),
        }
    }
}

/// Used for custom lifetimes on components. See [`Component::custom`].
/// 
/// A new checker is created for each service added to the injector using [`Component::custom`].
/// This includes every [`ComponentBuilder::with_dynamic`] added.
pub trait ComponentLifetimeChecker<L : Lock> : Send + Sync {
    /// Checks if a component needs a new instance. 
    ///
    /// If true a new component will be created and the old one will be destroyed.
    /// If false the old component will be passed to the caller of the function.
    fn needs_new_instance(&self, injector: &Injector<L>) -> bool;
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
pub struct Component<L : Lock, C> {
    pub(crate) lifetime: ComponentLifetime<C>,
    pub(crate) create_func: ComponentCreateFunction<L>,
    pub(crate) unique_id: TypeId,

    pub(crate) dynamics: Vec<DynamicComponent<L>>,
    
    #[cfg(debug_assertions)]
    pub(crate) type_name: &'static str,
}

impl<L : Lock, C> Component<L, C> {
    /// Returns the lifetime of the component.
    pub fn lifetime(&self) -> &ComponentLifetime<C> {
        &self.lifetime
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
pub struct ComponentBuilder<T, L : Lock, C, F> {
    lifetime: ComponentLifetime<C>,
    factory_func: Option<Box<DynInstanceCellFn<L>>>,
    
    dynamics: Vec<ComponentBuilderDynItem<L>>,
    phantom: PhantomData<T>,
    phantom_fn: PhantomData<F>,
}

impl<L : Lock> Component<L, ()> {
    /// Creates a singleton component builder.
    pub fn singleton<T : 'static>() -> ComponentBuilder<T, L, (), ()>
        where L : LockBound<T>
    {
        ComponentBuilder {
            lifetime: ComponentLifetime::Singleton,
            factory_func: None,
            dynamics: Vec::new(),
            phantom: PhantomData,
            phantom_fn: PhantomData,
        }
    }

    /// Creates a transient component builder.
    pub fn transient<T : 'static>() -> ComponentBuilder<T, L, (), ()>
        where L : LockBound<T>
    {
        ComponentBuilder {
            lifetime: ComponentLifetime::Transient,
            factory_func: None,
            dynamics: Vec::new(),
            phantom: PhantomData,
            phantom_fn: PhantomData,
        }
    }
    
    /// Creates a component builder containing a custom lifetime.
    pub fn custom<T : 'static, C : ComponentLifetimeChecker<L> + Clone>(checker: C) -> ComponentBuilder<T, L, C, ()> 
        where L : LockBound<T>
    {
        ComponentBuilder {
            lifetime: ComponentLifetime::Custom(checker),
            factory_func: None,
            dynamics: Vec::new(),
            phantom: PhantomData,
            phantom_fn: PhantomData,
        }
    }
}

impl<T : 'static, L : Lock, C> ComponentBuilder<T, L, C, ()> {
    /// Configures a custom factory function for creating the type.
    pub fn with_factory<F2 : Fn(&Injector<L>) -> T + Send + Sync + 'static>(self, factory: F2) -> ComponentBuilder<T, L, C, F2> {
        ComponentBuilder {
            lifetime: self.lifetime,
            factory_func: Some(Box::new(move |injector: &Injector<L>| {
                let mutex = L::new(factory(injector));
                InstanceCell::new(mutex)
            }) as Box<_>),
            dynamics: Vec::new(),
            phantom: PhantomData,
            phantom_fn: PhantomData,
        }
    }

    /// Builds the component.
    pub fn build(self) -> Component<L, C> 
        where T : FromInjector<L>
    {
        self.build_internal(ComponentCreateFunction::Pointer(|injector: &Injector<L>| {
            let mutex = L::new(T::from_injector(injector));
            InstanceCell::new(mutex)
        }))
    }
}

impl<T : 'static, L : Lock, C, F> ComponentBuilder<T, L, C, F> {
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
            phantom_fn: PhantomData,
        }
    }
    
    pub fn build_with_factory(mut self) -> Component<L, C> 
        where F : Fn(&Injector<L>) -> T + Send + Sync + 'static
    {
        let create_func = self.factory_func.take()
            .unwrap();
        self.build_internal(ComponentCreateFunction::Boxed(create_func))
    }

    fn build_internal(self, create_func: ComponentCreateFunction<L>) -> Component<L, C> {
        Component {
            lifetime: self.lifetime,
            create_func,
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

impl<L : Lock> ComponentLifetimeChecker<L> for () {
    fn needs_new_instance(&self, _: &Injector<L>) -> bool {
        false
    }
}