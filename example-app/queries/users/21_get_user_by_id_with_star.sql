-- @automodel
--    description: Get a single user by ID using SELECT * to fetch all columns
--    expect: possible_one
--    types:
--      public.users.profile: "crate::models::UserProfile"
-- @end

SELECT * 
FROM public.users 
WHERE id = #{user_id}
