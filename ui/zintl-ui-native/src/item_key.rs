//! Keys that identify the rows and items of lists, tables, and sidebars.

use std::collections::HashSet;
use std::rc::Rc;

/// A value that identifies a row or item across renders.
///
/// The backends know rows by a string, so each key turns into one. Keys that
/// cannot implement this trait, such as ID types from another crate, name
/// their rows through the view's `item_id(..)` instead.
#[diagnostic::on_unimplemented(
    message = "`{Self}` does not implement `ItemKey`",
    label = "rows keyed by `{Self}` need item IDs",
    note = "implement `ItemKey` for `{Self}`, or call `.item_id(|key| ..)` on the view"
)]
pub trait ItemKey: Clone + PartialEq + 'static {
    /// Returns the string the backend uses for this key.
    ///
    /// Different keys must return different strings.
    fn item_id(&self) -> String;
}

impl ItemKey for String {
    fn item_id(&self) -> String {
        self.clone()
    }
}

impl ItemKey for &'static str {
    fn item_id(&self) -> String {
        (*self).to_owned()
    }
}

macro_rules! integer_item_keys {
    ($($integer:ty),*) => {
        $(
            impl ItemKey for $integer {
                fn item_id(&self) -> String {
                    self.to_string()
                }
            }
        )*
    };
}

integer_item_keys!(
    u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize
);

/// Turns a view's keys into the strings the backend knows rows by.
///
/// Views use [`ByItemKey`] unless `item_id(..)` gives them an [`ItemIdFn`].
pub trait ItemIdSource<K>: Clone + 'static {
    /// Returns the string the backend uses for `key`.
    ///
    /// Different keys must return different strings.
    fn item_id(&self, key: &K) -> String;
}

/// Names rows by their key's [`ItemKey::item_id`].
#[derive(Clone, Copy, Debug, Default)]
pub struct ByItemKey;

impl<K: ItemKey> ItemIdSource<K> for ByItemKey {
    fn item_id(&self, key: &K) -> String {
        key.item_id()
    }
}

/// Names rows by a function of their key, as `item_id(..)` on a view sets.
pub struct ItemIdFn<K>(Rc<dyn Fn(&K) -> String>);

impl<K> ItemIdFn<K> {
    pub fn new(item_id: impl Fn(&K) -> String + 'static) -> Self {
        Self(Rc::new(item_id))
    }
}

impl<K> Clone for ItemIdFn<K> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<K: 'static> ItemIdSource<K> for ItemIdFn<K> {
    fn item_id(&self, key: &K) -> String {
        (self.0)(key)
    }
}

/// The keys of the rows a view shows, each with its backend string.
///
/// Event handlers hold it to turn the strings the backend reports back into keys.
pub(crate) struct ItemKeys<K>(Rc<[(String, K)]>);

impl<K> Clone for ItemKeys<K> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<K: Clone + PartialEq + 'static> ItemKeys<K> {
    /// Names each of `keys` through `source`.
    pub(crate) fn new<'a>(
        keys: impl IntoIterator<Item = &'a K>,
        source: &impl ItemIdSource<K>,
    ) -> Self {
        let keys: Rc<[(String, K)]> = keys
            .into_iter()
            .map(|key| (source.item_id(key), key.clone()))
            .collect();
        debug_assert!(
            {
                let mut seen = HashSet::new();
                keys.iter().all(|(id, _)| seen.insert(id.as_str()))
            },
            "item keys must have distinct item IDs"
        );
        Self(keys)
    }

    /// Returns the key the backend reported as `id`, if a row still has it.
    pub(crate) fn key(&self, id: &str) -> Option<&K> {
        self.0
            .iter()
            .find(|(candidate, _)| candidate == id)
            .map(|(_, key)| key)
    }

    /// Turns a reported selection into a key, or `Some(None)` for a cleared one.
    ///
    /// Returns `None` when the reported row is gone, so the event is ignored.
    pub(crate) fn selection(&self, id: Option<&str>) -> Option<Option<K>> {
        match id {
            Some(id) => self.key(id).cloned().map(Some),
            None => Some(None),
        }
    }

    /// Returns the backend string of `key`, or `None` when no row has it.
    pub(crate) fn id(&self, key: &K) -> Option<&str> {
        self.0
            .iter()
            .find(|(_, candidate)| candidate == key)
            .map(|(id, _)| id.as_str())
    }

    /// Returns the backend string of each row, in order.
    pub(crate) fn ids(&self) -> impl Iterator<Item = &str> {
        self.0.iter().map(|(id, _)| id.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn built_in_keys_use_their_text() {
        // Verifies strings pass through and integers use their decimal text.
        assert_eq!(String::from("inbox").item_id(), "inbox");
        assert_eq!("inbox".item_id(), "inbox");
        assert_eq!(42_u64.item_id(), "42");
        assert_eq!((-3_i32).item_id(), "-3");
    }

    #[test]
    fn keys_map_backend_ids_back_to_keys() {
        // Verifies reported IDs find their key, and gone rows or keys find nothing.
        let keys = ItemKeys::new(&[7_u32, 8], &ByItemKey);

        assert_eq!(keys.ids().collect::<Vec<_>>(), ["7", "8"]);
        assert_eq!(keys.key("8"), Some(&8));
        assert_eq!(keys.key("9"), None);
        assert_eq!(keys.id(&7), Some("7"));
        assert_eq!(keys.id(&9), None);
        assert_eq!(keys.selection(Some("7")), Some(Some(7)));
        assert_eq!(keys.selection(None), Some(None));
        assert_eq!(keys.selection(Some("9")), None);
    }

    #[test]
    #[should_panic(expected = "distinct item IDs")]
    #[cfg(debug_assertions)]
    fn duplicate_item_ids_are_caught_in_debug_builds() {
        // Verifies two keys with the same item ID trip the debug assertion.
        ItemKeys::new(&["a", "a"], &ByItemKey);
    }

    /// A key type that does not implement `ItemKey`.
    #[derive(Clone, Copy, Debug, PartialEq)]
    struct Foreign(i64);

    #[test]
    fn item_id_functions_name_keys_without_item_key() {
        // Verifies a function source names keys and maps reported IDs back to them.
        let keys = ItemKeys::new(
            &[Foreign(1), Foreign(2)],
            &ItemIdFn::new(|key: &Foreign| format!("t{}", key.0)),
        );

        assert_eq!(keys.ids().collect::<Vec<_>>(), ["t1", "t2"]);
        assert_eq!(keys.key("t2"), Some(&Foreign(2)));
        assert_eq!(keys.id(&Foreign(1)), Some("t1"));
    }

    #[test]
    #[should_panic(expected = "distinct item IDs")]
    #[cfg(debug_assertions)]
    fn colliding_item_id_functions_are_caught_in_debug_builds() {
        // Verifies distinct keys whose function IDs collide trip the debug assertion.
        ItemKeys::new(
            &[Foreign(1), Foreign(2)],
            &ItemIdFn::new(|_: &Foreign| "same".to_owned()),
        );
    }
}
