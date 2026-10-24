use std::cell::RefCell;
use std::rc::Rc;
use std::sync::OnceLock;

use zpd_objc::{Id, RcIvar, Strong};

type Callback = RefCell<Box<dyn FnMut(Id)>>;

static CALLBACK: RcIvar<Callback> = RcIvar::new(c"_zpdCallback");

unsafe extern "C" fn invoke(object: Id, _: zpd_objc::Sel, sender: Id) {
    // SAFETY: This method is installed only on targets with a `_zpdCallback` ivar.
    let Some(callback) = (unsafe { CALLBACK.get(object) }) else {
        return;
    };
    zpd_objc::abort_on_unwind(|| {
        let Ok(mut invoke) = callback.try_borrow_mut() else {
            std::process::abort()
        };
        invoke(sender);
    });
}

unsafe extern "C" fn dealloc(object: Id, _: zpd_objc::Sel) {
    // SAFETY: The receiver is a target created by `target`.
    unsafe { release(object) };
    // SAFETY: NSObject's dealloc completes destruction after Rust state is released.
    unsafe {
        zpd_objc::msg_send_super!(object, zpd_objc::class!("NSObject"), zpd_objc::sel!("dealloc"), () => ())
    };
}

pub(crate) unsafe fn release(object: Id) {
    // SAFETY: The caller passes a target created by `target`.
    drop(unsafe { CALLBACK.take(object) });
}

fn target_class() -> zpd_objc::Class {
    static CLASS: OnceLock<usize> = OnceLock::new();
    *CLASS.get_or_init(|| {
        zpd_objc::decl!(ZpdRustActionTarget: [zpd_objc::class!("NSObject")] {
            fields { _zpdCallback: ptr }
            methods {
                "invoke:": "v@:@" => invoke,
                "controlTextDidChange:": "v@:@" => invoke,
                "dealloc": "v@:" => dealloc,
            }
        }) as usize
    }) as zpd_objc::Class
}

pub(crate) fn target(callback: impl FnMut(Id) + 'static) -> Strong {
    let target = crate::native::alloc_init(target_class());
    // SAFETY: The registered target class declares the `_zpdCallback` ivar.
    unsafe {
        CALLBACK.set(
            target.as_ptr(),
            Rc::new(RefCell::new(Box::new(callback) as Box<dyn FnMut(Id)>)),
        );
    }
    target
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use crate::native;

    struct DropProbe(Rc<Cell<usize>>);
    impl Drop for DropProbe {
        fn drop(&mut self) {
            self.0.set(self.0.get() + 1);
        }
    }

    #[test]
    fn target_releases_callback_state_once() {
        // Verifies replacing or destroying a native target releases its Rust closure exactly once.
        let drops = Rc::new(Cell::new(0));
        let probe = DropProbe(drops.clone());
        let target = super::target(move |_| {
            let _ = &probe;
        });
        drop(target);
        assert_eq!(drops.get(), 1);
    }

    #[test]
    fn actor_owned_target_replacement_releases_each_callback_once() {
        // Verifies Actor attachment replacement drops old and current callback state once each.
        let tree = crate::actor::ActorTree::new(native::alloc_init(zpd_objc::class!("NSObject")));
        let owner = tree.insert_root(native::alloc_init(zpd_objc::class!("NSObject")));
        let drops = Rc::new(Cell::new(0));
        let first = DropProbe(drops.clone());
        tree.replace_owned(
            &owner,
            "target",
            super::target(move |_| {
                let _ = &first;
            }),
        )
        .unwrap();
        let second = DropProbe(drops.clone());
        tree.replace_owned(
            &owner,
            "target",
            super::target(move |_| {
                let _ = &second;
            }),
        )
        .unwrap();
        assert_eq!(drops.get(), 1);
        tree.clear_owned(&owner, "target").unwrap();
        assert_eq!(drops.get(), 2);
    }
}
