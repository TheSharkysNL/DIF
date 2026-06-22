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

pub struct InjectorLock<T : ?Sized> {
    pub(crate) value: Arc<LockOrCell<T>>,
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

pub struct InstanceCellLock {
    pub(crate) value: InstanceCell,
}

impl InstanceCellLock {
    /// Will return an error if the mutex has been poisoned.
    #[cfg(feature = "async")]
    pub async fn lock<T : ?Sized + 'static, F : FnOnce(&mut T) -> Fut, O, Fut : Future<Output = O>>(&self, fun: F) -> O {
        let value = self.value.get::<T>();
        let mut guard = value.lock().await;
        fun(guard.deref_mut()).await
    }

    /// Will return an error if the mutex has been poisoned.
    #[cfg(all(feature = "multithreaded", not(feature = "async")))]
    pub fn lock<T : ?Sized + 'static, F : FnOnce(&mut T) -> O, O>(&self, fun: F) -> std::result::Result<O, String> {
        let value = self.value.get::<T>();
        match value.lock() {
            Ok(mut guard) => Ok(fun(guard.deref_mut())),
            Err(err) => Err(format!("Error, mutex has been poisoned. {}", err)),
        }
    }

    #[cfg(not(feature = "multithreaded"))]
    pub fn borrow<T : ?Sized + 'static, F : FnOnce(&T) -> O, O>(&self, fun: F) -> O {
        let value = self.value.get::<T>();
        let borrow = value.borrow();
        fun(borrow.deref())
    }

    #[cfg(not(feature = "multithreaded"))]
    pub fn borrow_mut<T : ?Sized + 'static, F : FnOnce(&mut T) -> O, O>(&self, fun: F) -> O {
        let value = self.value.get::<T>();
        let mut borrow = value.borrow_mut();
        fun(borrow.deref_mut())
    }
}