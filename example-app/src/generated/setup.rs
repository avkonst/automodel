use sqlx::Row;

/// Create the users table with all necessary fields
/// Generated from SQL: CREATE TABLE IF NOT EXISTS users (id SERIAL PRIMARY KEY, name TEXT NOT NULL, email TEXT UNIQUE NOT NULL, age INTEGER, profile JSONB, created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(), updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW())
pub async fn create_users_table(pool: &sqlx::PgPool) -> Result<(), sqlx::Error> {
    let mut query = sqlx::query("CREATE TABLE IF NOT EXISTS users (id SERIAL PRIMARY KEY, name TEXT NOT NULL, email TEXT UNIQUE NOT NULL, age INTEGER, profile JSONB, created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(), updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW())");
    query.execute(pool).await?;
    Ok(())
}

