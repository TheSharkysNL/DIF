use crate::cell::InstanceCell;
use std::cell::RefCell as StdRefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex as StdMutex, RwLock as StdRwLock};

/// A common interface for the lock types supported by the injector.
pub trait Lock : Default {
    /// The lock type used to own and access a value of type `T`.
    type Lock<T : ?Sized> : Clone;
    
    /// The pointee type used when coercing `T` into a dynamic type.
    type Pointee<T : ?Sized>: ?Sized;
    
    /// Creates a new lock containing `value`.
    fn new<T>(value: T) -> Self::Lock<T>;

    /// Decomposes the lock into a raw pointer without changing its refcount.
    fn into_raw<T: ?Sized>(lock: Self::Lock<T>) -> *const Self::Pointee<T>;
    
    /// Reconstructs a lock from a raw pointer to the same allocation.
    ///
    /// # Safety
    /// `point` must have been returned by [`Self::into_raw`] for the same lock
    /// implementation and type, and ownership of that raw pointer must be
    /// transferred to this call exactly once.
    unsafe fn from_raw<T: ?Sized>(point: *const Self::Pointee<T>) -> Self::Lock<T>;

    /// Creates a raw, non-owning view of the underlying [`Self::Pointee`].
    fn as_raw<T: ?Sized>(lock: &Self::Lock<T>) -> *const Self::Pointee<T>;
}

/// A mutex lock using [`Arc<StdMutex<T>>`] under the hood.
#[derive(Default, Debug)]
pub struct Mutex;
impl Lock for Mutex {
    type Lock<T: ?Sized> = Arc<StdMutex<T>>;
    type Pointee<T: ?Sized> = StdMutex<T>;
    
    fn new<T>(value: T) -> Self::Lock<T> {
        Arc::new(StdMutex::new(value))
    }
    
    fn into_raw<T: ?Sized>(lock: Self::Lock<T>) -> *const Self::Pointee<T> {
        Arc::into_raw(lock)
    }
    
    unsafe fn from_raw<T: ?Sized>(point: *const Self::Pointee<T>) -> Self::Lock<T> {
        unsafe {
            Arc::from_raw(point)
        }
    }
    
    fn as_raw<T: ?Sized>(lock: &Self::Lock<T>) -> *const Self::Pointee<T> {
        Arc::as_ptr(lock)
    }
}

/// A refcell lock using [`Rc<StdRefCell<T>>`] under the hood.
#[derive(Default, Debug)]
pub struct RefCell;
impl Lock for RefCell {
    type Lock<T: ?Sized> = Rc<StdRefCell<T>>;
    type Pointee<T: ?Sized> = StdRefCell<T>;

    fn new<T>(value: T) -> Self::Lock<T> {
        Rc::new(StdRefCell::new(value))
    }
    
    fn into_raw<T: ?Sized>(lock: Self::Lock<T>) -> *const Self::Pointee<T> {
        Rc::into_raw(lock)
    }
    
    unsafe fn from_raw<T: ?Sized>(point: *const Self::Pointee<T>) -> Self::Lock<T> {
        unsafe {
            Rc::from_raw(point)
        }
    }
    
    fn as_raw<T: ?Sized>(lock: &Self::Lock<T>) -> *const Self::Pointee<T> {
        Rc::as_ptr(lock)
    }
}

/// A read-write lock using [`Arc<StdRwLock<T>>`] under the hood.
#[derive(Default, Debug)]
pub struct RwLock;
impl Lock for RwLock {
    type Lock<T: ?Sized> = Arc<StdRwLock<T>>;
    type Pointee<T: ?Sized> = StdRwLock<T>;
    
    fn new<T>(value: T) -> Self::Lock<T> {
        Arc::new(StdRwLock::new(value))
    }
    
    fn into_raw<T: ?Sized>(lock: Self::Lock<T>) -> *const Self::Pointee<T> {
        Arc::into_raw(lock)
    }
    
    unsafe fn from_raw<T: ?Sized>(point: *const Self::Pointee<T>) -> Self::Lock<T> {
        unsafe {
            Arc::from_raw(point)
        }
    }
    
    fn as_raw<T: ?Sized>(lock: &Self::Lock<T>) -> *const Self::Pointee<T> {
        Arc::as_ptr(lock)
    }
}

/// An async mutex lock using [`Arc<tokio::sync::Mutex<T>>`] under the hood.
#[cfg(any(feature = "async"))]
#[cfg_attr(feature = "async", derive(Default, Debug))]
#[cfg_attr(doc, doc(cfg(feature = "async")))]
pub struct AsyncMutex;
#[cfg(feature = "async")]
impl Lock for AsyncMutex {
    type Lock<T: ?Sized> = Arc<tokio::sync::Mutex<T>>;
    type Pointee<T: ?Sized> = tokio::sync::Mutex<T>;

    fn new<T>(value: T) -> Self::Lock<T> {
        Arc::new(tokio::sync::Mutex::new(value))
    }
    
    fn into_raw<T: ?Sized>(lock: Self::Lock<T>) -> *const Self::Pointee<T> {
        Arc::into_raw(lock)
    }
    
    unsafe fn from_raw<T: ?Sized>(point: *const Self::Pointee<T>) -> Self::Lock<T> {
        unsafe {
            Arc::from_raw(point)
        }
    }
    
    fn as_raw<T: ?Sized>(lock: &Self::Lock<T>) -> *const Self::Pointee<T> {
        Arc::as_ptr(lock)
    }
}

