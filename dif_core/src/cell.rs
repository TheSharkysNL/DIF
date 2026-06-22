use std::any::Any;
use std::mem;
use std::ptr::drop_in_place;
use std::sync::Arc;
use crate::sync::LockOrCell;
use std::mem::ManuallyDrop;

#[derive(Clone)]
pub struct InstanceCell {
    instance: ManuallyDrop<Arc<LockOrCell<dyn Any + Send + Sync>>>,
    _drop: unsafe fn(&mut Arc<LockOrCell<dyn Any + Send + Sync>>),
}

impl InstanceCell {
    pub fn new<T : ?Sized>(instance: Arc<LockOrCell<T>>) -> Self {
        // Safety: The drop in place fn should be safe to transmute here 
        // as we are always passing through a &mut Arc<LockOrCell<T>> 
        // but those bits were transmuted to act like &mut Arc<LockOrCell<dyn Any + Send + Sync>> using the into_any function
        // but passing data through as a reference makes the callee responsible for the type
        // this way we can safely drop the T type, while not holding a generic type reference to T
        unsafe {
            let _drop_fn = mem::transmute::<_, unsafe fn(&mut Arc<LockOrCell<dyn Any + Send + Sync>>)>(drop_in_place::<Arc<LockOrCell<T>>> as *const ());
            let instance = ManuallyDrop::new(into_any(&instance).clone());
            InstanceCell {
                instance,
                _drop: _drop_fn
            }
        }
    }
    
    pub fn get<T : ?Sized>(&self) -> Arc<LockOrCell<T>> {
        let value = &self.instance;
        unsafe {
            from_any(value).clone()
        }
    }
}

impl Drop for InstanceCell {
    fn drop(&mut self) {
        unsafe {
            (self._drop)(&mut self.instance)
        }
    }
}

unsafe fn from_any<'a, T : ?Sized>(value: &'a Arc<LockOrCell<dyn Any + Send + Sync>>) -> &'a Arc<LockOrCell<T>> {
    let any_ptr = value as *const Arc<LockOrCell<dyn Any + Send + Sync>>;
    let real_ptr = any_ptr as *const Arc<LockOrCell<T>>;
    unsafe { &*real_ptr }
}

unsafe fn into_any<'a, T : ?Sized>(value: &'a Arc<LockOrCell<T>>) -> &'a Arc<LockOrCell<dyn Any + Send + Sync>> {
    let real_ptr = value as *const Arc<LockOrCell<T>>;
    let any_ptr = real_ptr as *const Arc<LockOrCell<dyn Any + Send + Sync>>;
    unsafe { &*any_ptr }
}