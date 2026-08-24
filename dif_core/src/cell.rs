#[allow(unused)]
use std::any::{type_name, Any, TypeId};
use std::mem;
use std::ptr::drop_in_place;
use std::sync::Arc;
use crate::sync::{LockOrCell, SendTrait};
use std::mem::ManuallyDrop;

#[cfg(any(feature = "multithreaded", feature = "async"))]
type DynAny = dyn Any + Send;

#[cfg(not(any(feature = "multithreaded", feature = "async")))]
type DynAny = dyn Any;

/// Contains an instance of a dependency. 
/// Can be downcast to a type using the `.get::<T>()` function.
#[derive(Clone)]
pub(crate) struct InstanceCell {
    type_id: TypeId,
    instance: ManuallyDrop<Arc<LockOrCell<DynAny>>>,
    _drop: unsafe fn(&mut Arc<LockOrCell<DynAny>>),
}

impl InstanceCell {
    pub(crate) fn new<T : ?Sized + 'static + SendTrait>(instance: Arc<LockOrCell<T>>) -> Self {
        // Safety: The drop in place fn should be safe to transmute here 
        // as we are always passing through a &mut Arc<LockOrCell<T>> 
        // but those bits were transmuted to act like &mut Arc<LockOrCell<DynAny>> using the into_any function
        // but passing data through as a reference makes the callee responsible for the type
        // this way we can safely drop the T type, while not holding a generic type reference to T
        unsafe {
            let _drop_fn = mem::transmute::<_, unsafe fn(&mut Arc<LockOrCell<DynAny>>)>(drop_in_place::<Arc<LockOrCell<T>>> as *const ());
            let instance = ManuallyDrop::new(into_any(&instance).clone());
            InstanceCell {
                type_id: TypeId::of::<T>(),
                instance,
                _drop: _drop_fn
            }
        }
    }
    
    /// Downcasts the instance to the type of `T` if possible else it will return a None value.
    /// 
    /// # Examples
    /// ```rust
    /// // create injector
    /// let mut injector = Injector::new();
    ///     
    /// injector.singleton::<StdLogger>();
    ///     
    /// // get logger
    /// let logger = injector.get_any(TypeId::of::<StdLogger>());
    ///     
    /// assert!(logger.is_some());
    /// let logger = logger.unwrap();
    /// // downcast
    /// let logger = logger.get::<StdLogger>();
    /// ```
    pub fn get<T : ?Sized + 'static>(&self) -> Option<Arc<LockOrCell<T>>> {
        if !self.is::<T>() {
            return None;
        }
        
        let value = &self.instance;
        unsafe {
            Some(from_any(value).clone())
        }
    }
    
    /// Checks if the underlying value has the type of `T`.
    pub fn is<T : ?Sized + 'static>(&self) -> bool {
        self.type_id == TypeId::of::<T>()
    }
}

impl Drop for InstanceCell {
    fn drop(&mut self) {
        unsafe {
            (self._drop)(&mut self.instance)
        }
    }
}

unsafe fn from_any<'a, T : ?Sized>(value: &'a Arc<LockOrCell<DynAny>>) -> &'a Arc<LockOrCell<T>> {
    let any_ptr = value as *const Arc<LockOrCell<DynAny>>;
    let real_ptr = any_ptr as *const Arc<LockOrCell<T>>;
    unsafe { &*real_ptr }
}

unsafe fn into_any<'a, T : ?Sized>(value: &'a Arc<LockOrCell<T>>) -> &'a Arc<LockOrCell<DynAny>> {
    let real_ptr = value as *const Arc<LockOrCell<T>>;
    let any_ptr = real_ptr as *const Arc<LockOrCell<DynAny>>;
    unsafe { &*any_ptr }
}