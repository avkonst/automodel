/// Get the current timestamp
/// Generated from SQL: SELECT NOW() as current_time
pub async fn get_current_time(client: &tokio_postgres::Client) -> Result<Option<chrono::DateTime<chrono::Utc>>, tokio_postgres::Error> {
    let stmt = client.prepare("SELECT NOW() as current_time").await?;
    let row = client.query_one(&stmt, &[]).await?;
    Ok(row.get::<_, Option<chrono::DateTime<chrono::Utc>>>(0))
}

/// Get PostgreSQL version
/// Generated from SQL: SELECT version() as pg_version
pub async fn get_version(client: &tokio_postgres::Client) -> Result<Option<String>, tokio_postgres::Error> {
    let stmt = client.prepare("SELECT version() as pg_version").await?;
    let row = client.query_one(&stmt, &[]).await?;
    Ok(row.get::<_, Option<String>>(0))
}

