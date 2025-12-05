-- Test schema for AutoModel
-- Create user status enum
CREATE TYPE user_status AS ENUM ('active', 'inactive', 'suspended', 'pending');
CREATE TABLE IF NOT EXISTS public.users (
    id SERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    email VARCHAR(255) UNIQUE NOT NULL,
    status user_status DEFAULT 'pending',
    profile JSONB,
    settings JSONB,
    is_active BOOLEAN DEFAULT true,
    age INTEGER,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);
-- Insert some test data
INSERT INTO public.users (name, email, status, age, profile, settings)
VALUES (
        'John Doe',
        'john@example.com',
        'active',
        30,
        '{"bio": "Software developer", "avatar_url": "https://example.com/avatar1.jpg", "preferences": {"theme": "dark", "language": "en", "notifications_enabled": true}, "social_links": [{"platform": "github", "url": "https://github.com/johndoe"}]}',
        '{"privacy_level": "public", "email_notifications": true, "two_factor_enabled": false, "api_access": true}'
    ),
    (
        'Jane Smith',
        'jane@example.com',
        'active',
        28,
        '{"bio": "Product manager", "avatar_url": "https://example.com/avatar2.jpg", "preferences": {"theme": "light", "language": "en", "notifications_enabled": false}, "social_links": [{"platform": "linkedin", "url": "https://linkedin.com/in/janesmith"}]}',
        '{"privacy_level": "private", "email_notifications": false, "two_factor_enabled": true, "api_access": false}'
    ),
    (
        'Bob Wilson',
        'bob@example.com',
        'suspended',
        35,
        '{"bio": "Designer", "avatar_url": "https://example.com/avatar3.jpg", "preferences": {"theme": "dark", "language": "en", "notifications_enabled": true}, "social_links": []}',
        '{"privacy_level": "public", "email_notifications": true, "two_factor_enabled": true, "api_access": false}'
    ),
    (
        'Alice Johnson',
        'alice@example.com',
        'pending',
        25,
        '{"bio": "Marketing specialist", "avatar_url": "https://example.com/avatar4.jpg", "preferences": {"theme": "light", "language": "es", "notifications_enabled": true}, "social_links": [{"platform": "twitter", "url": "https://twitter.com/alicejohnson"}]}',
        '{"privacy_level": "private", "email_notifications": false, "two_factor_enabled": false, "api_access": true}'
    ) ON CONFLICT (email) DO NOTHING;
