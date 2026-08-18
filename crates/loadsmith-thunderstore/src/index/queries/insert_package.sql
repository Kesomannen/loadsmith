insert into
    packages (community, package_id, package, update_version)
values
    (?1, ?2, ?3, ?4) on conflict (community, package_id) do
update
set
    package = excluded.package,
    update_version = excluded.update_version;