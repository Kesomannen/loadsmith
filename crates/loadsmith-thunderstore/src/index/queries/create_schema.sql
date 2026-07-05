create table if not exists packages (
    community text not null,
    package_id text not null,
    package text not null,
    primary key (community, package_id)
);

create table if not exists community_meta (
    community text not null primary key,
    last_updated timestamp
);

create index if not exists idx_packages_package_id on packages(package_id);
