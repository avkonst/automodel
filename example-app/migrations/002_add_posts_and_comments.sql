-- Schema setup for automodel example app
-- This creates the necessary tables for the complex query examples
-- Add missing columns to existing public.users table
ALTER TABLE public.users
ADD COLUMN IF NOT EXISTS referrer_id INTEGER REFERENCES public.users(id);
ALTER TABLE public.users
ADD COLUMN IF NOT EXISTS status TEXT DEFAULT 'active';
-- Create public.posts table
CREATE TABLE IF NOT EXISTS public.posts (
    id SERIAL PRIMARY KEY,
    author_id INTEGER NOT NULL REFERENCES public.users(id),
    title TEXT NOT NULL,
    content TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    published_at TIMESTAMP WITH TIME ZONE
);
-- Create public.comments table
CREATE TABLE IF NOT EXISTS public.comments (
    id SERIAL PRIMARY KEY,
    post_id INTEGER NOT NULL REFERENCES public.posts(id),
    author_id INTEGER NOT NULL REFERENCES public.users(id),
    content TEXT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);
-- Add some sample data for testing
INSERT INTO public.users (name, email, age, profile, status)
VALUES (
        'John Doe',
        'john@example.com',
        30,
        '{"bio": "Developer"}',
        'active'
    ),
    (
        'Jane Smith',
        'jane@example.com',
        25,
        '{"bio": "Designer"}',
        'active'
    ),
    (
        'Bob Wilson',
        'bob@example.com',
        35,
        '{"bio": "Manager"}',
        'inactive'
    ) ON CONFLICT (email) DO NOTHING;
-- Add referrer relationships
UPDATE public.users
SET referrer_id = 1
WHERE email = 'jane@example.com';
UPDATE public.users
SET referrer_id = 1
WHERE email = 'bob@example.com';
-- Add some public.posts
INSERT INTO public.posts (author_id, title, content, published_at)
SELECT u.id,
    'Sample Post by ' || u.name,
    'This is sample content',
    NOW() - INTERVAL '1 day'
FROM public.users u
WHERE u.email IN ('john@example.com', 'jane@example.com') ON CONFLICT DO NOTHING;
-- Add some public.comments
INSERT INTO public.comments (post_id, author_id, content)
SELECT p.id,
    u.id,
    'Great post!'
FROM public.posts p,
    public.users u
WHERE u.email = 'bob@example.com'
    AND p.title LIKE 'Sample Post%'
LIMIT 2 ON CONFLICT DO NOTHING;
