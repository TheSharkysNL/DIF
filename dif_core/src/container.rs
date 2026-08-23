use crate::cell::InstanceCell;
use crate::components::{Component, ComponentCreateFunction, ComponentLifetime};
use crate::sync::{InjectorLock, InstanceCellLock};
use crate::Injector;
use std::any::{type_name, TypeId};
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fmt::Write;
use std::marker::PhantomData;
use std::mem;
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, RwLock};
use rustc_hash::{FxBuildHasher};

#[cfg(any(feature = "multithreaded", feature = "async"))]
pub(crate) type DynInstanceCellFn = dyn Fn(&Injector) -> InstanceCell + Send + Sync;

#[cfg(not(any(feature = "multithreaded", feature = "async")))]
pub(crate) type DynInstanceCellFn = dyn Fn(&Injector) -> InstanceCell;

#[derive(Default)]
pub(crate) struct DIContainer {
    components: HashMap<TypeId, SingleOrList<ContainerComponent>, FxBuildHasher>,
    #[cfg(debug_assertions)]
    current_dependency_chain: std::sync::Mutex<Vec<TypeId>>,
    #[cfg(debug_assertions)]
    current_dependency_chain_names: std::sync::Mutex<Vec<&'static str>>,
}

impl DIContainer {
    pub fn register(&mut self, component: Component) {
        let components: Vec<ContainerComponent> = component.into();
        for component in components {
            let unique_id = component.unique_id;
            #[cfg(debug_assertions)]
            let type_name = component.type_name;
            match self.components.entry(unique_id) {
                // if it is already occupied then it could be added to a list
                Entry::Occupied(mut o) => {
                    let value = o.get_mut();
                    match value {
                        SingleOrList::Single(item) => {
                            // If the item is a single item then check if the item is dynamic meaning it can be a list
                            if !item.dynamic_id.is_some() {
                                if cfg!(debug_assertions) { // give more informative error when in debug
                                    #[cfg(debug_assertions)]
                                    panic!("You are trying to register the type '{}'. But that type has already been registered or a type with a similar id has already been added. The type that was already added is: '{}'. If these types do not match, that means that you has a id collision.", type_name, item.type_name);
                                } else {
                                    panic!("The type you are trying to register has already been registered.");
                                }
                            }

                            let temp_component = ContainerComponent {
                                create_or_clone: CreateOrClone::Empty,
                                unique_id,
                                #[cfg(debug_assertions)]
                                type_name: "",
                                dynamic_id: None,
                            };
                            let old_value = mem::replace(item, temp_component);
                            *value = SingleOrList::List(vec![old_value, component]);
                        },
                        SingleOrList::List(items) => {
                            // if the item is already a list then that means the type is dynamic and it can be added
                            items.push(component);
                        },
                        SingleOrList::Empty => {
                            unreachable!("Should never happen as this is only used for the iterator.");
                        }
                    }
                },
                // If no item has been added then add it.
                Entry::Vacant(v) => {
                    v.insert(SingleOrList::Single(component));
                }
            }
        }
    }

