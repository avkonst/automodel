use serde::{Serialize, Deserialize};
use tokio_postgres::types::{FromSql, ToSql, Type};
use std::error::Error;

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

/// Test query with JSON parameter
/// Generated from SQL: SELECT ${test_data}::jsonb as test_data
pub async fn test_json_query(client: &tokio_postgres::Client, test_data: serde_json::Value) -> Result<Option<crate::models::TestData>, tokio_postgres::Error> {
    let stmt = client.prepare("SELECT $1::jsonb as test_data").await?;
    let row = client.query_one(&stmt, &[&test_data]).await?;
    Ok(row.get::<_, Option<JsonWrapper<crate::models::TestData>>>(0).map(|wrapper| wrapper.into_inner()))
}

/// Create the users table with all necessary fields
/// Generated from SQL: CREATE TABLE IF NOT EXISTS users (id SERIAL PRIMARY KEY, name TEXT NOT NULL, email TEXT UNIQUE NOT NULL, age INTEGER, profile JSONB, created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(), updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW())
pub async fn create_users_table(client: &tokio_postgres::Client) -> Result<(), tokio_postgres::Error> {
    let stmt = client.prepare("CREATE TABLE IF NOT EXISTS users (id SERIAL PRIMARY KEY, name TEXT NOT NULL, email TEXT UNIQUE NOT NULL, age INTEGER, profile JSONB, created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(), updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW())").await?;
    client.execute(&stmt, &[]).await?;
    Ok(())
}

