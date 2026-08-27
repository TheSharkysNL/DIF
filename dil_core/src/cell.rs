#[allow(unused)]
use std::any::{type_name, Any, TypeId};
use std::mem;
use std::ptr::drop_in_place;
use std::mem::ManuallyDrop;
use std::ops::DerefMut;
use crate::sync::{coerce, Lock, RawFatPtr};

type DynAny = dyn Any + Send + Sync;

/// Returns the [`Any`] trait vtable used for type erasure.
///
/// This trait should not be implemented outside this crate.
///
/// # Safety
/// Implementations must return the vtable for the `Any` representation of
/// `Self`. Returning a vtable for another type makes the pointer conversions
/// performed by the injector invalid.
pub unsafe trait AnyMetadata<L: Lock>: 'static {
    fn any_vtable(instance: &L::Lock<Self>) -> *const ();
}


unsafe impl<L: Lock, T: Any + Sized + 'static> AnyMetadata<L> for T {
    fn any_vtable(_instance: &L::Lock<T>) -> *const () {
        let dangling: *const T = std::ptr::NonNull::dangling().as_ptr();
        // Safety: LockBound<T> enforces the bounds required by the lock, and
        // the pointer is immediately used only to obtain the Any vtable.
        let RawFatPtr { vtable, .. } = unsafe {
            std::mem::transmute::<*const DynAny, RawFatPtr>(dangling as *const () as *const DynAny)
        };
        vtable
    }
}

/// Contains an erased dependency instance.
/// It can be downcast with [`Self::get::<T>()`].
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

        // Safety: The erased function is called only with the original lock
        // type, so changing the function pointer's erased signature is valid.
        let drop_fn = unsafe {
            mem::transmute::<_, unsafe fn(&mut L::Lock<DynAny>)>(
                drop_in_place::<L::Lock<T>> as *const (),
            )
        };
        // Safety: T::any_vtable supplies the vtable for this exact type.
        let erased = unsafe { coerce::<L, T, DynAny>(instance, vtable) };

        InstanceCell {
            type_id: TypeId::of::<T>(),
            instance: ManuallyDrop::new(erased),
            _drop: drop_fn,
        }
    }
    
    /// Downcasts the instance to `T`, returning `None` if the types do not match.
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
        // Safety: is() verified that the erased lock contains L::Lock<T>.
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
        // Safety: self.instance retains its original L::Lock<T> type, and
        // self._drop was created for that same type.
        unsafe {
            (self._drop)(self.instance.deref_mut())
        }
    }
}

/// Converts an erased lock back into a lock for `T`.
///
/// # Safety
/// The caller must ensure that the erased lock actually contains `T` and that
/// its allocation and metadata match `L::Lock<T>`.
unsafe fn from_any<'a, T : ?Sized, L : Lock>(value: &'a L::Lock<DynAny>) -> &'a L::Lock<T> {
    let any_ptr = value as *const L::Lock<DynAny>;
    let real_ptr = any_ptr as *const L::Lock<T>;
    unsafe { &*real_ptr }
}