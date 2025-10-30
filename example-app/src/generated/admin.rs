use sqlx::Row;

/// Get the current timestamp
/// Generated from SQL: SELECT NOW() as current_time
pub async fn get_current_time(pool: &sqlx::PgPool) -> Result<Option<chrono::DateTime<chrono::Utc>>, sqlx::Error> {
    let mut query = sqlx::query("SELECT NOW() as current_time");
    let row = query.fetch_one(pool).await?;
    Ok(row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("current_time")?)
}

/// Get PostgreSQL version
/// Generated from SQL: SELECT version() as pg_version
pub async fn get_version(pool: &sqlx::PgPool) -> Result<Option<String>, sqlx::Error> {
    let mut query = sqlx::query("SELECT version() as pg_version");
    let row = query.fetch_one(pool).await?;
    Ok(row.try_get::<Option<String>, _>("pg_version")?)
}

