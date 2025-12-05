-- @automodel
--    description: Update user fields using diff-based conditional updates - compares old and new structs
--    expect: exactly_one
--    conditions_type: true
-- @end

UPDATE public.users 
SET updated_at = NOW() 
#[, name = #{name?}] 
#[, email = #{email?}] 
#[, age = #{age?}] 
WHERE id = #{user_id} 
RETURNING id, name, email, age, updated_at
