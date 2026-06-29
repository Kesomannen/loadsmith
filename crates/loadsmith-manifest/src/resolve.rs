use std::collections::{HashSet, VecDeque};

use loadsmith_core::{LockedPackage, PackageId};
use loadsmith_registry::RegistrySet;
use tracing::trace;

use crate::{Error, Result, lockfile::Lockfile, manifest::Dependency};

pub(crate) async fn resolve<I>(
    deps: I,
    registries: &RegistrySet,
    existing_lockfile: Option<&Lockfile>,
) -> Result<Lockfile>
where
    I: IntoIterator<Item = (PackageId, Dependency)>,
{
    // simple BFS that uses the newest version of each encountered package,
    // while respecting the exisitng lockfile versions

    let mut queue = deps
        .into_iter()
        .map(|(id, dep)| {
            let registry = match dep.source() {
                Some(source) => registries
                    .get_registry(&source)
                    .ok_or_else(|| Error::UnknownRegistry(source.to_string()))?,
                None => registries
                    .default_registry()
                    .ok_or(Error::NoDefaultRegistry)?,
            };

            Ok((id, registry.id(), dep.into_registry_metadata()))
        })
        .collect::<Result<VecDeque<_>>>()?;

    let mut visited = HashSet::<PackageId>::from_iter(queue.iter().map(|(id, _, _)| id.clone()));
    let mut resolved = Vec::<LockedPackage>::new();

    while let Some((id, registry_id, metadata)) = queue.pop_front() {
        let locked = if let Some(locked) = existing_lockfile
            .and_then(|lockfile| lockfile.package_by_id_and_source(&id, registry_id.0))
        {
            trace!(%id, version = %locked.version, %registry_id, "using locked version of package");

            locked.clone()
        } else {
            let registry = registries
                .get_registry(&registry_id.0)
                .ok_or_else(|| Error::UnknownRegistry(registry_id.0.to_string()))?;

            let package = registry
                .get_package(&id, metadata.as_ref())
                .await?
                .ok_or_else(|| Error::UnknownPackage {
                    package: id.clone(),
                    registry: registry.id(),
                })?;

            let version = package
                .versions
                .into_iter()
                .max_by_key(|v| v.version)
                .ok_or_else(|| Error::NoAvailableVersion { id: id.clone() })?;

            trace!(%id, version = %version.version, %registry_id, "resolved package from registry");

            LockedPackage {
                package: id.clone(),
                version: version.version,
                source: registry.id().to_string(),
                url: version.download_url,
                checksum: version.checksum,
                deps: version.deps,
            }
        };

        for dep in locked.deps.iter() {
            if visited.insert(dep.clone()) {
                queue.push_back((dep.clone(), registry_id, None));
            }
        }

        resolved.push(locked);
    }

    Ok(Lockfile::new(resolved))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use loadsmith_core::PackageRef;
    use loadsmith_registry::{
        Registry,
        offline::{OfflineRegistry, Package, PackageVersion},
        thunderstore::ThunderstoreRegistry,
    };

    use crate::manifest::Dependency;

    use super::*;

    #[tokio::test]
    async fn resolve_manifest_offline_simple() {
        const DUMMY_URL: &str = "https://example.com/package.zip";

        let a = PackageId::new("A");
        let b = PackageId::new("B");

        let registry = OfflineRegistry::new(HashMap::from_iter([
            (
                a.clone(),
                Package::new(
                    a.clone(),
                    vec![
                        PackageVersion::new((1, 0, 0), DUMMY_URL)
                            .with_deps(vec![PackageRef::new(b.clone(), (1, 0, 0))]),
                    ],
                ),
            ),
            (
                b.clone(),
                Package::new(b.clone(), vec![PackageVersion::new((1, 0, 0), DUMMY_URL)]),
            ),
        ]));

        let registry_id = registry.id();

        let mut registries = RegistrySet::new();
        registries.add_default_registry(registry);

        let dependencies = vec![(a.clone(), Dependency::new((1, 0, 0)))];

        let lockfile = resolve(dependencies, &registries, None)
            .await
            .expect("failed to resolve manifest");

        assert_eq!(lockfile.packages().len(), 2);
        assert!(
            lockfile
                .package_by_id_and_source(&a, registry_id.0)
                .is_some()
        );
        assert!(
            lockfile
                .package_by_id_and_source(&b, registry_id.0)
                .is_some()
        );

        println!("{lockfile:#?}");
    }

    #[tokio::test]
    #[ignore]
    async fn resolve_manifest_thunderstore() {
        let registry = ThunderstoreRegistry::create("rounds").await.unwrap();

        let mut registries = RegistrySet::new();
        registries.add_default_registry(registry);

        let dependencies = vec![(
            PackageId::new("olavim-RoundsWithFriends"),
            Dependency::new((2, 2, 2)),
        )];

        let lockfile = resolve(dependencies, &registries, None)
            .await
            .expect("failed to resolve manifest");

        insta::assert_yaml_snapshot!(lockfile);
    }
}
