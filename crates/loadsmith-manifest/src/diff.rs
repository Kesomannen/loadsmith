use loadsmith_core::{Checksum, PackageId, Version};
use std::collections::{HashMap, HashSet};

/// The result of comparing an old and a new package set.
///
/// Each field identifies packages that were added, removed, or changed
/// (version or checksum mismatch) between the two sets.
///
/// Use [`Lockfile::diff`](crate::Lockfile::diff) or
/// [`ProfileState::diff`](crate::ProfileState::diff) to obtain a `Diff`
/// through the public API.
#[derive(Debug, Clone)]
pub struct Diff<'a, T, U> {
    pub added: Vec<&'a U>,
    pub removed: Vec<&'a T>,
    pub changed: Vec<(&'a T, &'a U)>,
}

/// Types that can be compared in a [`Diff`] operation.
///
/// Implementors must report a checksum and a version so that `Diff::compute`
/// can detect when a package has changed between two snapshots.
pub trait Diffable {
    /// Return the checksum of this package, if available.
    ///
    /// A `None` value signals that the package cannot be compared by checksum.
    fn checksum(&self) -> Option<&Checksum>;

    /// Return the version of this package.
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

    /// Returns `true` when no packages were added, removed, or changed.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use loadsmith_core::{FileUrl, PackageRef, Version};
    /// use loadsmith_manifest::{LockedPackage, Lockfile};
    ///
    /// let url = FileUrl::try_from_url("https://example.com/pkg.zip").unwrap();
    /// let a = Lockfile::new(vec![
    ///     LockedPackage::new(PackageRef::new("Pkg", Version::new(1, 0, 0)), "ts", url.clone()),
    /// ]);
    /// let b = Lockfile::new(vec![
    ///     LockedPackage::new(PackageRef::new("Pkg", Version::new(1, 0, 0)), "ts", url),
    /// ]);
    ///
    /// assert!(a.diff(&b).is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.removed.is_empty() && self.changed.is_empty()
    }

    /// Yields references to every package that should end up installed.
    ///
    /// This includes both freshly added packages and the *new* side of every
    /// changed entry.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use loadsmith_core::{FileUrl, PackageRef, Version};
    /// use loadsmith_manifest::{LockedPackage, Lockfile};
    ///
    /// let url = FileUrl::try_from_url("https://example.com/pkg.zip").unwrap();
    ///
    /// let old = Lockfile::new(vec![
    ///     LockedPackage::new(PackageRef::new("Pkg", Version::new(1, 0, 0)), "ts", url.clone()),
    /// ]);
    /// let new = Lockfile::new(vec![
    ///     LockedPackage::new(PackageRef::new("Pkg", Version::new(2, 0, 0)), "ts", url),
    /// ]);
    ///
    /// let diff = old.diff(&new);
    /// let to_add: Vec<&LockedPackage> = diff.to_add().collect();
    /// assert_eq!(to_add.len(), 1);
    /// assert_eq!(to_add[0].ref_.version().to_string(), "2.0.0");
    /// ```
    pub fn to_add(&'a self) -> impl Iterator<Item = &'a U> {
        self.added
            .iter()
            .chain(self.changed.iter().map(|(_old, new)| new))
            .copied()
    }

    /// Yields references to every package that should be removed.
    ///
    /// This includes both wholly removed packages and the *old* side of every
    /// changed entry.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use loadsmith_core::{FileUrl, PackageRef, Version};
    /// use loadsmith_manifest::{LockedPackage, Lockfile};
    ///
    /// let url = FileUrl::try_from_url("https://example.com/pkg.zip").unwrap();
    ///
    /// let old = Lockfile::new(vec![
    ///     LockedPackage::new(PackageRef::new("Pkg", Version::new(1, 0, 0)), "ts", url.clone()),
    /// ]);
    /// let new = Lockfile::new(vec![
    ///     LockedPackage::new(PackageRef::new("Pkg", Version::new(2, 0, 0)), "ts", url),
    /// ]);
    ///
    /// let diff = old.diff(&new);
    /// let to_remove: Vec<&LockedPackage> = diff.to_remove().collect();
    /// assert_eq!(to_remove.len(), 1);
    /// assert_eq!(to_remove[0].ref_.version().to_string(), "1.0.0");
    /// ```
    pub fn to_remove(&'a self) -> impl Iterator<Item = &'a T> {
        self.removed
            .iter()
            .chain(self.changed.iter().map(|(old, _new)| old))
            .copied()
    }
}
