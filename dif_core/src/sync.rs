use std::ops::{Deref, DerefMut};
use std::sync::{Arc};

#[cfg(all(feature = "multithreaded", not(feature = "async")))]
use std::sync::{LockResult, PoisonError};



#[cfg(feature = "async")]
pub use async_std::sync::{Mutex as LockOrCell, MutexGuard as Guard};

#[cfg(all(feature = "multithreaded", not(feature = "async")))]
pub use std::sync::{Mutex as LockOrCell, MutexGuard as Guard};

#[cfg(not(feature = "multithreaded"))]
pub use std::cell::{RefCell as LockOrCell, Ref as Guard, RefMut as GuardMut};

use crate::cell::InstanceCell;

/// A lock containing the dependency that was injected. 
/// Lock can change based on specific feature flags.
#[derive(Debug)]
pub struct InjectorLock<T : ?Sized> {
    pub(crate) value: Arc<LockOrCell<T>>,
}

impl<T : ?Sized> Clone for InjectorLock<T> {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
        }
    }
}

impl<T : ?Sized> InjectorLock<T> {
    /// Locks the value asynchronously
    #[cfg(feature = "async")]
    pub async fn lock(&self) -> InjectorLockGuard<'_, T> {
        let guard = self.value.lock().await;
        InjectorLockGuard { guard }
    }

    /// Locks the value. If the lock is poisoned an error result will be returned.
    #[cfg(all(feature = "multithreaded", not(feature = "async")))]
    pub fn lock(&self) -> LockResult<InjectorLockGuard<'_, T>> {
        self.value.lock()
            .map(|guard|
                InjectorLockGuard { guard })
            .map_err(|err| 
                PoisonError::new(InjectorLockGuard { guard: err.into_inner() }))
    }
    
    /// Borrows the value. If any the value is borrowed mutably this will panic.
    #[cfg(not(feature = "multithreaded"))]
    pub fn borrow(&self) -> InjectorLockGuard<'_, T> {
        let borrow = self.value.borrow();
        InjectorLockGuard { guard: borrow }
    }
    
    /// Borrows the value mutably. If the value is already borrowed this will panic.
    #[cfg(not(feature = "multithreaded"))]
    pub fn borrow_mut(&self) -> InjectorLockGuardMut<'_, T> {
        let borrow = self.value.borrow_mut();
        InjectorLockGuardMut { guard: borrow }
    }
}

pub struct InjectorLockGuard<'a, T : ?Sized> {
    guard: Guard<'a, T>,
}

impl<'a, T : 'static + ?Sized> Deref for InjectorLockGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.guard.deref()
    }
}

#[cfg(feature = "multithreaded")]
impl<'a, T : 'static + ?Sized> DerefMut for InjectorLockGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.guard.deref_mut()
    }
}

#[cfg(not(feature = "multithreaded"))]
pub struct InjectorLockGuardMut<'a, T : ?Sized> {
    guard: GuardMut<'a, T>,
}

#[cfg(not(feature = "multithreaded"))]
impl<'a, T : 'static + ?Sized> Deref for InjectorLockGuardMut<'a, T> {
    type Target = T;
    
    fn deref(&self) -> &Self::Target {
        self.guard.deref()
    }
}

#[cfg(not(feature = "multithreaded"))]
impl<'a, T : 'static + ?Sized> DerefMut for InjectorLockGuardMut<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.guard.deref_mut()
    }
}

/// Lock containing an InstanceCell which can contain any type.
#[derive(Clone)]
pub struct InstanceCellLock {
    pub(crate) value: InstanceCell,
}

impl InstanceCellLock {
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
    pub fn get<T : ?Sized + 'static>(&self) -> Option<InjectorLock<T>> {
        self.value.get()
            .map(|value| {
                InjectorLock {
                    value
                }
            })
    }
}

#[cfg(any(feature = "multithreaded", feature = "async"))]
pub trait SendTrait : Send {}

#[cfg(not(any(feature = "multithreaded", feature = "async")))]
pub trait SendTrait {}

#[cfg(any(feature = "multithreaded", feature = "async"))]
impl<T : Send + ?Sized> SendTrait for T {}

#[cfg(not(any(feature = "multithreaded", feature = "async")))]
impl<T : ?Sized> SendTrait for T {}