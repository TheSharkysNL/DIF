#[allow(unused)]
use std::any::{type_name, Any, TypeId};
use std::ptr::{drop_in_place, NonNull};
use std::mem::ManuallyDrop;
use std::ops::{Deref, DerefMut};
use crate::sync::Lock;

type DynAny = dyn Any + Send + Sync;

unsafe fn drop_instance<T: ?Sized + 'static, L: Lock>(
    value: &mut L::Lock<DynAny>,
    trait_vtable: Option<NonNull<()>>,
) {
    match trait_vtable {
        Some(vtable) => {
            // Safety: using correct vtable for vtable is guaranteed by the caller
            let mut coerced = unsafe {
                coerce::<L, DynAny, T>(value.clone(), vtable)
            };
            
            unsafe { drop_in_place::<L::Lock<T>>(&mut coerced); }
        }
        None => {
            // Safety: The erased function is called only with the original lock
            // type, so changing the function pointer's erased signature is valid.
            let drop_fn = unsafe {
                std::mem::transmute::<_, unsafe fn(&mut L::Lock<DynAny>)>(
                    drop_in_place::<L::Lock<T>> as *const (),
                )
            };
            
            unsafe { drop_fn(value); }
        },
    }
}

/// Returns the [`Any`] trait vtable used for type erasure.
///
/// This trait should not be implemented outside this crate.
///
/// # Safety
/// Implementations must return the vtable for the `Any` representation of
/// `Self`. Returning a vtable for another type makes the pointer conversions
/// performed by the injector invalid.
pub unsafe trait AnyMetadata<L: Lock>: 'static {
    /// Retrieves the Any vtable for the type of Self 
    /// and if this is already a trait it also returns the traits vtable
    fn any_vtable(instance: &L::Lock<Self>) -> (NonNull<()>, Option<NonNull<()>>);
}


unsafe impl<L: Lock, T: Any + Sized + 'static> AnyMetadata<L> for T {
    fn any_vtable(_instance: &L::Lock<T>) -> (NonNull<()>, Option<NonNull<()>>) {
        let dangling: *const T = std::ptr::NonNull::dangling().as_ptr();
        // Safety: LockBound<T> enforces the bounds required by the lock, and
        // the pointer is immediately used only to obtain the Any vtable.
        let RawFatPtr { vtable, .. } = unsafe {
            std::mem::transmute::<*const DynAny, RawFatPtr>(dangling as *const () as *const DynAny)
        };
        
        // Safety: vtable should always be nonnull
        (unsafe { NonNull::new_unchecked(vtable as *mut ()) }, None)
    }
}

#[derive(Clone, Copy)]
pub(crate) struct ThreadSafePtr<T : ?Sized>(NonNull<T>);

impl<T : ?Sized> ThreadSafePtr<T> {
    /// Safety: 
    /// Only safe is the pointer is only read. 
    /// The underlying data never moves.
    /// It doesn't point at thread local storage.
    pub unsafe fn new(ptr: NonNull<T>) -> Self {
        Self(ptr)
    }
}

/// Safety: 
/// see [`Self::new`]
unsafe impl<T : ?Sized> Send for ThreadSafePtr<T> {}

/// Safety: 
/// see [`Self::new`]
unsafe impl<T : ?Sized> Sync for ThreadSafePtr<T> {}

/// Contains an erased dependency instance.
/// It can be downcast with [`Self::get::<T>()`].
pub(crate) struct InstanceCell<L : Lock> {
    type_id: TypeId,
    instance: ManuallyDrop<L::Lock<DynAny>>,
    _drop: unsafe fn(&mut L::Lock<DynAny>, Option<NonNull<()>>),
    trait_vtable: Option<ThreadSafePtr<()>>
}

impl<L : Lock> Clone for InstanceCell<L> {
    fn clone(&self) -> Self {
        Self {
            type_id: self.type_id.clone(),
            instance: ManuallyDrop::new(self.instance.deref().clone()),
            _drop: self._drop,
            trait_vtable: self.trait_vtable.clone()
        }
    }
}

impl<L : Lock> InstanceCell<L> {
    pub(crate) fn new<T>(instance: L::Lock<T>) -> Self
        where
            T: ?Sized + AnyMetadata<L> + 'static,
    {
        let (any_vtable, trait_vtable) = T::any_vtable(&instance);
        
        // Safety: T::any_vtable supplies the vtable for this exact type.
        let erased = unsafe { coerce::<L, T, DynAny>(instance, any_vtable) };

        let trait_vtable = trait_vtable
            .map(|vtable| 
                // Safety: vtable is only read never written to.
                // vtables are baked into the binary so they never move.
                unsafe { ThreadSafePtr::new(vtable) }
            );
        
        InstanceCell {
            type_id: TypeId::of::<T>(),
            instance: ManuallyDrop::new(erased),
            _drop: drop_instance::<T, L>,
            trait_vtable
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
            match self.trait_vtable {
                Some(ref trait_vtable) => Some(coerce::<L, DynAny, T>(value.deref().clone(), trait_vtable.0)),
                None => Some(from_any::<T, L>(value).clone())
            }
        }
    }
    
    /// Checks if the underlying value has the type of `T`.
    pub fn is<T : ?Sized + 'static>(&self) -> bool {
        self.type_id == TypeId::of::<T>()
    }
}

/// Only need drop here as other references to the underlying Arc or Rc type
/// should have the correct type information. 
impl<L : Lock> Drop for InstanceCell<L> {
    fn drop(&mut self) {
        // Safety: self.instance retains its original L::Lock<T> type, and
        // self._drop was created for that same type.
        unsafe {
            (self._drop)(self.instance.deref_mut(), self.trait_vtable.map(|x| x.0))
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


/// A view into dyn fat pointers
#[doc(hidden)]
#[repr(C)]
pub struct RawFatPtr {
    pub data: *const (),
    pub vtable: *const (),
}

/// Used to coerce Sized types of [`T`] into Unsized types of [`U`]
///
/// # Safety
/// `vtable` must be the correct vtable for coercing the pointee for `T` into
/// the pointee for `U`.
#[doc(hidden)]
pub unsafe fn coerce<L: Lock, T : ?Sized, U: ?Sized>(
    lock: L::Lock<T>,
    vtable: NonNull<()>,
) -> L::Lock<U> {
    let raw = L::into_raw(lock);

    let fat = RawFatPtr { data: raw as *const (), vtable: vtable.as_ptr() };
    // Safety: T can be coerced to U and the caller supplied the matching vtable.
    let dyn_ptr: *const L::Pointee<U> = unsafe { std::mem::transmute_copy(&fat) };

    unsafe { L::from_raw(dyn_ptr) }
}