create table
    if not exists packages (
        community text not null,
        package_id text not null,
        package text not null,
        -- version number for mark and sweep
        update_version integer not null default 0,
        primary key (community, package_id)
    );

create table
    if not exists community_meta (
        community text not null primary key,
        last_updated timestamp,
        update_version integer not null default 0
    );

create index if not exists idx_packages_package_id on packages (package_id);