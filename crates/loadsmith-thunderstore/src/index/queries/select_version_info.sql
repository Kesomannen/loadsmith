select 
    json_extract(value, '$.version_number') as version
from
    packages, 
    json_each(packages.package, '$.versions') 
where package_id = ?1;