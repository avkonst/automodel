/// Create the users table with all necessary fields
/// Generated from SQL: CREATE TABLE IF NOT EXISTS users (id SERIAL PRIMARY KEY, name TEXT NOT NULL, email TEXT UNIQUE NOT NULL, age INTEGER, profile JSONB, created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(), updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW())
pub async fn create_users_table(client: &tokio_postgres::Client) -> Result<(), tokio_postgres::Error> {
    let stmt = client.prepare("CREATE TABLE IF NOT EXISTS users (id SERIAL PRIMARY KEY, name TEXT NOT NULL, email TEXT UNIQUE NOT NULL, age INTEGER, profile JSONB, created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(), updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW())").await?;
    client.execute(&stmt, &[]).await?;
    Ok(())
}

