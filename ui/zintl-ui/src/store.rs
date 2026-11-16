use crate::hook::{Hook, HookId};
use std::marker::PhantomData;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct StoreId {
    pub(crate) sequence_id: usize,
    pub(crate) index: usize,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct StoreHandle {
    pub(crate) id: StoreId,
    pub(crate) hook_id: HookId,
}

/// A typed handle to state owned by a [`crate::composer::Composer`].
///
/// A Store may be declared before it is attached to a Composer. Initialize it
/// from [`crate::view::View::init`] with [`crate::view::Context::store`] before
/// reading, watching, or updating it.
#[derive(Debug, PartialEq, Eq)]
pub struct Store<T: 'static> {
    handle: Option<StoreHandle>,
    phantom: PhantomData<fn() -> T>,
}

impl<T: 'static> Copy for Store<T> {}

impl<T: 'static> Clone for Store<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: 'static> Store<T> {
    /// Creates a Store declaration that is not yet attached to a Composer.
    pub const fn uninitialized() -> Self {
        Self {
            handle: None,
            phantom: PhantomData,
        }
    }

    /// Returns whether this Store has been initialized by a Context.
    pub const fn is_initialized(&self) -> bool {
        self.handle.is_some()
    }

    pub(crate) fn new(id: StoreId, hook_id: HookId) -> Self {
        Self {
            handle: Some(StoreHandle { id, hook_id }),
            phantom: PhantomData,
        }
    }

    pub(crate) fn handle(self) -> StoreHandle {
        self.handle
            .expect("Store was used before View::init initialized it")
    }
}

impl<T: 'static> Default for Store<T> {
    fn default() -> Self {
        Self::uninitialized()
    }
}

impl<T: 'static> Hook for Store<T> {
    fn hook_id(&self) -> HookId {
        self.handle
            .expect("Store was used before View::init initialized it")
            .hook_id
    }
}

/// A one-shot command carrying a value, such as "focus this field" or
/// "scroll to this row". Each [`crate::view::Context::fire`] runs it once,
/// even with an equal value.
///
/// Create it in [`crate::view::View::init`] with
/// [`crate::view::Context::trigger`] and pass it to the view that obeys it.
#[derive(Debug, PartialEq, Eq)]
pub struct Trigger<T: 'static = ()>(Store<Option<T>>);

impl<T: 'static> Copy for Trigger<T> {}

impl<T: 'static> Clone for Trigger<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: 'static> Trigger<T> {
    /// Creates a Trigger declaration that is not yet attached to a Composer.
    pub const fn uninitialized() -> Self {
        Self(Store::uninitialized())
    }

    pub(crate) fn new(store: Store<Option<T>>) -> Self {
        Self(store)
    }

    /// Returns the Store behind the trigger, for views that obey it: its
    /// value is the last fired value and its revision counts the fires.
    pub fn store(self) -> Store<Option<T>> {
        self.0
    }
}

impl<T: 'static> Default for Trigger<T> {
    fn default() -> Self {
        Self::uninitialized()
    }
}
