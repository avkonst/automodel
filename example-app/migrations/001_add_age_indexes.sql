-- Migration: Add indexes for age range and updated_at queries
-- This migration optimizes the query: 
-- SELECT id, name, email, age, profile, created_at FROM users WHERE age BETWEEN ${min_age} AND ${max_age} AND updated_at > NOW() - INTERVAL '30 days'
-- Add index on age column for age range queries
CREATE INDEX IF NOT EXISTS idx_users_age ON users(age);
-- Add index on updated_at column for time-based filtering
CREATE INDEX IF NOT EXISTS idx_users_updated_at ON users(updated_at);
-- Add composite index on (age, updated_at) for optimal performance of the combined condition
-- This index will be most effective for the specific query pattern
CREATE INDEX IF NOT EXISTS idx_users_age_updated_at ON users(age, updated_at);
-- Optional: Add index on (updated_at, age) for queries that filter primarily by updated_at
-- Uncomment if you have queries that filter by updated_at first, then age
-- CREATE INDEX IF NOT EXISTS idx_users_updated_at_age ON users(updated_at, age);
-- Add some statistics comments
COMMENT ON INDEX idx_users_age IS 'Index for age range queries (age BETWEEN min_age AND max_age)';
COMMENT ON INDEX idx_users_updated_at IS 'Index for time-based queries (updated_at > timestamp)';
COMMENT ON INDEX idx_users_age_updated_at IS 'Composite index for age range + time-based queries (optimal for age BETWEEN x AND y AND updated_at > z)';