/// An async read-write lock using [`Arc<tokio::sync::RwLock<T>>`] under the hood
#[cfg(any(feature = "async"))]
#[cfg_attr(feature = "async", derive(Default, Debug))]
#[cfg_attr(doc, doc(cfg(feature = "async")))]
pub struct AsyncRwLock;
#[cfg(feature = "async")]
impl Lock for AsyncRwLock {
    type Lock<T: ?Sized> = Arc<tokio::sync::RwLock<T>>;
    type Pointee<T: ?Sized> = tokio::sync::RwLock<T>;
    
    fn new<T>(value: T) -> Self::Lock<T> {
        Arc::new(tokio::sync::RwLock::new(value))
    }
    
    fn into_raw<T: ?Sized>(lock: Self::Lock<T>) -> *const Self::Pointee<T> {
        Arc::into_raw(lock)
    }
    
    unsafe fn from_raw<T: ?Sized>(point: *const Self::Pointee<T>) -> Self::Lock<T> {
        unsafe {
            Arc::from_raw(point)
        }
    }
    
    fn as_raw<T: ?Sized>(lock: &Self::Lock<T>) -> *const Self::Pointee<T> {
        Arc::as_ptr(lock)
    }
}

/// A marker trait used for forcing [`Send`] and/or [`Sync`] on a type based on the lock.
/// 
/// # Safety
/// Implementations must enforce the [`Send`] and [`Sync`] bounds required by
/// the lock type. A lock must not allow a value to cross a thread boundary
/// unless that value is safe to send or share there.
pub unsafe trait LockBound<T: ?Sized> {}


unsafe impl<T: ?Sized + Send> LockBound<T> for Mutex {}
unsafe impl<T: ?Sized + Send + Sync> LockBound<T> for RwLock {}
unsafe impl<T: ?Sized> LockBound<T> for RefCell {}

#[cfg(feature = "async")]
unsafe impl<T: ?Sized + Send> LockBound<T> for AsyncMutex {}
#[cfg(feature = "async")]
unsafe impl<T: ?Sized + Send + Sync> LockBound<T> for AsyncRwLock {}

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
    vtable: *const (),
) -> L::Lock<U> {
    let raw = L::into_raw(lock);

    let fat = RawFatPtr { data: raw as *const (), vtable };
    // Safety: T can be coerced to U and the caller supplied the matching vtable.
    let dyn_ptr: *const L::Pointee<U> = unsafe { std::mem::transmute_copy(&fat) };
    
    unsafe { L::from_raw(dyn_ptr) }
}

/// Used for generic locking of the different [`Lock`] implementations.
pub trait Lockable<T : ?Sized> {
    fn read(&self) -> impl std::ops::Deref<Target = T> + '_;
    
    fn write(&self) -> impl std::ops::DerefMut<Target = T>  + '_;
}

impl<T : ?Sized> Lockable<T> for Arc<StdMutex<T>> {
    fn read(&self) -> impl std::ops::Deref<Target = T> + '_ {
        self.lock().unwrap()
    }
    
    fn write(&self) -> impl std::ops::DerefMut<Target = T> + '_ {
        self.lock().unwrap()
    }
}

impl<T : ?Sized> Lockable<T> for Arc<StdRwLock<T>> {
    fn read(&self) -> impl std::ops::Deref<Target = T> + '_ {
        StdRwLock::read(self).unwrap()
    }

    fn write(&self) -> impl std::ops::DerefMut<Target = T> + '_ {
        StdRwLock::write(self).unwrap()
    }
}

impl<T : ?Sized> Lockable<T> for Rc<StdRefCell<T>> {
    fn read(&self) -> impl std::ops::Deref<Target = T> + '_ {
        StdRefCell::borrow(self)
    }

    fn write(&self) -> impl std::ops::DerefMut<Target = T> + '_ {
        StdRefCell::borrow_mut(self)
    }
}

/// An asynchronous variant of the [`Lockable`] trait.
///
/// Provides generic access to the supported [`Lock`] implementations in
/// asynchronous code.
#[cfg(feature = "async")]
pub trait AsyncLockable<T : ?Sized> {
    fn read(&self) -> impl Future<Output = impl std::ops::Deref<Target = T> + '_> + Send + Sync + '_;
    
    fn write(&self) -> impl Future<Output = impl std::ops::DerefMut<Target = T> + '_> + Send + Sync + '_;
}

#[cfg(feature = "async")]
impl<T : ?Sized + Send> AsyncLockable<T> for Arc<tokio::sync::Mutex<T>> {
    fn read(&self) -> impl Future<Output = impl std::ops::Deref<Target = T> + '_> + Send + Sync + '_ {
        self.lock()
    }
    
    fn write(&self) -> impl Future<Output = impl std::ops::DerefMut<Target = T> + '_> + Send + Sync + '_{
        self.lock()
    }
} 

#[cfg(feature = "async")]
impl<T : ?Sized + Send + Sync> AsyncLockable<T> for Arc<tokio::sync::RwLock<T>> {
    fn read(&self) -> impl Future<Output = impl std::ops::Deref<Target = T> + '_> + Send + Sync + '_ {
        tokio::sync::RwLock::read(self)
    }
    
    fn write(&self) -> impl Future<Output = impl std::ops::DerefMut<Target = T> + '_> + Send + Sync + '_ {
        tokio::sync::RwLock::write(self)
    }
}

/// A lock containing an [`InstanceCell`] that can hold any type.
#[derive(Clone)]
pub struct InstanceCellLock<L : Lock> {
    pub(crate) value: InstanceCell<L>,
}

impl<L : Lock> InstanceCellLock<L> {
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
        self.value.get()
    }

    /// Checks if the underlying value has the type of `T`.
    pub fn is<T : ?Sized + 'static>(&self) -> bool {
        self.value.is::<T>()
    }
}