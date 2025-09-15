-- Schema setup for automodel example app
-- This creates the necessary tables for the complex query examples

-- Add missing columns to existing users table
ALTER TABLE users ADD COLUMN IF NOT EXISTS referrer_id INTEGER REFERENCES users(id);
ALTER TABLE users ADD COLUMN IF NOT EXISTS status TEXT DEFAULT 'active';

-- Create posts table
CREATE TABLE IF NOT EXISTS posts (
    id SERIAL PRIMARY KEY,
    author_id INTEGER NOT NULL REFERENCES users(id),
    title TEXT NOT NULL,
    content TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    published_at TIMESTAMP WITH TIME ZONE
);

-- Create comments table
CREATE TABLE IF NOT EXISTS comments (
    id SERIAL PRIMARY KEY,
    post_id INTEGER NOT NULL REFERENCES posts(id),
    author_id INTEGER NOT NULL REFERENCES users(id),
    content TEXT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Add some sample data for testing
INSERT INTO users (name, email, age, profile, status) VALUES
    ('John Doe', 'john@example.com', 30, '{"bio": "Developer"}', 'active'),
    ('Jane Smith', 'jane@example.com', 25, '{"bio": "Designer"}', 'active'),
    ('Bob Wilson', 'bob@example.com', 35, '{"bio": "Manager"}', 'inactive')
ON CONFLICT (email) DO NOTHING;

-- Add referrer relationships
UPDATE users SET referrer_id = 1 WHERE email = 'jane@example.com';
UPDATE users SET referrer_id = 1 WHERE email = 'bob@example.com';

-- Add some posts
INSERT INTO posts (author_id, title, content, published_at) 
SELECT u.id, 'Sample Post by ' || u.name, 'This is sample content', NOW() - INTERVAL '1 day'
FROM users u WHERE u.email IN ('john@example.com', 'jane@example.com')
ON CONFLICT DO NOTHING;

-- Add some comments
INSERT INTO comments (post_id, author_id, content)
SELECT p.id, u.id, 'Great post!'
FROM posts p, users u 
WHERE u.email = 'bob@example.com' AND p.title LIKE 'Sample Post%'
LIMIT 2
ON CONFLICT DO NOTHING;
