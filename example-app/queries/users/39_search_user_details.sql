-- @automodel
--    description: Search user details - reuses UserDetails return struct
--    expect: multiple
--    return_type: UserDetails
-- @end

SELECT id, name, email, age, created_at 
FROM public.users 
WHERE name ILIKE #{pattern}