#[derive(Debug, Clone)]
pub struct InsertUserResult {
    pub id: Option<i32>,
    pub name: Option<String>,
    pub email: Option<String>,
    pub age: Option<i32>,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Insert a new user with all fields and return the created user
/// Generated from SQL:
/// INSERT INTO users (name, email, age, profile)
/// VALUES (${name}, ${email}, ${age}, ${profile})
/// RETURNING id, name, email, age, created_at
pub async fn insert_user(client: &tokio_postgres::Client, name: String, email: String, age: i32, profile: serde_json::Value) -> Result<InsertUserResult, tokio_postgres::Error> {
    let stmt = client.prepare("INSERT INTO users (name, email, age, profile)\nVALUES ($1, $2, $3, $4)\nRETURNING id, name, email, age, created_at\n").await?;
    let row = client.query_one(&stmt, &[&name, &email, &age, &profile]).await?;
    Ok(InsertUserResult {
        id: row.get::<_, Option<i32>>(0),
        name: row.get::<_, Option<String>>(1),
        email: row.get::<_, Option<String>>(2),
        age: row.get::<_, Option<i32>>(3),
        created_at: row.get::<_, Option<chrono::DateTime<chrono::Utc>>>(4),
    })
}

#[derive(Debug, Clone)]
pub struct GetAllUsersResult {
    pub id: Option<i32>,
    pub name: Option<String>,
    pub email: Option<String>,
    pub age: Option<i32>,
    pub profile: Option<crate::models::UserProfile>,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Get all users with all their fields
/// Generated from SQL: SELECT id, name, email, age, profile, created_at, updated_at FROM users ORDER BY created_at DESC
pub async fn get_all_users(client: &tokio_postgres::Client) -> Result<GetAllUsersResult, tokio_postgres::Error> {
    let stmt = client.prepare("SELECT id, name, email, age, profile, created_at, updated_at FROM users ORDER BY created_at DESC").await?;
    let row = client.query_one(&stmt, &[]).await?;
    Ok(GetAllUsersResult {
        id: row.get::<_, Option<i32>>(0),
        name: row.get::<_, Option<String>>(1),
        email: row.get::<_, Option<String>>(2),
        age: row.get::<_, Option<i32>>(3),
        profile: row.get::<_, Option<JsonWrapper<crate::models::UserProfile>>>(4).map(|wrapper| wrapper.into_inner()),
        created_at: row.get::<_, Option<chrono::DateTime<chrono::Utc>>>(5),
        updated_at: row.get::<_, Option<chrono::DateTime<chrono::Utc>>>(6),
    })
}

#[derive(Debug, Clone)]
pub struct FindUserByEmailResult {
    pub id: Option<i32>,
    pub name: Option<String>,
    pub email: Option<String>,
    pub age: Option<i32>,
    pub profile: Option<crate::models::UserProfile>,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Find a user by their email address
/// Generated from SQL: SELECT id, name, email, age, profile, created_at, updated_at FROM users WHERE email = ${email}
pub async fn find_user_by_email(client: &tokio_postgres::Client, email: String) -> Result<FindUserByEmailResult, tokio_postgres::Error> {
    let stmt = client.prepare("SELECT id, name, email, age, profile, created_at, updated_at FROM users WHERE email = $1").await?;
    let row = client.query_one(&stmt, &[&email]).await?;
    Ok(FindUserByEmailResult {
        id: row.get::<_, Option<i32>>(0),
        name: row.get::<_, Option<String>>(1),
        email: row.get::<_, Option<String>>(2),
        age: row.get::<_, Option<i32>>(3),
        profile: row.get::<_, Option<JsonWrapper<crate::models::UserProfile>>>(4).map(|wrapper| wrapper.into_inner()),
        created_at: row.get::<_, Option<chrono::DateTime<chrono::Utc>>>(5),
        updated_at: row.get::<_, Option<chrono::DateTime<chrono::Utc>>>(6),
    })
}

#[derive(Debug, Clone)]
pub struct UpdateUserProfileResult {
    pub id: Option<i32>,
    pub name: Option<String>,
    pub email: Option<String>,
    pub age: Option<i32>,
    pub profile: Option<crate::models::UserProfile>,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Update a user's profile by their ID
/// Generated from SQL: UPDATE users SET profile = ${profile}, updated_at = NOW() WHERE id = ${user_id} RETURNING id, name, email, age, profile, updated_at
pub async fn update_user_profile(client: &tokio_postgres::Client, profile: serde_json::Value, user_id: i32) -> Result<UpdateUserProfileResult, tokio_postgres::Error> {
    let stmt = client.prepare("UPDATE users SET profile = $1, updated_at = NOW() WHERE id = $2 RETURNING id, name, email, age, profile, updated_at").await?;
    let row = client.query_one(&stmt, &[&profile, &user_id]).await?;
    Ok(UpdateUserProfileResult {
        id: row.get::<_, Option<i32>>(0),
        name: row.get::<_, Option<String>>(1),
        email: row.get::<_, Option<String>>(2),
        age: row.get::<_, Option<i32>>(3),
        profile: row.get::<_, Option<JsonWrapper<crate::models::UserProfile>>>(4).map(|wrapper| wrapper.into_inner()),
        updated_at: row.get::<_, Option<chrono::DateTime<chrono::Utc>>>(5),
    })
}


// JSON wrapper for custom types that implement Serialize/Deserialize
struct JsonWrapper<T>(T);

impl<T> JsonWrapper<T>
where
    T: for<'de> Deserialize<'de> + Serialize,
{
    fn new(value: T) -> Self {
        Self(value)
    }
    
    fn into_inner(self) -> T {
        self.0
    }
}

impl<T> FromSql<'_> for JsonWrapper<T>
where
    T: for<'de> Deserialize<'de>,
{
    fn from_sql(ty: &Type, raw: &[u8]) -> Result<Self, Box<dyn Error + Sync + Send>> {
        let json_value = serde_json::Value::from_sql(ty, raw)?;
        let value = T::deserialize(json_value)?;
        Ok(JsonWrapper(value))
    }

    fn accepts(ty: &Type) -> bool {
        matches!(*ty, Type::JSON | Type::JSONB)
    }
}

impl<T> ToSql for JsonWrapper<T>
where
    T: Serialize + std::fmt::Debug,
{
    fn to_sql(&self, ty: &Type, out: &mut bytes::BytesMut) -> Result<tokio_postgres::types::IsNull, Box<dyn Error + Sync + Send>> {
        let json_value = serde_json::to_value(&self.0)?;
        json_value.to_sql(ty, out)
    }

    fn accepts(ty: &Type) -> bool {
        matches!(*ty, Type::JSON | Type::JSONB)
    }

    tokio_postgres::types::to_sql_checked!();
}

impl<T> std::fmt::Debug for JsonWrapper<T>
where
    T: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("JsonWrapper").field(&self.0).finish()
    }
}
