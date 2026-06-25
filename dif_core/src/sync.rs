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
    #[cfg(feature = "async")]
    pub async fn lock(&self) -> InjectorLockGuard<'_, T> {
        let guard = self.value.lock().await;
        InjectorLockGuard { guard }
    }

    #[cfg(all(feature = "multithreaded", not(feature = "async")))]
    pub fn lock(&self) -> LockResult<InjectorLockGuard<'_, T>> {
        self.value.lock()
            .map(|guard|
                InjectorLockGuard { guard })
            .map_err(|err| 
                PoisonError::new(InjectorLockGuard { guard: err.into_inner() }))
    }
    
    #[cfg(not(feature = "multithreaded"))]
    pub fn borrow(&self) -> InjectorLockGuard<'_, T> {
        let borrow = self.value.borrow();
        InjectorLockGuard { guard: borrow }
    }
    
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

#[derive(Clone)]
pub struct InstanceCellLock {
    pub(crate) value: InstanceCell,
}

impl InstanceCellLock {
    pub fn get<T : ?Sized + 'static>(&self) -> InjectorLock<T> {
        InjectorLock {
            value: self.value.get()
        }
    }
}