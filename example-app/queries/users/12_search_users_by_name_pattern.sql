-- @automodel
--    description: Search public.users by name pattern - expects at least one match
--    expect: at_least_one
-- @end

SELECT id, name, email 
FROM public.users 
WHERE name ILIKE #{pattern} 
ORDER BY name
