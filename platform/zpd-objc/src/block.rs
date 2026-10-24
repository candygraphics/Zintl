//! Minimal Objective-C block literals for one-argument completion handlers.

use std::cell::RefCell;
use std::ffi::c_void;
use std::rc::Rc;

use crate::ffi::Id;

/// The block can be copied with helper functions (`BLOCK_HAS_COPY_DISPOSE`).
const BLOCK_HAS_COPY_DISPOSE: i32 = 1 << 25;

unsafe extern "C" {
    static _NSConcreteStackBlock: [*const c_void; 32];
}

type Closure<A> = RefCell<Box<dyn FnMut(A)>>;

#[repr(C)]
struct Descriptor {
    reserved: usize,
    size: usize,
    copy: unsafe extern "C" fn(*mut c_void, *const c_void),
    dispose: unsafe extern "C" fn(*const c_void),
}

/// A stack block literal with the `void (^)(A)` signature.
///
/// The runtime copies it to the heap when the callee retains it. Each copy
/// shares the Rust closure through an `Rc`, so the closure lives until the
/// last copy is released.
#[repr(C)]
pub struct StackBlock<A: Copy + 'static> {
    isa: *const c_void,
    flags: i32,
    reserved: i32,
    invoke: unsafe extern "C" fn(*mut StackBlock<A>, A),
    descriptor: *const Descriptor,
    closure: *const Closure<A>,
}

impl<A: Copy + 'static> StackBlock<A> {
    const DESCRIPTOR: Descriptor = Descriptor {
        reserved: 0,
        size: std::mem::size_of::<Self>(),
        copy: Self::copy,
        dispose: Self::dispose,
    };

    /// Wraps `closure` in a block that may be passed to Objective-C APIs.
    pub fn new(closure: impl FnMut(A) + 'static) -> Self {
        let closure: Rc<Closure<A>> = Rc::new(RefCell::new(Box::new(closure)));
        Self {
            // SAFETY: Only the address of the runtime's stack block class is taken.
            isa: unsafe { _NSConcreteStackBlock.as_ptr().cast() },
            flags: BLOCK_HAS_COPY_DISPOSE,
            reserved: 0,
            invoke: Self::invoke,
            descriptor: &Self::DESCRIPTOR,
            closure: Rc::into_raw(closure),
        }
    }

    /// Returns the block pointer for an Objective-C message argument.
    pub fn as_id(&self) -> Id {
        (self as *const Self).cast_mut().cast()
    }

    unsafe extern "C" fn invoke(block: *mut Self, argument: A) {
        // SAFETY: The runtime passes a live copy of this block literal.
        let closure = unsafe { (*block).closure };
        crate::abort_on_unwind(|| {
            // SAFETY: Every live copy owns one strong reference to the closure.
            let closure = unsafe { &*closure };
            let Ok(mut callback) = closure.try_borrow_mut() else {
                std::process::abort()
            };
            callback(argument);
        });
    }

    unsafe extern "C" fn copy(destination: *mut c_void, source: *const c_void) {
        // SAFETY: The runtime has copied the literal bytes; the heap copy needs its own reference.
        unsafe {
            let source = source.cast::<Self>();
            let destination = destination.cast::<Self>();
            Rc::increment_strong_count((*source).closure);
            (*destination).closure = (*source).closure;
        }
    }

    unsafe extern "C" fn dispose(block: *const c_void) {
        // SAFETY: The runtime disposes each heap copy once, releasing its reference.
        unsafe { drop(Rc::from_raw((*block.cast::<Self>()).closure)) };
    }
}

impl<A: Copy + 'static> Drop for StackBlock<A> {
    fn drop(&mut self) {
        // SAFETY: The stack literal owns the reference created in `new`.
        unsafe { drop(Rc::from_raw(self.closure)) };
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use super::*;

    unsafe extern "C" {
        fn _Block_copy(block: *const c_void) -> *mut c_void;
        fn _Block_release(block: *const c_void);
    }

    #[test]
    fn heap_copy_keeps_the_closure_until_released() {
        // Verifies a runtime copy can run after the stack literal drops, then frees the closure.
        let calls = Rc::new(Cell::new(0_i64));
        let received = calls.clone();
        let block = StackBlock::new(move |value: i64| received.set(received.get() + value));
        // SAFETY: The literal is a valid block; the runtime returns a +1 heap copy.
        let copy = unsafe { _Block_copy(block.as_id().cast()) };
        drop(block);
        assert_eq!(Rc::strong_count(&calls), 2);

        // SAFETY: The heap copy uses the same layout and invoke signature.
        unsafe {
            let literal = copy.cast::<StackBlock<i64>>();
            ((*literal).invoke)(literal, 5);
            _Block_release(copy);
        }

        assert_eq!(calls.get(), 5);
        assert_eq!(Rc::strong_count(&calls), 1);
    }
}
