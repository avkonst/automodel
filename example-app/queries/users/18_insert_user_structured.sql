-- @automodel
--    description: Insert a new user using structured parameters - all params passed as a single struct
--    expect: exactly_one
--    parameters_type: true
-- @end

INSERT INTO public.users (name, email, age) 
VALUES (#{name}, #{email}, #{age}) 
RETURNING id, name, email, age, created_at
