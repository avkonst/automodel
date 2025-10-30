use sqlx::Row;

/// Get the current timestamp
pub async fn get_current_time(executor: impl sqlx::Executor<'_, Database = sqlx::Postgres>) -> Result<Option<chrono::DateTime<chrono::Utc>>, sqlx::Error> {
    let query = sqlx::query(
        r"SELECT NOW() as current_time"
    );
    let row = query.fetch_one(executor).await?;
    Ok(row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("current_time")?)
}

/// Get PostgreSQL version
pub async fn get_version(executor: impl sqlx::Executor<'_, Database = sqlx::Postgres>) -> Result<Option<String>, sqlx::Error> {
    let query = sqlx::query(
        r"SELECT version() as pg_version"
    );
    let row = query.fetch_one(executor).await?;
    Ok(row.try_get::<Option<String>, _>("pg_version")?)
}

