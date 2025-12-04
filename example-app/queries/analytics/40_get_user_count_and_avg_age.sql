-- @automodel
--    description: Get user count and average age - uses default GetUserCountAndAvgAgeItem struct
--    expect: exactly_one
-- @end

SELECT COUNT(*) as count, AVG(age) as avg_age FROM users