    pub fn get<T : ?Sized + 'static>(&self, injector: &Injector) -> Option<InjectorLock<T>> {
        self.get_underlying(TypeId::of::<T>(), type_name::<T>())
            .map(|x| {
                InjectorLock {
                    value: x
                        .first()
                        .create_or_clone
                        .create_or_clone::<T>(injector),
                }
            })
    }

    pub fn get_list<'a, T : ?Sized  + 'static>(&'a self, injector: &'a Injector) -> DependencyIter<'a, T> {
        self.get_underlying(TypeId::of::<T>(), type_name::<T>())
            .map(|x| DependencyIter {
                iterator: x.iter(),
                injector,
                phantom: PhantomData,
            })
            .unwrap_or(DependencyIter {
                injector,
                iterator: SingleOrListIterator {
                    items: &SingleOrList::Empty,
                    position: 0,
                },
                phantom: PhantomData,
            })
    }

    pub fn get_instance_cell(&self, type_id: TypeId, injector: &Injector) -> Option<InstanceCellLock> {
        self.get_underlying(type_id, "")
            .map(|x| {
                InstanceCellLock {
                    value: x.first()
                        .create_or_clone
                        .get_instance_cell(injector)
                }
            })
    }

    pub fn get_by_id<T: ?Sized + 'static>(&self, type_id: TypeId, injector: &Injector) -> Option<InjectorLock<T>> {
        self.get_underlying_mapped(TypeId::of::<T>(), type_name::<T>(), |component| 
            component
                .and_then(|component| {
                    match component {
                        SingleOrList::Single(item) => if item.dynamic_id == Some(type_id) {
                            Some(item)
                        } else {
                            None
                        },
                        SingleOrList::List(list) => list
                            .iter()
                            .find(|item| item.dynamic_id == Some(type_id)),
                        SingleOrList::Empty => None,
                    }
                })
        )
        .map(|component| {
            InjectorLock {
                value: component
                    .create_or_clone
                    .create_or_clone::<T>(injector),
            }
        })
    }

    fn get_underlying(&self, type_id: TypeId, type_name: &'static str) -> CircularDependencyGuard<'_, Option<&SingleOrList<ContainerComponent>>> {
        self.get_underlying_mapped(type_id, type_name, std::convert::identity)
    }

    fn get_underlying_mapped<'a, F : FnOnce(Option<&'a SingleOrList<ContainerComponent>>) -> T, T>(&'a self, type_id: TypeId, #[allow(unused)] type_name: &'static str, mapper: F) -> CircularDependencyGuard<'a, T> {
        let component = self.components
            .get(&type_id);

        #[cfg(debug_assertions)]
        {
            let error = {
                let mut chain = self.current_dependency_chain.lock()
                    .unwrap();

                if chain.contains(&type_id) {
                    let mut names = self.current_dependency_chain_names.lock()
                        .unwrap();
                    let error = self.create_circular_dependency_error(&mut chain, &mut names, type_id, type_name);
                    Some(error)
                } else {
                    chain.push(type_id);
                    let mut names = self.current_dependency_chain_names.lock()
                        .unwrap();
                    names.push(type_name);
                    None
                }
            };

            if let Some(error) = error {
                panic!("{}", error);
            }
        }

        CircularDependencyGuard {
            value: mapper(component),
            id: type_id,
            container: self
        }
    }

    #[allow(unused)]
    fn create_circular_dependency_error(&self, dependencies: &mut Vec<TypeId>, names: &mut Vec<&'static str>, type_id: TypeId, type_name: &'static str) -> String {
        let mut str = String::with_capacity(256);

        str.write_fmt(format_args!("Circular dependency detected when trying to get: '{}'\n", type_name))
            .unwrap();

        dependencies.push(type_id);
        names.push(type_name);

        for (i, dependency) in names.windows(2).enumerate() {
            let left = &dependency[0];
            let right = &dependency[1];

            str.write_fmt(format_args!("\n{}the instance '{}' is trying to get -> '{}'", if i != 0 { "which is trying to get " } else { "" }, left, right))
                .unwrap();
        }

        str
    }
}

#[derive(Clone, Eq, PartialEq, Debug)]
enum SingleOrList<T> {
    Single(T),
    List(Vec<T>),
    Empty
}

impl<T> SingleOrList<T> {
    pub fn first(&self) -> &T {
        match self {
            SingleOrList::Single(item) => item,
            SingleOrList::List(items) => &items[0],
            SingleOrList::Empty => panic!("No item in list"),
        }
    }
}

impl<T> SingleOrList<T> {
    pub fn iter(&self) -> SingleOrListIterator<'_, T> {
        SingleOrListIterator {
            items: self,
            position: 0,
        }
    }
}

struct SingleOrListIterator<'a, T> {
    items: &'a SingleOrList<T>,
    position: usize,
}

impl<'a, T> Iterator for SingleOrListIterator<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        let item = match self.items {
            SingleOrList::Single(item) => {
                if self.position != 0 {
                    None
                } else {
                    Some(item)
                }
            },
            SingleOrList::List(list) => {
                let item = list.get(self.position);

                item
            },
            SingleOrList::Empty => None,
        };
        
        self.position += 1;
        item
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match self.items {
            SingleOrList::Single(_) => (1, Some(1)),
            SingleOrList::List(x) => (x.len(), Some(x.len())),
            SingleOrList::Empty => (0, None)
        }
    }
}

pub struct DependencyIter<'a, T : ?Sized> {
    iterator: SingleOrListIterator<'a, ContainerComponent>,
    injector: &'a Injector,
    phantom: PhantomData<T>
}

impl<'a, T : 'static + ?Sized> Iterator for DependencyIter<'a, T> {
    type Item = InjectorLock<T>;
    
    fn next(&mut self) -> Option<Self::Item> {
        let next = self.iterator.next();
        
        next
            .map(|x| InjectorLock {
                value: x
                    .create_or_clone
                    .create_or_clone::<T>(self.injector),
            })
    }
    
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iterator.size_hint()
    }
}

pub(crate) struct ContainerComponent {
    create_or_clone: CreateOrClone,

    unique_id: TypeId,
    
    #[cfg(debug_assertions)]
    type_name: &'static str,

    /// The type id of the underlying type. None if it is not a dyn type.
    dynamic_id: Option<TypeId>,
}

