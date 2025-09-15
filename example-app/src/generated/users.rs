#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UserStatus {
    Active,
    Inactive,
    Suspended,
    Pending,
}

impl std::str::FromStr for UserStatus {
    type Err = String;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "active" => Ok(UserStatus::Active),
            "inactive" => Ok(UserStatus::Inactive),
            "suspended" => Ok(UserStatus::Suspended),
            "pending" => Ok(UserStatus::Pending),
            _ => Err(format!("Invalid UserStatus variant: {}", s)),
        }
    }
}

impl std::fmt::Display for UserStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            UserStatus::Active => "active",
            UserStatus::Inactive => "inactive",
            UserStatus::Suspended => "suspended",
            UserStatus::Pending => "pending",
        };
        write!(f, "{}", s)
    }
}

impl tokio_postgres::types::FromSql<'_> for UserStatus {
    fn from_sql(
        _ty: &tokio_postgres::types::Type,
        raw: &[u8],
    ) -> Result<Self, Box<dyn std::error::Error + Sync + Send>> {
        let s = std::str::from_utf8(raw)?;
        s.parse().map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e)) as Box<dyn std::error::Error + Sync + Send>)
    }

    fn accepts(ty: &tokio_postgres::types::Type) -> bool {
        matches!(ty.kind(), tokio_postgres::types::Kind::Enum(_))
    }
}

impl tokio_postgres::types::ToSql for UserStatus {
    fn to_sql(
        &self,
        _ty: &tokio_postgres::types::Type,
        out: &mut tokio_postgres::types::private::BytesMut,
    ) -> Result<tokio_postgres::types::IsNull, Box<dyn std::error::Error + Sync + Send>> {
        out.extend_from_slice(self.to_string().as_bytes());
        Ok(tokio_postgres::types::IsNull::No)
    }

    fn accepts(ty: &tokio_postgres::types::Type) -> bool {
        matches!(ty.kind(), tokio_postgres::types::Kind::Enum(_))
    }

    tokio_postgres::types::to_sql_checked!();
}


#[derive(Debug, Clone)]
pub struct InsertUserResult {
    pub id: i32,
    pub name: String,
    pub email: String,
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
        id: row.get::<_, i32>(0),
        name: row.get::<_, String>(1),
        email: row.get::<_, String>(2),
        age: row.get::<_, Option<i32>>(3),
        created_at: row.get::<_, Option<chrono::DateTime<chrono::Utc>>>(4),
    })
}

