//! A table in memory with room for so many keys, for the limits of
//! [`crate::rate`] and [`crate::login`].
//!
//! When it is full, a new key takes the place of the entry that matters least
//! of the next [`LOOK`], not of all of them: a hand goes round the table as on
//! a clock, and making room costs the same however many keys there are. What
//! matters is for the caller to say. An entry that is worth more than another
//! of the [`LOOK`] stays, so a key that is held back goes only when all of
//! those beside it are held back too.

use std::hash::Hash;

use indexmap::IndexMap;

/// How many entries are looked at to make room.
pub const LOOK: usize = 16;

pub struct Table<K, V> {
    entries: IndexMap<K, V>,
    max: usize,
    /// Where the next look for room starts.
    hand: usize,
}

impl<K: Hash + Eq, V> Table<K, V> {
    /// A table with room for `max` keys, at least one.
    pub fn new(max: usize) -> Self {
        Self {
            entries: IndexMap::new(),
            max: max.max(1),
            hand: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        self.entries.get(key)
    }

    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        self.entries.get_mut(key)
    }

    pub fn remove(&mut self, key: &K) -> Option<V> {
        self.entries.swap_remove(key)
    }

    /// The key's entry, made with `new` if it has none. In a full table a new
    /// one first takes the place of the entry that `worth` puts lowest of the
    /// next [`LOOK`] after the hand, of equal ones the first.
    pub fn entry<W: Ord>(
        &mut self,
        key: K,
        new: impl FnOnce() -> V,
        worth: impl Fn(&K, &V) -> W,
    ) -> &mut V {
        let len = self.entries.len();
        if len >= self.max && !self.entries.contains_key(&key) {
            let least = (0..LOOK.min(len))
                .filter_map(|step| {
                    let at = (self.hand + step) % len;
                    let (key, value) = self.entries.get_index(at)?;
                    Some((worth(key, value), step, at))
                })
                .min();
            if let Some((_, _, at)) = least {
                self.entries.swap_remove_index(at);
            }
            self.hand = (self.hand + LOOK) % len;
        }
        self.entries.entry(key).or_insert_with(new)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use proptest::prelude::*;

    use super::*;

    #[derive(Debug, Clone)]
    enum Step {
        /// Sets the key's value, which is also what it is worth.
        Set(u8, u8),
        Remove(u8),
    }

    fn step() -> impl Strategy<Value = Step> {
        prop_oneof![
            4 => (any::<u8>(), any::<u8>()).prop_map(|(key, value)| Step::Set(key, value)),
            1 => any::<u8>().prop_map(Step::Remove),
        ]
    }

    proptest! {
        /// What the table keeps is what was set last, never more than it has
        /// room for, and a key it has is never what makes room.
        #[test]
        fn keeps_what_was_set_within_its_room(max in 1usize..40, steps in prop::collection::vec(step(), 0..400)) {
            let mut table = Table::new(max);
            let mut model = HashMap::new();
            for step in steps {
                match step {
                    Step::Set(key, value) => {
                        let (had, len) = (table.get(&key).is_some(), table.len());
                        *table.entry(key, || value, |_, worth| *worth) = value;
                        model.insert(key, value);
                        if had {
                            prop_assert_eq!(table.len(), len);
                        }
                        prop_assert_eq!(table.get(&key), Some(&value));
                    }
                    Step::Remove(key) => {
                        let had = table.get(&key).copied();
                        prop_assert_eq!(table.remove(&key), had);
                        model.remove(&key);
                    }
                }
                prop_assert!(table.len() <= max);
                for (key, value) in &table.entries {
                    prop_assert_eq!(model.get(key), Some(value));
                }
            }
        }
    }

    #[test]
    fn what_is_worth_more_than_those_beside_it_stays() {
        let mut table = Table::new(100);
        *table.entry(0, || 2, |_, worth| *worth) = 2;
        for key in 1..100 {
            table.entry(key, || 1, |_, worth| *worth);
        }
        // Many times as many new keys as there is room for: each takes the
        // place of one worth as little as itself, never of the one worth more.
        for key in 100..10_000 {
            table.entry(key, || 1, |_, worth| *worth);
            assert_eq!(table.len(), 100);
            assert_eq!(table.get(&0), Some(&2), "{key}");
        }
        // Once all are worth as much, it may go too.
        for key in 10_000..11_000 {
            table.entry(key, || 2, |_, worth| *worth);
        }
        assert_eq!(table.get(&0), None);
    }

    #[test]
    fn of_those_looked_at_the_least_goes() {
        let mut table = Table::new(LOOK);
        for key in 0..LOOK {
            table.entry(key, || 10 + key, |_, worth| *worth);
        }
        *table.get_mut(&5).unwrap() = 0;
        table.entry(LOOK, || 99, |_, worth| *worth);
        assert_eq!(table.get(&5), None);
        assert_eq!(table.len(), LOOK);
    }
}
