-- Test schema for AutoModel
CREATE TABLE IF NOT EXISTS users (
    id SERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    email VARCHAR(255) UNIQUE NOT NULL,
    profile JSONB,
    settings JSONB,
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);
-- Insert some test data
INSERT INTO users (name, email, profile, settings)
VALUES (
        'John Doe',
        'john@example.com',
        '{"bio": "Software developer", "avatar_url": "https://example.com/avatar1.jpg", "preferences": {"theme": "dark", "language": "en", "notifications_enabled": true}, "social_links": [{"platform": "github", "url": "https://github.com/johndoe"}]}',
        '{"privacy_level": "public", "email_notifications": true, "two_factor_enabled": false, "api_access": true}'
    ),
    (
        'Jane Smith',
        'jane@example.com',
        '{"bio": "Product manager", "avatar_url": "https://example.com/avatar2.jpg", "preferences": {"theme": "light", "language": "en", "notifications_enabled": false}, "social_links": [{"platform": "linkedin", "url": "https://linkedin.com/in/janesmith"}]}',
        '{"privacy_level": "private", "email_notifications": false, "two_factor_enabled": true, "api_access": false}'
    ) ON CONFLICT (email) DO NOTHING;
