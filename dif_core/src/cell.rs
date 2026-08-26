#[allow(unused)]
use std::any::{type_name, Any, TypeId};
use std::mem;
use std::ptr::drop_in_place;
use std::mem::ManuallyDrop;
use std::ops::DerefMut;
use crate::sync::Lock;

type DynAny = dyn Any + Send + Sync;

/// Contains an instance of a dependency. 
/// Can be downcast to a type using the [`Self::get::<T>()`] function.
pub(crate) struct InstanceCell<L : Lock> {
    type_id: TypeId,
    instance: ManuallyDrop<L::Lock<DynAny>>,
    _drop: unsafe fn(&mut L::Lock<DynAny>),
}

impl<L : Lock> Clone for InstanceCell<L> {
    fn clone(&self) -> Self {
        Self {
            type_id: self.type_id.clone(),
            instance: self.instance.clone(),
            _drop: self._drop.clone(),
        }
    }
}

impl<L : Lock> InstanceCell<L> {
    pub(crate) fn new<T : ?Sized + 'static>(instance: L::Lock<T>) -> Self {
        // Safety: The drop in place fn should be safe to transmute here 
        // as we are always passing through a &mut L::Lock<T>
        // but those bits were transmuted to act like &mut L::Lock<T> using the into_any function
        // but passing data through as a reference makes the callee responsible for the type
        // this way we can safely drop the T type, while not holding a generic type reference to T
        unsafe {
            let _drop_fn = mem::transmute::<_, unsafe fn(&mut L::Lock<DynAny>)>(drop_in_place::<L::Lock<T>> as *const ());
            
            let instance = ManuallyDrop::new(into_any::<T, L>(&instance).clone());
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
    pub fn get<T : ?Sized + 'static>(&self) -> Option<L::Lock<T>> {
        if !self.is::<T>() {
            return None;
        }
        
        let value = &self.instance;
        unsafe {
            Some(from_any::<T, L>(value).clone())
        }
    }
    
    /// Checks if the underlying value has the type of `T`.
    pub fn is<T : ?Sized + 'static>(&self) -> bool {
        self.type_id == TypeId::of::<T>()
    }
}

impl<L : Lock> Drop for InstanceCell<L> {
    fn drop(&mut self) {
        unsafe {
            (self._drop)(self.instance.deref_mut())
        }
    }
}

unsafe fn from_any<'a, T : ?Sized, L : Lock>(value: &'a L::Lock<DynAny>) -> &'a L::Lock<T> {
    let any_ptr = value as *const L::Lock<DynAny>;
    let real_ptr = any_ptr as *const L::Lock<T>;
    unsafe { &*real_ptr }
}

unsafe fn into_any<'a, T : ?Sized, L : Lock>(value: &'a L::Lock<T>) -> &'a L::Lock<DynAny> {
    let real_ptr = value as *const L::Lock<T>;
    let any_ptr = real_ptr as *const L::Lock<DynAny>;
    unsafe { &*any_ptr }
}