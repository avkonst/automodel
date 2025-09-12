-- Example database schema for testing the automodel library
-- You can run this in your PostgreSQL database to test the generated functions
CREATE TABLE IF NOT EXISTS users (
    id SERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    email VARCHAR(255) UNIQUE NOT NULL,
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW()
);
CREATE TABLE IF NOT EXISTS posts (
    id SERIAL PRIMARY KEY,
    user_id INTEGER REFERENCES users(id) ON DELETE CASCADE,
    title VARCHAR(255) NOT NULL,
    content TEXT,
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW()
);
-- Insert some sample data
INSERT INTO users (name, email, is_active)
VALUES ('John Doe', 'john@example.com', true),
    ('Jane Smith', 'jane@example.com', true),
    ('Bob Johnson', 'bob@example.com', false),
    ('Alice Brown', 'alice@example.com', true) ON CONFLICT (email) DO NOTHING;
INSERT INTO posts (user_id, title, content)
VALUES (1, 'Hello World', 'This is my first post!'),
    (
        1,
        'Learning Rust',
        'Rust is an amazing language.'
    ),
    (
        2,
        'PostgreSQL Tips',
        'Here are some useful PostgreSQL tips.'
    ),
    (
        4,
        'Web Development',
        'Building web applications with Rust.'
    ) ON CONFLICT DO NOTHING;
