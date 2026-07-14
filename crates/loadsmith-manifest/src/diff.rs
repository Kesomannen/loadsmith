use loadsmith_core::{Checksum, PackageId, Version};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct Diff<'a, T, U> {
    pub added: Vec<&'a U>,
    pub removed: Vec<&'a T>,
    pub changed: Vec<(&'a T, &'a U)>,
}

pub trait Diffable {
    fn checksum(&self) -> Option<&Checksum>;
    fn version(&self) -> &Version;
}

impl<'a, T: Diffable, U: Diffable> Diff<'a, T, U> {
    pub(crate) fn compute(
        old_map: HashMap<&'a PackageId, &'a T>,
        new_map: HashMap<&'a PackageId, &'a U>,
    ) -> Self {
        let all_ids = old_map.keys().chain(new_map.keys()).collect::<HashSet<_>>();

        let mut added = Vec::new();
        let mut removed = Vec::new();
        let mut changed = Vec::new();

        for id in all_ids {
            let old = old_map.get(id);
            let new = new_map.get(id);

            match (old, new) {
                (Some(old), Some(new)) => {
                    if old.version() != new.version() || old.checksum() != new.checksum() {
                        changed.push((*old, *new));
                    }
                }
                (None, Some(new)) => {
                    added.push(*new);
                }
                (Some(old), None) => {
                    removed.push(*old);
                }
                (None, None) => unreachable!(),
            }
        }

        Self {
            added,
            removed,
            changed,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.removed.is_empty() && self.changed.is_empty()
    }

    pub fn to_add(&'a self) -> impl Iterator<Item = &'a U> {
        self.added
            .iter()
            .chain(self.changed.iter().map(|(_old, new)| new))
            .copied()
    }

    pub fn to_remove(&'a self) -> impl Iterator<Item = &'a T> {
        self.removed
            .iter()
            .chain(self.changed.iter().map(|(old, _new)| old))
            .copied()
    }
}
