use crate::cell::InstanceCell;
use std::cell::RefCell as StdRefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex as StdMutex, RwLock as StdRwLock};

/// Generic trait for lock types
pub trait Lock : Default {
    #[cfg(not(feature = "test-utils"))]
    type Lock<T : ?Sized> : Clone;

    #[cfg(feature = "test-utils")]
    type Lock<T : ?Sized> : Clone + Lockable<T>;
    
    type Pointee<T : ?Sized>: ?Sized;
    
    /// Initialize a new lock instance
    fn new<T>(value: T) -> Self::Lock<T>;

    /// Decompose into the raw pointee pointer. Must not touch the refcount.
    fn into_raw<T: ?Sized>(lock: Self::Lock<T>) -> *const Self::Pointee<T>;
    
    /// Reconstruct from a raw pointee pointer of the same allocation.
    /// 
    /// Safety: Callee must ensure that pointer comes from the [`Self::into_raw`] function.
    unsafe fn from_raw<T: ?Sized>(point: *const Self::Pointee<T>) -> Self::Lock<T>;
}

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
}

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
}

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
}

#[cfg(feature = "async")]
#[cfg_attr(feature = "async", derive(Default, Debug))]
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
}

#[cfg(feature = "async")]
#[cfg_attr(feature = "async", derive(Default, Debug))]
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
}

#[doc(hidden)]
#[repr(C)]
pub struct RawFatPtr {
    pub data: *const (),
    pub vtable: *const (),
}

#[doc(hidden)]
pub unsafe fn coerce<L: Lock, T, U: ?Sized>(
    lock: L::Lock<T>,
    vtable: *const (),
) -> L::Lock<U> {
    let raw = L::into_raw(lock);

    let fat = RawFatPtr { data: raw as *const (), vtable };
    let dyn_ptr: *const L::Pointee<U> = unsafe { std::mem::transmute_copy(&fat) };
    
    unsafe { L::from_raw(dyn_ptr) }
}

#[cfg(feature = "test-utils")]
pub trait Lockable<T : ?Sized> {
    fn read(&self) -> impl std::ops::Deref<Target = T> + '_;
    
    fn write(&self) -> impl std::ops::DerefMut<Target = T> + std::ops::Deref<Target = T>  + '_;
}

#[cfg(feature = "test-utils")]
impl<T : ?Sized> Lockable<T> for Arc<StdMutex<T>> {
    fn read(&self) -> impl std::ops::Deref<Target = T> + '_ {
        self.lock().unwrap()
    }
    
    fn write(&self) -> impl std::ops::DerefMut<Target = T> + '_ {
        self.lock().unwrap()
    }
}

#[cfg(feature = "test-utils")]
impl<T : ?Sized> Lockable<T> for Arc<StdRwLock<T>> {
    fn read(&self) -> impl std::ops::Deref<Target = T> + '_ {
        StdRwLock::read(self).unwrap()
    }

    fn write(&self) -> impl std::ops::DerefMut<Target = T> + '_ {
        StdRwLock::write(self).unwrap()
    }
}

#[cfg(feature = "test-utils")]
impl<T : ?Sized> Lockable<T> for Rc<StdRefCell<T>> {
    fn read(&self) -> impl std::ops::Deref<Target = T> + '_ {
        StdRefCell::borrow(self)
    }

    fn write(&self) -> impl std::ops::DerefMut<Target = T> + '_ {
        StdRefCell::borrow_mut(self)
    }
}

/// Lock containing an InstanceCell which can contain any type.
#[derive(Clone)]
pub struct InstanceCellLock<L : Lock> {
    pub(crate) value: InstanceCell<L>,
}

impl<L : Lock> InstanceCellLock<L> {
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
        self.value.get()
    }

    /// Checks if the underlying value has the type of `T`.
    pub fn is<T : ?Sized + 'static>(&self) -> bool {
        self.value.is::<T>()
    }
}