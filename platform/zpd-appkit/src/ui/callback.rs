use std::cell::RefCell;
use std::rc::Rc;
use std::sync::OnceLock;

use zpd_objc::{Id, RcIvar, Strong};

use crate::actor::{ActorRef, WindowEventKind};

use super::UiError;

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

/// Forwards two-argument delegate messages, such as `tabView:didSelectTabViewItem:`.
unsafe extern "C" fn invoke_first(object: Id, selector: zpd_objc::Sel, first: Id, _: Id) {
    // SAFETY: The receiver is a target created by `target`; only the first argument is used.
    unsafe { invoke(object, selector, first) };
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
                "comboBoxSelectionDidChange:": "v@:@" => invoke,
                "popoverDidClose:": "v@:@" => invoke,
                "windowWillClose:": "v@:@" => invoke,
                "tabView:didSelectTabViewItem:": "v@:@@" => invoke_first,
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

/// Routes a control's target-action messages to `handler` until its Actor is removed.
pub(crate) fn install_action(
    actor: &ActorRef,
    handler: impl FnMut(Id) + 'static,
) -> Result<(), UiError> {
    let tree = actor.tree_handle().ok_or(UiError::Closed)?;
    let target = tree
        .replace_owned(actor, "action-target", target(handler))
        .map_err(UiError::from)?;
    // SAFETY: The owned Actor holds a target created by `target`.
    tree.add_teardown(&target, |target| unsafe { release(target) })
        .map_err(UiError::from)?;
    actor.with(|control| {
        target.with(|target| {
            // SAFETY: The control is a live NSControl-like object that retains neither argument.
            unsafe {
                zpd_objc::msg_send!(control, zpd_objc::sel!("setTarget:"), ((target): Id) => ());
                zpd_objc::msg_send!(control, zpd_objc::sel!("setAction:"), ((zpd_objc::sel!("invoke:")): zpd_objc::Sel) => ());
            }
        })
    })??;
    tree.add_teardown(actor, |control| {
        // SAFETY: Detach the unretained target before the owned target is released.
        unsafe {
            zpd_objc::msg_send!(control, zpd_objc::sel!("setTarget:"), ((zpd_objc::NIL): Id) => ());
            zpd_objc::msg_send!(control, zpd_objc::sel!("setAction:"), ((zpd_objc::NIL): Id) => ());
        }
    })
    .map_err(UiError::from)
}

/// Installs a delegate whose supported notifications call `handler` with their argument.
pub(crate) fn install_delegate(
    actor: &ActorRef,
    handler: impl FnMut(Id) + 'static,
) -> Result<(), UiError> {
    let tree = actor.tree_handle().ok_or(UiError::Closed)?;
    let delegate = tree
        .replace_owned(actor, "delegate", target(handler))
        .map_err(UiError::from)?;
    // SAFETY: The owned Actor holds a target created by `target`.
    tree.add_teardown(&delegate, |delegate| unsafe { release(delegate) })
        .map_err(UiError::from)?;
    actor.with(|object| {
        delegate.with(|delegate| {
            // SAFETY: AppKit delegates are unretained; teardown clears this one first.
            unsafe {
                zpd_objc::msg_send!(object, zpd_objc::sel!("setDelegate:"), ((delegate): Id) => ())
            }
        })
    })??;
    tree.add_teardown(actor, |object| {
        // SAFETY: Detach the unretained delegate before the owned delegate is released.
        unsafe {
            zpd_objc::msg_send!(object, zpd_objc::sel!("setDelegate:"), ((zpd_objc::NIL): Id) => ())
        }
    })
    .map_err(UiError::from)
}

/// Routes `name` notifications posted by `object` to `handler` until `actor` is removed.
///
/// `object` must stay alive while `actor` is live, such as a view it retains.
pub(crate) fn install_observer(
    actor: &ActorRef,
    name: &str,
    object: Id,
    handler: impl FnMut(Id) + 'static,
) -> Result<(), UiError> {
    let tree = actor.tree_handle().ok_or(UiError::Closed)?;
    let observer = tree
        .replace_owned(actor, "observer", target(handler))
        .map_err(UiError::from)?;
    tree.add_teardown(&observer, |observer| {
        // SAFETY: Unregister the unretained observer before its callback state is released.
        unsafe {
            let center = zpd_objc::msg_send!(zpd_objc::class!("NSNotificationCenter"), zpd_objc::sel!("defaultCenter"), () => Id);
            zpd_objc::msg_send!(center, zpd_objc::sel!("removeObserver:"), ((observer): Id) => ());
            release(observer);
        }
    })
    .map_err(UiError::from)?;
    let name = crate::native::nsstring(name);
    observer.with(|observer| {
        // SAFETY: The center keeps the observer unretained; teardown removes it first.
        unsafe {
            let center = zpd_objc::msg_send!(zpd_objc::class!("NSNotificationCenter"), zpd_objc::sel!("defaultCenter"), () => Id);
            zpd_objc::msg_send!(center, zpd_objc::sel!("addObserver:selector:name:object:"), ((observer): Id, (zpd_objc::sel!("invoke:")): zpd_objc::Sel, (name.as_ptr()): Id, (object): Id) => ());
        }
    })?;
    Ok(())
}

/// Returns a function that emits semantic events from `actor` while it is live.
pub(crate) fn emitter(actor: &ActorRef) -> impl Fn(WindowEventKind) + 'static {
    let actor = actor.clone();
    move |kind| {
        if let Some(tree) = actor.tree_handle() {
            tree.emit(&actor, kind);
        }
    }
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
