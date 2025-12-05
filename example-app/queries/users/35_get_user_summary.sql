-- @automodel
--    description: Get user summary - generates UserSummary return struct with custom name
--    expect: exactly_one
--    return_type: UserSummary
-- @end

SELECT id, name, email 
FROM public.users 
WHERE id = ${user_id}
