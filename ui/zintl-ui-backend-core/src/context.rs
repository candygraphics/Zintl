use std::cell::RefCell;
use std::sync::Arc;

use zintl_ui::element::IntoElement;
use zintl_ui::store::Store;
use zintl_ui::view::{Context as ContextTrait, StoreContext, StoreWatcher};

use crate::MainTaskSender;

/// Gives views access to Stores and to the platform main thread.
pub struct Context<'a> {
    store_context: StoreContext<'a>,
    perform_main: Option<Arc<MainTaskSender>>,
}

impl<'a> Context<'a> {
    #[doc(hidden)]
    pub fn new(store_context: StoreContext<'a>, perform_main: Option<Arc<MainTaskSender>>) -> Self {
        Self {
            store_context,
            perform_main,
        }
    }

    /// Queues a task for execution on the platform main thread.
    ///
    /// # Panics
    /// Panics when the active backend has no main-thread sender or its message
    /// loop no longer accepts work.
    pub fn perform_main(&self, task: impl FnOnce() + Send + 'static) {
        assert!(
            self.main_sender()(Box::new(task)),
            "the main-thread message loop is closed"
        );
    }

    /// Returns a handle that writes `store` from any thread.
    ///
    /// # Panics
    /// Panics when the active backend has no main-thread sender.
    pub fn sender<T: Send + 'static>(&self, store: Store<T>) -> StoreSender<T> {
        StoreSender {
            store,
            sender: self.main_sender().clone(),
        }
    }

    fn main_sender(&self) -> &Arc<MainTaskSender> {
        self.perform_main
            .as_ref()
            .expect("the active backend does not provide a main-thread sender")
    }

    pub fn store<T: 'static>(&mut self, value: T) -> Store<T> {
        self.store_context.store(value)
    }

    pub fn get<T: 'static>(&self, store: Store<T>) -> &T {
        self.store_context.get(store)
    }

    pub fn watch<T, F, E>(&self, store: Store<T>, render: F) -> StoreWatcher<T, F, E>
    where
        T: 'static,
        F: Fn(&T) -> E + 'static,
        E: IntoElement + 'static,
    {
        self.store_context.watch(store, render)
    }

    pub fn update<T: 'static, U>(
        &mut self,
        store: Store<T>,
        update: impl FnOnce(&mut T) -> U,
    ) -> U {
        self.store_context.update(store, update)
    }
}

impl<'a> ContextTrait<'a> for Context<'a> {
    fn store_context<'context>(&'context self) -> &'context StoreContext<'a> {
        &self.store_context
    }

    fn store_context_mut<'context>(&'context mut self) -> &'context mut StoreContext<'a> {
        &mut self.store_context
    }
}

/// A Store update queued from another thread.
pub type StoreUpdate = Box<dyn for<'a> FnOnce(&mut Context<'a>)>;

thread_local! {
    static STORE_UPDATES: RefCell<Vec<StoreUpdate>> = const { RefCell::new(Vec::new()) };
}

/// Writes a Store from any thread; the write applies on the main thread.
///
/// Backends apply queued writes with [`StoreUpdates::take`] after
/// running main-thread tasks, then render the views that read the Store.
pub struct StoreSender<T: 'static> {
    store: Store<T>,
    sender: Arc<MainTaskSender>,
}

impl<T: 'static> Clone for StoreSender<T> {
    fn clone(&self) -> Self {
        Self {
            store: self.store,
            sender: self.sender.clone(),
        }
    }
}

impl<T: Send + 'static> StoreSender<T> {
    /// Queues replacing the value; returns `false` when the main thread no longer accepts work.
    pub fn set(&self, value: T) -> bool {
        self.update(move |current| *current = value)
    }

    /// Queues `update` to run on the value on the main thread.
    pub fn update(&self, update: impl FnOnce(&mut T) + Send + 'static) -> bool {
        let store = self.store;
        (self.sender)(Box::new(move || {
            STORE_UPDATES.with(|updates| {
                updates
                    .borrow_mut()
                    .push(Box::new(move |cx: &mut Context<'_>| {
                        cx.update(store, update)
                    }))
            });
        }))
    }
}

/// The Store writes [`StoreSender`]s queued to this thread.
pub struct StoreUpdates;

impl StoreUpdates {
    /// Removes the queued writes, oldest first.
    pub fn take() -> Vec<StoreUpdate> {
        STORE_UPDATES.with(|updates| std::mem::take(&mut *updates.borrow_mut()))
    }
}
