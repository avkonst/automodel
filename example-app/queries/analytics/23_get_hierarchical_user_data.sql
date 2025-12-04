-- @automodel
--    description: Recursive CTE to build user hierarchy with referral relationships
--    expect: multiple
-- @end

WITH RECURSIVE user_hierarchy AS (
  -- Base case: users without referrers (or top-level users)
  SELECT 
    id, 
    name, 
    email, 
    NULL::integer as referrer_id,
    1 as level,
    ARRAY[id] as path
  FROM users 
  WHERE referrer_id IS NULL
  
  UNION ALL
  
  -- Recursive case: users with referrers
  SELECT 
    u.id,
    u.name,
    u.email,
    u.referrer_id,
    uh.level + 1,
    uh.path || u.id
  FROM users u
  INNER JOIN user_hierarchy uh ON u.referrer_id = uh.id
  WHERE u.id != ALL(uh.path) -- Prevent cycles
  AND uh.level < 5 -- Limit depth
)
SELECT 
  uh.id,
  uh.name,
  uh.email,
  uh.referrer_id,
  uh.level,
  uh.path,
  COUNT(referrals.id) as direct_referrals_count
FROM user_hierarchy uh
LEFT JOIN users referrals ON referrals.referrer_id = uh.id
GROUP BY uh.id, uh.name, uh.email, uh.referrer_id, uh.level, uh.path
ORDER BY uh.level, uh.name
