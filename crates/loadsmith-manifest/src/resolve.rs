use std::collections::{HashSet, VecDeque};

use loadsmith_core::{Dependency, PackageId, PackageRef};
use loadsmith_registry::RegistrySet;
use tracing::trace;

use crate::{
    Error, Result,
    lockfile::{LockedPackage, Lockfile},
};

pub async fn resolve<I>(
    deps: I,
    registries: &RegistrySet,
    existing_lockfile: Option<&Lockfile>,
) -> Result<Lockfile>
where
    I: IntoIterator<Item = Dependency>,
{
    // simple BFS that uses the newest version of each encountered package,
    // while respecting the exisitng lockfile versions

    let mut queue = deps.into_iter().collect::<VecDeque<Dependency>>();

    let mut visited = HashSet::<PackageId>::from_iter(queue.iter().map(|dep| dep.id.clone()));
    let mut resolved = Vec::<LockedPackage>::new();

    while let Some(dep) = queue.pop_front() {
        let Dependency {
            id,
            version_range,
            source,
            registry_metadata,
        } = dep;

        let existing = existing_lockfile
            .and_then(|lockfile| lockfile.package_by_id(&id))
            .and_then(|locked| {
                if version_range.matches(&locked.ref_.version) {
                    Some(locked)
                } else {
                    None
                }
            });

        let locked = if let Some(existing) = existing {
            trace!(%id, version = %existing.ref_.version, source, "using locked version of package");

            existing.clone()
        } else {
            let registry = registries
                .get(&source)
                .ok_or_else(|| Error::UnknownRegistry(source.to_string()))?;

            let versions = registry
                .version_info(&id, registry_metadata.as_ref())
                .await
                .map_err(|err| Error::VersionInfo {
                    id: id.clone(),
                    err,
                })?;

            let version = versions
                .into_iter()
                .filter(|v| version_range.matches(&v.version))
                .max_by_key(|v| v.version)
                .ok_or_else(|| Error::NoAvailableVersion(id.clone(), version_range.clone()))?;

            trace!(%id, version = %version.version, source, "resolved package from registry");

            let ref_ = PackageRef::new(id.clone(), version.version);

            let resolved = registry
                .resolve(&id, &version.version, registry_metadata.as_ref())
                .await
                .map_err(|err| Error::Resolve {
                    ref_: ref_.clone(),
                    err,
                })?;

            LockedPackage {
                ref_,
                source: source.clone(),
                deps: resolved.deps,
                url: resolved.url,
                size: resolved.size,
                checksum: resolved.checksum,
            }
        };

        for trans_dep in locked.deps.iter() {
            if visited.insert(trans_dep.id.clone()) {
                queue.push_back(trans_dep.clone());
            }
        }

        resolved.push(locked);
    }

    Ok(Lockfile::new(resolved))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use loadsmith_core::{Dependency, VersionRange};
    use loadsmith_registry::offline::{OfflineRegistry, Package, PackageVersion};

    use super::*;

    #[tokio::test]
    async fn resolve_manifest_offline_simple() {
        const DUMMY_URL: &str = "https://example.com/package.zip";

        let a = PackageId::new("A");
        let b = PackageId::new("B");

        let offline_registry = OfflineRegistry::new(HashMap::from_iter([
            (
                a.clone(),
                Package::new(
                    a.clone(),
                    vec![PackageVersion::new((1, 0, 0), DUMMY_URL).with_deps(vec![
                        Dependency::new(b.clone(), VersionRange::any(), "offline"),
                    ])],
                ),
            ),
            (
                b.clone(),
                Package::new(b.clone(), vec![PackageVersion::new((1, 0, 0), DUMMY_URL)]),
            ),
        ]));

        let mut registries = RegistrySet::new();
        registries.add("offline", offline_registry);

        let dependencies = vec![Dependency::new(
            a.clone(),
            VersionRange::exact((1, 0, 0)),
            "offline",
        )];

        let lockfile = resolve(dependencies, &registries, None)
            .await
            .expect("failed to resolve manifest");

        assert_eq!(lockfile.packages().len(), 2);
        assert!(lockfile.package_by_id(&a).is_some());
        assert!(lockfile.package_by_id(&b).is_some());

        println!("{lockfile:#?}");
    }
}
