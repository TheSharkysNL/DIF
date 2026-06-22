use crate::cell::InstanceCell;
use crate::sync::LockOrCell;
use crate::Injector;
use std::any::TypeId;
use std::sync::Arc;

pub trait Injectable {
}

pub trait FromInjector {
    fn from_injector(injector: &Injector) -> Self;
}

pub trait DynamicInjectable<T : Injectable + Sync + Send + ?Sized> : FromInjector {
    fn into_dynamic(self) -> Arc<LockOrCell<T>>;
}

#[derive(Debug, Copy, Clone, Default, PartialEq, Eq)]
pub enum ComponentLifetime {
    #[default]
    Singleton,
    Transient
}

pub struct Component {
    lifetime: ComponentLifetime,
    create_func: Box<dyn Fn(&Injector) -> InstanceCell + Send + Sync>,
    unique_id: TypeId,
    is_dynamic: bool,
    
    #[cfg(debug_assertions)]
    type_name: &'static str,
}

impl Component {
    pub fn lifetime(&self) -> ComponentLifetime {
        self.lifetime
    }
    
    pub fn unique_id(&self) -> TypeId {
        self.unique_id
    }
    
    pub fn is_dynamic(&self) -> bool {
        self.is_dynamic
    }
    
    pub fn into_create_func(self) -> Box<dyn Fn(&Injector) -> InstanceCell + Send + Sync> {
        self.create_func
    }
    
    #[cfg(debug_assertions)]
    pub fn type_name(&self) -> &'static str {
        self.type_name
    }
}

pub struct ComponentBuilder<T> {
    lifetime: ComponentLifetime,
    create_func: fn(&Injector) -> T,
    dyn_create_func: Option<fn(&Injector, fn(&Injector) -> T) -> InstanceCell>,
    unique_id: Option<TypeId>,
    is_dynamic: bool,
    
    #[cfg(debug_assertions)]
    type_name: &'static str,
}

impl Component {
    pub fn singleton<T : FromInjector + 'static>() -> ComponentBuilder<T> {
        ComponentBuilder {
            lifetime: ComponentLifetime::Singleton,
            create_func: T::from_injector,
            dyn_create_func: None,
            unique_id: Some(TypeId::of::<T>()),
            is_dynamic: false,
            #[cfg(debug_assertions)]
            type_name: std::any::type_name::<T>(),
        }
    }

    pub fn transient<T : FromInjector  + 'static>() -> ComponentBuilder<T> {
        ComponentBuilder {
            lifetime: ComponentLifetime::Transient,
            create_func: T::from_injector,
            dyn_create_func: None,
            unique_id: Some(TypeId::of::<T>()),
            is_dynamic: false,
            #[cfg(debug_assertions)]
            type_name: std::any::type_name::<T>(),
        }
    }
    
    /// Make sure that if you call this function you always call the into_dynamic function to set the unique_id
    pub(crate) fn with_no_id<T : FromInjector>(lifetime: ComponentLifetime) -> ComponentBuilder<T> {
        ComponentBuilder {
            lifetime,
            create_func: T::from_injector,
            dyn_create_func: None,
            unique_id: None,
            is_dynamic: false,
            #[cfg(debug_assertions)]
            type_name: std::any::type_name::<T>(),
        }
    }
}

impl<T : FromInjector + 'static> ComponentBuilder<T> {
    pub fn with_factory(self, factory: fn(&Injector) -> T) -> Self {
        Self {
            lifetime: self.lifetime,
            create_func: factory,
            dyn_create_func: None,
            unique_id: self.unique_id,
            is_dynamic: self.is_dynamic,
            #[cfg(debug_assertions)]
            type_name: self.type_name,
        }
    }

    pub fn into_dynamic<TDyn : Injectable + Sync + Send + ?Sized  + 'static>(self) -> Self
        where T : DynamicInjectable<TDyn>
    {
        Self {
            lifetime: self.lifetime,
            create_func: self.create_func,
            dyn_create_func: Some(|injector, create_func| {
                let dynamic = T::into_dynamic(create_func(injector));
                InstanceCell::new(dynamic)
            }),
            unique_id: Some(TypeId::of::<TDyn>()),
            is_dynamic: true,
            #[cfg(debug_assertions)]
            type_name: std::any::type_name::<TDyn>(),
        }
    }

    pub fn build(self) -> Component {
        Component {
            lifetime: self.lifetime,
            create_func: self.dyn_create_func
                .map(|x| Box::new(move |injector: &Injector| x(injector, self.create_func)) as Box<_>)
                .unwrap_or_else(|| Box::new(move |injector: &Injector| {
                    let mutex = Arc::new(LockOrCell::new((self.create_func)(injector)));
                    InstanceCell::new(mutex)
                }) as Box<_>),
            unique_id: self.unique_id.expect("Should never be None here."),
            is_dynamic: self.is_dynamic,
            #[cfg(debug_assertions)]
            type_name: self.type_name,
        }
    }
}