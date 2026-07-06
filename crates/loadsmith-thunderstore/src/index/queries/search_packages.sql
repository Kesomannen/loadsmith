select
    package_id
from packages
where
    package_id like ?1 and
    (
        ?2 is null or
        community = ?2
    )
order by json_extract(packages.package, '$.downloads') desc;
