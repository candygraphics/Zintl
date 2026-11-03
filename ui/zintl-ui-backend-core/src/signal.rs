//! Values that canvases share without rebuilding views.

use std::cell::{Cell, RefCell};
use std::collections::BTreeSet;
use std::fmt;
use std::rc::Rc;

use crate::canvas::CanvasRegistry;

thread_local! {
    /// The canvas drawing on this thread, which records the signals it reads.
    static READER: Cell<Option<u64>> = const { Cell::new(None) };
}

/// Marks a canvas as drawing until dropped.
pub(crate) struct Reading {
    previous: Option<u64>,
}

impl Reading {
    pub(crate) fn start(canvas: u64) -> Self {
        Self {
            previous: READER.with(|reader| reader.replace(Some(canvas))),
        }
    }
}

impl Drop for Reading {
    fn drop(&mut self) {
        READER.with(|reader| reader.set(self.previous));
    }
}

struct SignalCell<T> {
    value: RefCell<T>,
    /// Canvases that read the value while drawing.
    readers: RefCell<BTreeSet<u64>>,
}

/// A value that canvases share without rebuilding views, such as a pointer position.
///
/// A canvas that reads the signal while drawing is redrawn whenever the
/// signal is written; views and other canvases are left alone. Clones share
/// the value, so pass one to each handler. Signals live on the main thread.
pub struct Signal<T>(Rc<SignalCell<T>>);

impl<T> Signal<T> {
    pub fn new(value: T) -> Self {
        Self(Rc::new(SignalCell {
            value: RefCell::new(value),
            readers: RefCell::new(BTreeSet::new()),
        }))
    }

    /// Reads the value; while drawing, the canvas redraws when it changes.
    pub fn with<R>(&self, read: impl FnOnce(&T) -> R) -> R {
        if let Some(canvas) = READER.with(Cell::get) {
            self.0.readers.borrow_mut().insert(canvas);
        }
        read(&self.0.value.borrow())
    }

    /// Replaces the value and redraws the canvases that read it.
    pub fn set(&self, value: T) {
        self.update(|current| *current = value);
    }

    /// Changes the value in place and redraws the canvases that read it.
    pub fn update(&self, update: impl FnOnce(&mut T)) {
        update(&mut self.0.value.borrow_mut());
        let readers = self.0.readers.borrow().clone();
        // Canvases that are gone stop being readers.
        let gone = readers
            .into_iter()
            .filter(|canvas| !CanvasRegistry::invalidate(*canvas))
            .collect::<Vec<_>>();
        let mut readers = self.0.readers.borrow_mut();
        for canvas in gone {
            readers.remove(&canvas);
        }
    }
}

impl<T: Clone> Signal<T> {
    /// Returns a copy of the value; while drawing, the canvas redraws when it changes.
    pub fn get(&self) -> T {
        self.with(T::clone)
    }
}

impl<T> Clone for Signal<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

/// Signals are equal when they share one value.
impl<T> PartialEq for Signal<T> {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl<T: fmt::Debug> fmt::Debug for Signal<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("Signal")
            .field(&self.0.value.borrow())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use zintl_ui_graphics::{Graphics2D, Rect};

    use super::*;
    use crate::{
        Canvas2DHandler, CanvasEvent, CanvasEventContext, CanvasHost, CanvasId, CanvasNode, Cursor,
        Frame,
    };

    /// Draws, reading `signal` only when `reads` is set.
    #[derive(Clone)]
    struct Plot {
        signal: Signal<f64>,
        reads: bool,
    }

    impl Canvas2DHandler for Plot {
        fn draw(&mut self, _g: &mut Graphics2D<'_>, _frame: &Frame) {
            if self.reads {
                self.signal.get();
            }
        }

        fn event(&mut self, _event: &CanvasEvent, _cx: &mut CanvasEventContext) -> bool {
            self.signal.set(1.0);
            true
        }
    }

    /// Counts invalidations.
    #[derive(Default)]
    struct Host(Cell<u32>);

    impl CanvasHost for Host {
        fn invalidate(&self) {
            self.0.set(self.0.get() + 1);
        }
        fn set_focus(&self, _focused: bool) {}
        fn set_cursor(&self, _cursor: Cursor) {}
        fn actions_emitted(&self, _canvas: CanvasId) {}
    }

    fn mount(signal: &Signal<f64>, reads: bool) -> (CanvasNode, Rc<Host>) {
        let host = Rc::new(Host::default());
        let canvas = CanvasNode::new(Plot {
            signal: signal.clone(),
            reads,
        });
        canvas.attach(host.clone(), None);
        canvas.record(&Frame {
            bounds: Rect::from_size(1.0, 1.0),
            safe_area: Rect::from_size(1.0, 1.0),
            scale: 1.0,
            time: 0.0,
            delta: 0.0,
        });
        (canvas, host)
    }

    #[test]
    fn writing_redraws_only_canvases_that_read_while_drawing() {
        // Verifies a write from one canvas's event invalidates the readers and skips canvases that did not read.
        let signal = Signal::new(0.0);
        let (writer, writer_host) = mount(&signal, false);
        let (_reader, reader_host) = mount(&signal, true);

        writer.event(&CanvasEvent::Focus(true));

        assert_eq!(signal.get(), 1.0);
        assert_eq!(reader_host.0.get(), 1);
        assert_eq!(writer_host.0.get(), 0);
    }

    #[test]
    fn removed_readers_are_forgotten() {
        // Verifies a dropped canvas no longer counts as a reader once the signal is written.
        let signal = Signal::new(0.0);
        let (reader, _) = mount(&signal, true);
        drop(reader);

        signal.set(2.0);
        assert!(signal.0.readers.borrow().is_empty());
    }
}