#[derive(Debug, Clone)]
pub struct GetAllUsersResult {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub age: Option<i32>,
    pub profile: Option<serde_json::Value>,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Get all users with all their fields
/// Generated from SQL: SELECT id, name, email, age, profile, created_at, updated_at FROM users ORDER BY created_at DESC
pub async fn get_all_users(client: &tokio_postgres::Client) -> Result<Vec<GetAllUsersResult>, tokio_postgres::Error> {
    let stmt = client.prepare("SELECT id, name, email, age, profile, created_at, updated_at FROM users ORDER BY created_at DESC").await?;
    let rows = client.query(&stmt, &[]).await?;
    let result = rows.into_iter().map(|row| {
        GetAllUsersResult {
        id: row.get::<_, i32>(0),
        name: row.get::<_, String>(1),
        email: row.get::<_, String>(2),
        age: row.get::<_, Option<i32>>(3),
        profile: row.get::<_, Option<serde_json::Value>>(4),
        created_at: row.get::<_, Option<chrono::DateTime<chrono::Utc>>>(5),
        updated_at: row.get::<_, Option<chrono::DateTime<chrono::Utc>>>(6),
    }
    }).collect();
    Ok(result)
}

#[derive(Debug, Clone)]
pub struct FindUserByEmailResult {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub age: Option<i32>,
    pub profile: Option<serde_json::Value>,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Find a user by their email address
/// Generated from SQL: SELECT id, name, email, age, profile, created_at, updated_at FROM users WHERE email = ${email}
pub async fn find_user_by_email(client: &tokio_postgres::Client, email: String) -> Result<Option<FindUserByEmailResult>, tokio_postgres::Error> {
    let stmt = client.prepare("SELECT id, name, email, age, profile, created_at, updated_at FROM users WHERE email = $1").await?;
    let rows = client.query(&stmt, &[&email]).await?;
    let extracted_value = if let Some(row) = rows.into_iter().next() {
        Some(FindUserByEmailResult {
        id: row.get::<_, i32>(0),
        name: row.get::<_, String>(1),
        email: row.get::<_, String>(2),
        age: row.get::<_, Option<i32>>(3),
        profile: row.get::<_, Option<serde_json::Value>>(4),
        created_at: row.get::<_, Option<chrono::DateTime<chrono::Utc>>>(5),
        updated_at: row.get::<_, Option<chrono::DateTime<chrono::Utc>>>(6),
    })
    } else {
        None
    };
    Ok(extracted_value)
}

#[derive(Debug, Clone)]
pub struct UpdateUserProfileResult {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub age: Option<i32>,
    pub profile: Option<serde_json::Value>,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Update a user's profile by their ID
/// Generated from SQL: UPDATE users SET profile = ${profile}, updated_at = NOW() WHERE id = ${user_id} RETURNING id, name, email, age, profile, updated_at
pub async fn update_user_profile(client: &tokio_postgres::Client, profile: serde_json::Value, user_id: i32) -> Result<UpdateUserProfileResult, tokio_postgres::Error> {
    let stmt = client.prepare("UPDATE users SET profile = $1, updated_at = NOW() WHERE id = $2 RETURNING id, name, email, age, profile, updated_at").await?;
    let row = client.query_one(&stmt, &[&profile, &user_id]).await?;
    Ok(UpdateUserProfileResult {
        id: row.get::<_, i32>(0),
        name: row.get::<_, String>(1),
        email: row.get::<_, String>(2),
        age: row.get::<_, Option<i32>>(3),
        profile: row.get::<_, Option<serde_json::Value>>(4),
        updated_at: row.get::<_, Option<chrono::DateTime<chrono::Utc>>>(5),
    })
}

#[derive(Debug, Clone)]
pub struct FindUsersByNameAndAgeResult {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub age: Option<i32>,
}

/// Find users by name pattern with optional minimum age filter
/// Generated from SQL: SELECT id, name, email, age FROM users WHERE name ILIKE ${name_pattern} AND (${min_age?}::integer IS NULL OR age >= ${min_age?})
pub async fn find_users_by_name_and_age(client: &tokio_postgres::Client, name_pattern: String, min_age: Option<i32>) -> Result<Vec<FindUsersByNameAndAgeResult>, tokio_postgres::Error> {
    let stmt = client.prepare("SELECT id, name, email, age FROM users WHERE name ILIKE $1 AND ($2::integer IS NULL OR age >= $3)").await?;
    let rows = client.query(&stmt, &[&name_pattern, &min_age, &min_age]).await?;
    let result = rows.into_iter().map(|row| {
        FindUsersByNameAndAgeResult {
        id: row.get::<_, i32>(0),
        name: row.get::<_, String>(1),
        email: row.get::<_, String>(2),
        age: row.get::<_, Option<i32>>(3),
    }
    }).collect();
    Ok(result)
}

#[derive(Debug, Clone)]
pub struct GetRecentUsersResult {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub age: Option<i32>,
    pub profile: Option<serde_json::Value>,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Get users created after a specific timestamp - expects at least one user
/// Generated from SQL: SELECT id, name, email, age, profile, created_at, updated_at FROM users WHERE created_at > ${since} ORDER BY created_at DESC
pub async fn get_recent_users(client: &tokio_postgres::Client, since: chrono::DateTime<chrono::Utc>) -> Result<Vec<GetRecentUsersResult>, tokio_postgres::Error> {
    let stmt = client.prepare("SELECT id, name, email, age, profile, created_at, updated_at FROM users WHERE created_at > $1 ORDER BY created_at DESC").await?;
    let rows = client.query(&stmt, &[&since]).await?;
    if rows.is_empty() {
        // Simulate the same error that query_one would produce
        let _ = client.query_one("SELECT 1 WHERE FALSE", &[]).await?;
    }
    let result = rows.into_iter().map(|row| {
        GetRecentUsersResult {
        id: row.get::<_, i32>(0),
        name: row.get::<_, String>(1),
        email: row.get::<_, String>(2),
        age: row.get::<_, Option<i32>>(3),
        profile: row.get::<_, Option<serde_json::Value>>(4),
        created_at: row.get::<_, Option<chrono::DateTime<chrono::Utc>>>(5),
        updated_at: row.get::<_, Option<chrono::DateTime<chrono::Utc>>>(6),
    }
    }).collect();
    Ok(result)
}

#[derive(Debug, Clone)]
pub struct GetActiveUsersByAgeRangeResult {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub age: Option<i32>,
    pub profile: Option<serde_json::Value>,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Get active users within an age range - must return at least one user or fails
/// Generated from SQL: SELECT id, name, email, age, profile, created_at FROM users WHERE age BETWEEN ${min_age} AND ${max_age} AND updated_at > NOW() - INTERVAL '30 days'
pub async fn get_active_users_by_age_range(client: &tokio_postgres::Client, min_age: i32, max_age: i32) -> Result<Vec<GetActiveUsersByAgeRangeResult>, tokio_postgres::Error> {
    let stmt = client.prepare("SELECT id, name, email, age, profile, created_at FROM users WHERE age BETWEEN $1 AND $2 AND updated_at > NOW() - INTERVAL '30 days'").await?;
    let rows = client.query(&stmt, &[&min_age, &max_age]).await?;
    if rows.is_empty() {
        // Simulate the same error that query_one would produce
        let _ = client.query_one("SELECT 1 WHERE FALSE", &[]).await?;
    }
    let result = rows.into_iter().map(|row| {
        GetActiveUsersByAgeRangeResult {
        id: row.get::<_, i32>(0),
        name: row.get::<_, String>(1),
        email: row.get::<_, String>(2),
        age: row.get::<_, Option<i32>>(3),
        profile: row.get::<_, Option<serde_json::Value>>(4),
        created_at: row.get::<_, Option<chrono::DateTime<chrono::Utc>>>(5),
    }
    }).collect();
    Ok(result)
}

#[derive(Debug, Clone)]
pub struct SearchUsersByNamePatternResult {
    pub id: i32,
    pub name: String,
    pub email: String,
}

/// Search users by name pattern - expects at least one match
/// Generated from SQL: SELECT id, name, email FROM users WHERE name ILIKE ${pattern} ORDER BY name
pub async fn search_users_by_name_pattern(client: &tokio_postgres::Client, pattern: String) -> Result<Vec<SearchUsersByNamePatternResult>, tokio_postgres::Error> {
    let stmt = client.prepare("SELECT id, name, email FROM users WHERE name ILIKE $1 ORDER BY name").await?;
    let rows = client.query(&stmt, &[&pattern]).await?;
    if rows.is_empty() {
        // Simulate the same error that query_one would produce
        let _ = client.query_one("SELECT 1 WHERE FALSE", &[]).await?;
    }
    let result = rows.into_iter().map(|row| {
        SearchUsersByNamePatternResult {
        id: row.get::<_, i32>(0),
        name: row.get::<_, String>(1),
        email: row.get::<_, String>(2),
    }
    }).collect();
    Ok(result)
}

#[derive(Debug, Clone)]
pub struct GetUsersByStatusResult {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub status: Option<UserStatus>,
}

/// Get users by their status (enum parameter and enum output)
/// Generated from SQL: SELECT id, name, email, status FROM users WHERE status = ${user_status} ORDER BY name
pub async fn get_users_by_status(client: &tokio_postgres::Client, user_status: UserStatus) -> Result<Vec<GetUsersByStatusResult>, tokio_postgres::Error> {
    let stmt = client.prepare("SELECT id, name, email, status FROM users WHERE status = $1 ORDER BY name").await?;
    let rows = client.query(&stmt, &[&user_status]).await?;
    let result = rows.into_iter().map(|row| {
        GetUsersByStatusResult {
        id: row.get::<_, i32>(0),
        name: row.get::<_, String>(1),
        email: row.get::<_, String>(2),
        status: row.get::<_, Option<UserStatus>>(3),
    }
    }).collect();
    Ok(result)
}

#[derive(Debug, Clone)]
pub struct UpdateUserStatusResult {
    pub id: i32,
    pub status: Option<UserStatus>,
}

/// Update user status and return the new status
/// Generated from SQL: UPDATE users SET status = ${new_status} WHERE id = ${user_id} RETURNING id, status
pub async fn update_user_status(client: &tokio_postgres::Client, new_status: UserStatus, user_id: i32) -> Result<UpdateUserStatusResult, tokio_postgres::Error> {
    let stmt = client.prepare("UPDATE users SET status = $1 WHERE id = $2 RETURNING id, status").await?;
    let row = client.query_one(&stmt, &[&new_status, &user_id]).await?;
    Ok(UpdateUserStatusResult {
        id: row.get::<_, i32>(0),
        status: row.get::<_, Option<UserStatus>>(1),
    })
}

/// Get all possible user statuses currently in use
/// Generated from SQL: SELECT DISTINCT status FROM users ORDER BY status
pub async fn get_all_user_statuses(client: &tokio_postgres::Client) -> Result<Vec<Option<UserStatus>>, tokio_postgres::Error> {
    let stmt = client.prepare("SELECT DISTINCT status FROM users ORDER BY status").await?;
    let rows = client.query(&stmt, &[]).await?;
    let result = rows.into_iter().map(|row| {
        row.get::<_, Option<UserStatus>>(0)
    }).collect();
    Ok(result)
}

#[derive(Debug, Clone)]
pub struct GetAllUsersWithStarResult {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub age: Option<i32>,
    pub profile: Option<serde_json::Value>,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
    pub status: Option<UserStatus>,
    pub referrer_id: Option<i32>,
}

/// Get all users using SELECT * to fetch all columns
/// Generated from SQL: SELECT * FROM users ORDER BY created_at DESC
pub async fn get_all_users_with_star(client: &tokio_postgres::Client) -> Result<Vec<GetAllUsersWithStarResult>, tokio_postgres::Error> {
    let stmt = client.prepare("SELECT * FROM users ORDER BY created_at DESC").await?;
    let rows = client.query(&stmt, &[]).await?;
    let result = rows.into_iter().map(|row| {
        GetAllUsersWithStarResult {
        id: row.get::<_, i32>(0),
        name: row.get::<_, String>(1),
        email: row.get::<_, String>(2),
        age: row.get::<_, Option<i32>>(3),
        profile: row.get::<_, Option<serde_json::Value>>(4),
        created_at: row.get::<_, Option<chrono::DateTime<chrono::Utc>>>(5),
        updated_at: row.get::<_, Option<chrono::DateTime<chrono::Utc>>>(6),
        status: row.get::<_, Option<UserStatus>>(7),
        referrer_id: row.get::<_, Option<i32>>(8),
    }
    }).collect();
    Ok(result)
}

#[derive(Debug, Clone)]
pub struct GetUserByIdWithStarResult {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub age: Option<i32>,
    pub profile: Option<serde_json::Value>,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
    pub status: Option<UserStatus>,
    pub referrer_id: Option<i32>,
}

/// Get a single user by ID using SELECT * to fetch all columns
/// Generated from SQL: SELECT * FROM users WHERE id = ${user_id}
pub async fn get_user_by_id_with_star(client: &tokio_postgres::Client, user_id: i32) -> Result<Option<GetUserByIdWithStarResult>, tokio_postgres::Error> {
    let stmt = client.prepare("SELECT * FROM users WHERE id = $1").await?;
    let rows = client.query(&stmt, &[&user_id]).await?;
    let extracted_value = if let Some(row) = rows.into_iter().next() {
        Some(GetUserByIdWithStarResult {
        id: row.get::<_, i32>(0),
        name: row.get::<_, String>(1),
        email: row.get::<_, String>(2),
        age: row.get::<_, Option<i32>>(3),
        profile: row.get::<_, Option<serde_json::Value>>(4),
        created_at: row.get::<_, Option<chrono::DateTime<chrono::Utc>>>(5),
        updated_at: row.get::<_, Option<chrono::DateTime<chrono::Utc>>>(6),
        status: row.get::<_, Option<UserStatus>>(7),
        referrer_id: row.get::<_, Option<i32>>(8),
    })
    } else {
        None
    };
    Ok(extracted_value)
}

