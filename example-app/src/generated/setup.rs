/// Create the users table with all necessary fields
pub async fn create_users_table(executor: impl sqlx::Executor<'_, Database = sqlx::Postgres>) -> Result<(), sqlx::Error> {
    let query = sqlx::query(
        r"CREATE TABLE IF NOT EXISTS users (id SERIAL PRIMARY KEY, name TEXT NOT NULL, email TEXT UNIQUE NOT NULL, age INTEGER, profile JSONB, created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(), updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW())"
    );
    query.execute(executor).await?;
    Ok(())
}

