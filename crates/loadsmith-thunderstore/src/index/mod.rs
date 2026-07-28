use loadsmith_core::{Dependency, PackageId, VersionReq};
use thunderstore::VersionIdent;

use crate::PackageIdExt;

pub mod in_memory;
pub mod sqlite;

const MODPACK_CATEGORY: &str = "Modpacks";

fn dependencies_from_idents<'a, I>(idents: I, is_modpack: bool) -> Vec<Dependency>
where
    I: IntoIterator<Item = &'a VersionIdent>,
{
    idents
        .into_iter()
        .map(|ident| dependency_from_ident(ident, is_modpack))
        .collect()
}

fn dependency_from_ident(ident: &VersionIdent, is_modpack: bool) -> Dependency {
    let package_id = PackageId::from_ts_ident(ident.package_id());

    // TODO: better source handling instead of random string
    // TODO: better version range handling
    Dependency::new(
        package_id,
        if is_modpack {
            loadsmith_util::exact_version_eq(&ident.parsed_version())
        } else {
            VersionReq::STAR
        },
        "thunderstore".to_string(),
    )
}
