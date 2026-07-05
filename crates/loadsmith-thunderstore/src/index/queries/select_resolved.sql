select 
    json_extract(value, '$.download_url') as download_url,
    json_extract(value, '$.file_size') as file_size,
    json_extract(value, '$.dependencies') as deps,
    json_extract(package, '$.categories') as categories
from
    packages,
    json_each(package, '$.versions') 
where 
    package_id = ?1 and
    json_extract(value, '$.version_number') = ?2
limit 1;
