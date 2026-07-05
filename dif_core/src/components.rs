use crate::cell::InstanceCell;
use crate::sync::{LockOrCell, SendTrait};
use crate::Injector;
use std::any::{type_name, TypeId};
use std::sync::Arc;
use crate::container::DynInstanceCellFn;

pub trait Injectable : SendTrait + 'static {
}

pub trait FromInjector {
    fn from_injector(injector: &Injector) -> Self;
}

pub trait DynamicInjectable<T : Injectable + ?Sized> : FromInjector + SendTrait + 'static {
    fn create_dynamic(s: Arc<LockOrCell<Self>>) -> Arc<LockOrCell<T>>;
}

#[derive(Debug, Copy, Clone, Default, PartialEq, Eq)]
pub enum ComponentLifetime {
    #[default]
    Singleton,
    Transient
}

pub struct DynamicComponent {
    pub(crate) create_func: Box<DynInstanceCellFn>,
    pub(crate) unique_id: TypeId,

    #[cfg(debug_assertions)]
    pub(crate) type_name: &'static str,
}

pub struct Component {
    pub(crate) lifetime: ComponentLifetime,
    pub(crate) create_func: Box<DynInstanceCellFn>,
    pub(crate) unique_id: TypeId,

    pub(crate) dynamics: Vec<DynamicComponent>,
    
    #[cfg(debug_assertions)]
    pub(crate) type_name: &'static str,
}

impl Component {
    pub fn lifetime(&self) -> ComponentLifetime {
        self.lifetime
    }
    
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

pub struct ComponentBuilder<T> {
    lifetime: ComponentLifetime,
    create_func: fn(&Injector) -> T,
    
    dynamics: Vec<ComponentBuilderDynItem>,
}

impl Component {
    pub fn singleton<T : FromInjector + 'static>() -> ComponentBuilder<T> {
        ComponentBuilder {
            lifetime: ComponentLifetime::Singleton,
            create_func: T::from_injector,
            dynamics: Vec::new(),
        }
    }

    pub fn transient<T : FromInjector  + 'static>() -> ComponentBuilder<T> {
        ComponentBuilder {
            lifetime: ComponentLifetime::Transient,
            create_func: T::from_injector,
            dynamics: Vec::new(),
        }
    }
}

impl<T : FromInjector + 'static + SendTrait> ComponentBuilder<T> {
    pub fn with_factory(self, factory: fn(&Injector) -> T) -> Self {
        Self {
            lifetime: self.lifetime,
            create_func: factory,
            dynamics: Vec::new(),
        }
    }

    pub fn with_dynamic<TDyn : Injectable + ?Sized  + 'static>(self) -> Self
        where T : DynamicInjectable<TDyn>
    {
        let mut dynamics = self.dynamics;
        dynamics.push(ComponentBuilderDynItem {
            create_func: |injector| {
                let value = injector.get::<T>()
                    .expect("This should not be called before T gets inserted into the injector");
                let value = T::create_dynamic(value.value.clone());
                InstanceCell::new(value)
            },
            unique_id: TypeId::of::<TDyn>(),
            #[cfg(debug_assertions)]
            type_name: std::any::type_name::<TDyn>(),
        });
        Self {
            lifetime: self.lifetime,
            create_func: self.create_func,
            dynamics,
        }
    }

    pub fn build(self) -> Component {
        Component {
            lifetime: self.lifetime,
            create_func: Box::new(move |injector: &Injector| {
                let mutex = Arc::new(LockOrCell::new((self.create_func)(injector)));
                InstanceCell::new(mutex)
            }) as Box<_>,
            unique_id: TypeId::of::<T>(),
            dynamics: self.dynamics
                .into_iter()
                .map(|d| {
                    DynamicComponent {
                        create_func: Box::new(d.create_func),
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