-- @automodel
--    description: Get all user summaries - reuses UserSummary return struct
--    expect: multiple
--    return_type: UserSummary
-- @end

SELECT id, name, email 
FROM public.users 
ORDER BY name
