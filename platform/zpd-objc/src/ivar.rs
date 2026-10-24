use std::ffi::CStr;
use std::marker::PhantomData;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::rc::Rc;

use crate::ffi::Id;
use crate::{get_pointer_ivar, set_pointer_ivar};

/// A `ptr` ivar that owns one `Rc<T>` reference for its Objective-C object.
///
/// Reads return a cloned `Rc`, so a callback keeps its state alive even when
/// it synchronously clears the ivar or deallocates the receiver.
pub struct RcIvar<T> {
    name: &'static CStr,
    // A function marker keeps the name usable in statics; access is unsafe and single-threaded.
    _state: PhantomData<fn() -> Rc<T>>,
}

impl<T: 'static> RcIvar<T> {
    /// Names an ivar declared as `ptr` with `decl!`.
    pub const fn new(name: &'static CStr) -> Self {
        Self {
            name,
            _state: PhantomData,
        }
    }

    /// Stores `value`, releasing any previously stored state.
    ///
    /// # Safety
    /// `object` must be live, its class must declare this ivar as `ptr`, and
    /// the ivar must only be accessed through this `RcIvar` on one thread.
    pub unsafe fn set(&self, object: Id, value: Rc<T>) {
        drop(unsafe { self.take(object) });
        // SAFETY: The caller guarantees pointer-sized storage for this ivar.
        unsafe { set_pointer_ivar(object, self.name.as_ptr(), Rc::into_raw(value).cast_mut()) };
    }

    /// Returns a new reference to the stored state.
    ///
    /// # Safety
    /// Same contract as [`RcIvar::set`].
    pub unsafe fn get(&self, object: Id) -> Option<Rc<T>> {
        // SAFETY: The caller guarantees pointer-sized storage for this ivar.
        let raw = unsafe { get_pointer_ivar::<T>(object, self.name.as_ptr()) };
        if raw.is_null() {
            return None;
        }
        // SAFETY: A non-null value is the reference stored by `set`.
        unsafe {
            Rc::increment_strong_count(raw);
            Some(Rc::from_raw(raw))
        }
    }

    /// Clears the ivar and returns its owned reference. Repeated calls return `None`.
    ///
    /// # Safety
    /// Same contract as [`RcIvar::set`].
    pub unsafe fn take(&self, object: Id) -> Option<Rc<T>> {
        // SAFETY: The caller guarantees pointer-sized storage for this ivar.
        let raw = unsafe { get_pointer_ivar::<T>(object, self.name.as_ptr()) };
        if raw.is_null() {
            return None;
        }
        // SAFETY: Clearing first transfers the stored reference to Rust exactly once.
        unsafe {
            set_pointer_ivar(object, self.name.as_ptr(), std::ptr::null_mut::<T>());
            Some(Rc::from_raw(raw))
        }
    }
}

/// Runs Rust code called from Objective-C, aborting instead of unwinding across FFI.
pub fn abort_on_unwind<R>(operation: impl FnOnce() -> R) -> R {
    catch_unwind(AssertUnwindSafe(operation)).unwrap_or_else(|_| std::process::abort())
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;
    use std::sync::OnceLock;

    use super::RcIvar;
    use crate::ffi::{Id, Sel};
    use crate::{Strong, class, decl, msg_send, msg_send_super, sel};

    struct DropProbe(Rc<Cell<usize>>);
    impl Drop for DropProbe {
        fn drop(&mut self) {
            self.0.set(self.0.get() + 1);
        }
    }

    static STATE: RcIvar<DropProbe> = RcIvar::new(c"_state");

    unsafe extern "C" fn dealloc(object: Id, _: Sel) {
        drop(unsafe { STATE.take(object) });
        let (superclass, selector) = (class!("NSObject"), sel!("dealloc"));
        // SAFETY: NSObject's dealloc completes destruction of this receiver.
        unsafe { msg_send_super!(object, superclass, selector, () => ()) };
    }

    fn object() -> Strong {
        static CLASS: OnceLock<usize> = OnceLock::new();
        let class = *CLASS.get_or_init(|| {
            decl!(ZpdObjcRcIvarTest: [class!("NSObject")] {
                fields { _state: ptr }
                methods { "dealloc": "v@:" => dealloc }
            }) as usize
        }) as crate::ffi::Class;
        let (alloc, init) = (sel!("alloc"), sel!("init"));
        // SAFETY: The test class inherits NSObject's alloc/init.
        unsafe {
            let allocated = msg_send!(class, alloc, () => Id);
            Strong::from_retained(msg_send!(allocated, init, () => Id)).unwrap()
        }
    }

    #[test]
    fn replacement_and_take_release_each_value_once() {
        // Verifies set replaces old state, get shares it, and take releases it exactly once.
        let drops = Rc::new(Cell::new(0));
        let object = object();
        // SAFETY: The test class declares `_state` as a ptr ivar.
        unsafe {
            STATE.set(object.as_ptr(), Rc::new(DropProbe(drops.clone())));
            STATE.set(object.as_ptr(), Rc::new(DropProbe(drops.clone())));
            assert_eq!(drops.get(), 1);
            let shared = STATE.get(object.as_ptr()).unwrap();
            drop(STATE.take(object.as_ptr()));
            assert_eq!(drops.get(), 1);
            drop(shared);
            assert_eq!(drops.get(), 2);
            assert!(STATE.take(object.as_ptr()).is_none());
        }
    }

    #[test]
    fn dealloc_releases_remaining_state() {
        // Verifies an object that still owns state releases it when deallocated.
        let drops = Rc::new(Cell::new(0));
        let object = object();
        // SAFETY: The test class declares `_state` as a ptr ivar.
        unsafe { STATE.set(object.as_ptr(), Rc::new(DropProbe(drops.clone()))) };
        drop(object);
        assert_eq!(drops.get(), 1);
    }
}
