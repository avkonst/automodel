-- @automodel
--    description: Get user info by email - reuses UserSummary return struct
--    expect: possible_one
--    return_type: UserSummary
-- @end

SELECT id, name, email 
FROM public.users 
WHERE email = ${email}
