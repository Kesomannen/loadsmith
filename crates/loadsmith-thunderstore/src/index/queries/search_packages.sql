select
    packages.package_id
from 
    packages,
    json_each(packages.package, '$.versions') as versions
where
    packages.package_id like ?1 and
    (
        ?2 is null or
        packages.community = ?2
    )
group by
    packages.community,
    packages.package_id
order by
    json_extract(packages.package, '$.is_pinned') desc,
    json_extract(packages.package, '$.rating_score') desc,
    sum(json_extract(versions.value, '$.downloads')) desc;

