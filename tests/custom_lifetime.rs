use std::cell::Cell;
use std::sync::atomic::{AtomicU32, Ordering};
use dilian_core::{ComponentLifetimeChecker, Injector};
use dilian_core::sync::Lock;

thread_local! {
    static CUSTOM_LIFETIME_SCOPE: Cell<u32> = Cell::new(1);
}

#[derive(Debug)]
pub struct CustomLifetime {
    scope: AtomicU32,
}

impl<L : Lock> ComponentLifetimeChecker<L> for CustomLifetime {
    fn needs_new_instance(&self, _: &Injector<L>) -> bool {
        CUSTOM_LIFETIME_SCOPE.with(|scope| {
            let current_scope = scope.get();
            if current_scope == self.scope.load(Ordering::SeqCst) {
                false
            } else {
                self.scope.store(current_scope, Ordering::SeqCst);
                true
            }
        })
    }
}

impl Clone for CustomLifetime {
    fn clone(&self) -> Self {
        Self {
            scope: AtomicU32::new(self.scope.load(Ordering::SeqCst)),
        }
    }
}

impl CustomLifetime {
    pub fn new() -> Self {
        Self {
            scope: AtomicU32::new(0),
        }
    }
    
    pub fn update_scope() {
        CUSTOM_LIFETIME_SCOPE.with(|scope| scope.set(scope.get() + 1));
    }

    pub fn reset_scope() {
        CUSTOM_LIFETIME_SCOPE.with(|scope| scope.set(0));
    }
}