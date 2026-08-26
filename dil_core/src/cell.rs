#[allow(unused)]
use std::any::{type_name, Any, TypeId};
use std::mem;
use std::ptr::drop_in_place;
use std::mem::ManuallyDrop;
use std::ops::DerefMut;
use crate::sync::{coerce, Lock, RawFatPtr};

type DynAny = dyn Any + Send + Sync;

/// Gets the [`Any`] trait vtable used for coercing under the hood. 
/// Should not be implemented for your own types.
/// 
/// Safety: can be unsafe if implementation returns an incorrect vtable.
pub unsafe trait AnyMetadata<L: Lock>: 'static {
    fn any_vtable(instance: &L::Lock<Self>) -> *const ();
}


unsafe impl<L: Lock, T: Any + Sized + 'static> AnyMetadata<L> for T {
    fn any_vtable(_instance: &L::Lock<T>) -> *const () {
        let dangling: *const T = std::ptr::NonNull::dangling().as_ptr();
        // Safety: Can convert to a Send + Sync type here as this is enforced by the LockBound<T> trait.
        // Safety: The type of *const DynAny can be converted into a RawFatPtr to get the vtable for the type.
        let RawFatPtr { vtable, .. } = unsafe {
            std::mem::transmute::<*const DynAny, RawFatPtr>(dangling as *const () as *const DynAny)
        };
        vtable
    }
}

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
    pub(crate) fn new<T>(instance: L::Lock<T>) -> Self
    where
        T: ?Sized + AnyMetadata<L> + 'static,
    {
        let vtable = T::any_vtable(&instance);

        // Safety: Only the function signature changes. See drop function for more details.
        let drop_fn = unsafe {
            mem::transmute::<_, unsafe fn(&mut L::Lock<DynAny>)>(
                drop_in_place::<L::Lock<T>> as *const (),
            )
        };
        // Safety: got the correct vtable via the T::any_vtable function.
        let erased = unsafe { coerce::<L, T, DynAny>(instance, vtable) };

        InstanceCell {
            type_id: TypeId::of::<T>(),
            instance: ManuallyDrop::new(erased),
            _drop: drop_fn,
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
        // Safety: Checked if the actual underlying type 
        // of the lock is L::Lock<T> using the self.is function.
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
        // Safety: As under the hood the self.instance value 
        // is actually still the type of L::Lock<T> and the self._drop function
        // points to a function which accepts L::Lock<T> this is valid.
        // 
        unsafe {
            (self._drop)(self.instance.deref_mut())
        }
    }
}

/// converting a lock from a DynAny lock into the type T
/// 
/// Safety: If the actual underlying type of the lock is L::Lock<T> then this is safe.
unsafe fn from_any<'a, T : ?Sized, L : Lock>(value: &'a L::Lock<DynAny>) -> &'a L::Lock<T> {
    let any_ptr = value as *const L::Lock<DynAny>;
    let real_ptr = any_ptr as *const L::Lock<T>;
    unsafe { &*real_ptr }
}