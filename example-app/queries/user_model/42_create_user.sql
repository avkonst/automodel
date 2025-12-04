-- @automodel
--    description: Insert a new user and return as UserModel
--    expect: exactly_one
--    return_type: UserModel
--    error_type: UserContentConstraints
-- @end

INSERT INTO users (name, email, age) 
VALUES (${name}, ${email}, ${age?}) 
RETURNING id, name, email, age