impl Into<Vec<ContainerComponent>> for Component {
    fn into(self) -> Vec<ContainerComponent> {
        let mut components = Vec::with_capacity(1 + self.dynamics.len());

        components
            .push(ContainerComponent {
                create_or_clone: match self.lifetime {
                    ComponentLifetime::Singleton => CreateOrClone::Singleton(CreateOrCloneSingleton::new(self.create_func)),
                    ComponentLifetime::Transient => CreateOrClone::Transient(CreateOrCloneTransient::new(self.create_func))
                },
                dynamic_id: None,
                unique_id: self.unique_id,
                #[cfg(debug_assertions)]
                type_name: self.type_name,
            });
        
        for dynamic_component in self.dynamics {
            components.push(ContainerComponent {
                create_or_clone: match self.lifetime {
                    ComponentLifetime::Singleton => CreateOrClone::Singleton(CreateOrCloneSingleton::new(dynamic_component.create_func)),
                    ComponentLifetime::Transient => CreateOrClone::Transient(CreateOrCloneTransient::new(dynamic_component.create_func))
                },
                dynamic_id: Some(self.unique_id),
                unique_id: dynamic_component.unique_id,
                #[cfg(debug_assertions)]
                type_name: dynamic_component.type_name,
            });
        }
        
        components
    }
}

enum CreateOrClone {
    Singleton(CreateOrCloneSingleton),
    Transient(CreateOrCloneTransient),
    Empty,
}

struct CreateOrCloneSingleton {
    func: ComponentCreateFunction,
    value: RwLock<Option<InstanceCell>>,
}

struct CreateOrCloneTransient {
    func: ComponentCreateFunction,
}

impl CreateOrClone {
    pub fn create_or_clone<T : ?Sized + 'static>(&self, injector: &Injector) -> Arc<crate::sync::LockOrCell<T>> {
        match self {
            CreateOrClone::Singleton(item) =>
                item.create_or_clone(injector),
            CreateOrClone::Transient(item) =>
                item.create_or_clone(injector),
            CreateOrClone::Empty => unreachable!("Empty create or clone type should never be used"),
        }
    }
    
    pub fn get_instance_cell(&self, injector: &Injector) -> InstanceCell {
        match self {
            CreateOrClone::Singleton(item) =>
                item.create_or_clone_any(injector),
            CreateOrClone::Transient(item) =>
                item.create_or_clone_any(injector),
            CreateOrClone::Empty => unreachable!("Empty create or clone type should never be used"),
        }
    }
}

impl CreateOrCloneSingleton {
    pub fn new(func: ComponentCreateFunction) -> Self {
        Self { func, value: RwLock::new(None) }
    }

    pub fn create_or_clone<T : ?Sized + 'static>(&self, injector: &Injector) -> Arc<crate::sync::LockOrCell<T>> {
        let value = self.create_or_clone_any(&injector);
        value.get()
    }

    pub fn create_or_clone_any(&self, injector: &Injector) -> InstanceCell {
        {
            let lock = self.value.read()
                .unwrap(); // lock should never be poisoned
            if let Some(value) = lock.as_ref() {
                return value.clone();
            }
        }

        let func_value = self.func.call(&injector);
        let clone = func_value.clone();
        {
            let mut guard = self.value.write()
                .unwrap(); // lock should never be poisoned
            *guard = Some(clone);
        }

        func_value
    }
}

impl CreateOrCloneTransient {
    pub fn new(func: ComponentCreateFunction) -> Self {
        Self { func }
    }

    pub fn create_or_clone<T : ?Sized + 'static>(&self, injector: &Injector) -> Arc<crate::sync::LockOrCell<T>> {
        let value = self.create_or_clone_any(&injector);
        value.get()
    }

    pub fn create_or_clone_any(&self, injector: &Injector) -> InstanceCell {
        let func_value = self.func.call(&injector);
        func_value
    }
}

#[allow(unused)]
pub(crate) struct CircularDependencyGuard<'a, T> {
    container: &'a DIContainer,
    value: T,
    id: TypeId,
}

#[cfg(debug_assertions)]
impl<T> Drop for CircularDependencyGuard<'_, T> {
    fn drop(&mut self) {
        let mut deps = self.container.current_dependency_chain.lock()
            .unwrap();

        let index = deps.iter().position(|x| x == &self.id);
        debug_assert!(index.is_some());
        deps.remove(index.unwrap());
        
        let mut names = self.container.current_dependency_chain_names.lock()
            .unwrap();
        
        names.remove(index.unwrap());
    }
}

impl<T> Deref for CircularDependencyGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T> DerefMut for CircularDependencyGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}