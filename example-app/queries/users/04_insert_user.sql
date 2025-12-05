-- @automodel
--    description: Insert a new user with all fields and return the created user
--    expect: exactly_one
--    types:
--      profile: "crate::models::UserProfile"
--    telemetry:
--      level: trace
--      include_params: [name, email, age]
-- @end

INSERT INTO public.users (name, email, age, profile)
VALUES (#{name}, #{email}, #{age}, #{profile})
RETURNING id, name, email, age, created_at
