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
    // while respecting the exisitng lockfile versions and version requirements
    // conflicts are not handled

    let mut queue = deps
        .into_iter()
        .map(|dep| (dep, false))
        .collect::<VecDeque<(Dependency, bool)>>();

    let mut visited = HashSet::<PackageId>::from_iter(queue.iter().map(|(dep, _)| dep.id.clone()));
    let mut resolved = Vec::<LockedPackage>::new();

    while let Some((dep, transitive)) = queue.pop_front() {
        let existing = validate_locked_package(existing_lockfile, registries, &dep)?;

        let locked = if let Some(existing) = existing {
            let mut existing = existing.clone();
            existing.transitive = transitive;

            trace!(
                id = %dep.id,
                version = %existing.ref_.version(),
                source = %existing.source,
                "using locked version of package"
            );

            existing
        } else {
            resolve_from_registry_set(registries, dep, transitive).await?
        };

        for trans_dep in locked.deps.iter() {
            if visited.insert(trans_dep.id.clone()) {
                queue.push_back((trans_dep.clone(), true));
            }
        }

        resolved.push(locked);
    }

    Ok(Lockfile::new(resolved))
}

async fn resolve_from_registry_set(
    registries: &RegistrySet,
    dep: Dependency,
    transitive: bool,
) -> Result<LockedPackage> {
    let Dependency {
        id,
        version_req: version_range,
        source,
        registry_metadata,
    } = dep;

    let registry = registries
        .get(&source)
        .ok_or_else(|| Error::UnknownRegistry(source.to_string()))?;

    let versions = registry
        .version_info(&id, registry_metadata.as_ref())
        .await
        .map_err(|err| Error::VersionInfo {
            id: id.clone(),
            source: source.clone(),
            err,
        })?;

    let version = versions
        .into_iter()
        .filter(|v| version_range.matches(&v.version))
        .max_by(|a, b| a.version.cmp(&b.version))
        .ok_or_else(|| Error::NoAvailableVersion(id.clone(), version_range.clone()))?;

    trace!(%id, version = %version.version, source, "resolved package from registry");

    let ref_ = PackageRef::new(id.clone(), version.version);

    let resolved = registry
        .resolve(&ref_, registry_metadata.as_ref())
        .await
        .map_err(|err| Error::Resolve {
            ref_: ref_.clone(),
            source: source.clone(),
            err,
        })?;

    let locked = LockedPackage {
        ref_,
        source: source.clone(),
        deps: resolved.deps,
        url: resolved.url,
        size: resolved.size,
        checksum: resolved.checksum,
        registry_metadata,
        transitive,
    };

    Ok(locked)
}

fn validate_locked_package<'a>(
    lockfile: Option<&'a Lockfile>,
    registries: &RegistrySet,
    dependency: &Dependency,
) -> Result<Option<&'a LockedPackage>> {
    let Some(lockfile) = lockfile else {
        return Ok(None);
    };

    let Some(package) = lockfile.package_by_id(&dependency.id) else {
        return Ok(None);
    };

    if !dependency.version_req.matches(package.ref_.version()) {
        return Ok(None);
    }

    if package.source != dependency.source {
        return Ok(None);
    }

    if package.registry_metadata != dependency.registry_metadata {
        return Ok(None);
    }

    let Some(existing_checksum) = package.checksum.as_ref() else {
        return Ok(Some(package));
    };

    let registry = registries
        .get(&package.source)
        .ok_or_else(|| Error::UnknownRegistry(dependency.source.to_string()))?;

    let new_checksum = registry
        .revalidate_checksum(&package.ref_, package.registry_metadata.as_ref())
        .map_err(|err| Error::Revalidate {
            ref_: package.ref_.clone(),
            source: package.source.clone(),
            err,
        })?;

    if new_checksum.is_some_and(|new| new != *existing_checksum) {
        trace!(
            id = %package.ref_.id(),
            version = %package.ref_.version(),
            source = %package.source,
            "locked package checksum mismatch, revalidating from registry"
        );

        Ok(None)
    } else {
        Ok(Some(package))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use loadsmith_core::{Dependency, FileUrl, Version, VersionReq};
    use loadsmith_registry::offline::{OfflineRegistry, Package, PackageVersion};

    use super::*;

    #[tokio::test]
    async fn resolve_manifest_offline_simple() {
        let dummy_url = FileUrl::try_from_url("https://example.com/dummy.zip").unwrap();

        let a = PackageId::new("A");
        let b = PackageId::new("B");

        let offline_registry = OfflineRegistry::new(HashMap::from_iter([
            (
                a.clone(),
                Package::new(
                    a.clone(),
                    vec![
                        PackageVersion::new(Version::new(1, 0, 0), dummy_url.clone()).with_deps(
                            vec![Dependency::new(b.clone(), VersionReq::STAR, "offline")],
                        ),
                    ],
                ),
            ),
            (
                b.clone(),
                Package::new(
                    b.clone(),
                    vec![PackageVersion::new(Version::new(1, 0, 0), dummy_url)],
                ),
            ),
        ]));

        let mut registries = RegistrySet::new();
        registries.add("offline", offline_registry);

        let dependencies = vec![Dependency::new(
            a.clone(),
            VersionReq::parse("=1.0.0").unwrap(),
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
