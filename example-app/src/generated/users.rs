use sqlx::Row;

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

impl sqlx::Type<sqlx::Postgres> for UserStatus {
    fn type_info() -> sqlx::postgres::PgTypeInfo {
        sqlx::postgres::PgTypeInfo::with_name("user_status")
    }
}

impl<'r> sqlx::Decode<'r, sqlx::Postgres> for UserStatus {
    fn decode(value: sqlx::postgres::PgValueRef<'r>) -> Result<Self, Box<dyn std::error::Error + Send + Sync + 'static>> {
        let s = <&str as sqlx::Decode<sqlx::Postgres>>::decode(value)?;
        s.parse().map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e)) as Box<dyn std::error::Error + Send + Sync + 'static>)
    }
}

impl<'q> sqlx::Encode<'q, sqlx::Postgres> for UserStatus {
    fn encode_by_ref(&self, buf: &mut sqlx::postgres::PgArgumentBuffer) -> Result<sqlx::encode::IsNull, Box<dyn std::error::Error + Send + Sync + 'static>> {
        <&str as sqlx::Encode<sqlx::Postgres>>::encode(&self.to_string(), buf)
    }
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
pub async fn insert_user(pool: &sqlx::PgPool, name: String, email: String, age: i32, profile: serde_json::Value) -> Result<InsertUserResult, sqlx::Error> {
    let mut query = sqlx::query("INSERT INTO users (name, email, age, profile)\nVALUES ($1, $2, $3, $4)\nRETURNING id, name, email, age, created_at\n");
    query = query.bind(&name);
    query = query.bind(&email);
    query = query.bind(age);
    query = query.bind(profile);
    let row = query.fetch_one(pool).await?;
    let result: Result<_, sqlx::Error> = (|| {
        Ok(InsertUserResult {
        id: row.try_get::<i32, _>("id")?,
        name: row.try_get::<String, _>("name")?,
        email: row.try_get::<String, _>("email")?,
        age: row.try_get::<Option<i32>, _>("age")?,
        created_at: row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("created_at")?,
    })
    })();
    result
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
pub async fn get_all_users(pool: &sqlx::PgPool) -> Result<Vec<GetAllUsersResult>, sqlx::Error> {
    let mut query = sqlx::query("SELECT id, name, email, age, profile, created_at, updated_at FROM users ORDER BY created_at DESC");
    let rows = query.fetch_all(pool).await?;
    let result: Result<Vec<_>, sqlx::Error> = rows.iter().map(|row| {
        Ok(GetAllUsersResult {
        id: row.try_get::<i32, _>("id")?,
        name: row.try_get::<String, _>("name")?,
        email: row.try_get::<String, _>("email")?,
        age: row.try_get::<Option<i32>, _>("age")?,
        profile: row.try_get::<Option<serde_json::Value>, _>("profile")?,
        created_at: row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("created_at")?,
        updated_at: row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("updated_at")?,
    })
    }).collect();
    result
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
pub async fn find_user_by_email(pool: &sqlx::PgPool, email: String) -> Result<Option<FindUserByEmailResult>, sqlx::Error> {
    let mut query = sqlx::query("SELECT id, name, email, age, profile, created_at, updated_at FROM users WHERE email = $1");
    query = query.bind(&email);
    let row = query.fetch_optional(pool).await?;
    match row {
        Some(row) => {
            let result: Result<_, sqlx::Error> = (|| {
                Ok(FindUserByEmailResult {
        id: row.try_get::<i32, _>("id")?,
        name: row.try_get::<String, _>("name")?,
        email: row.try_get::<String, _>("email")?,
        age: row.try_get::<Option<i32>, _>("age")?,
        profile: row.try_get::<Option<serde_json::Value>, _>("profile")?,
        created_at: row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("created_at")?,
        updated_at: row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("updated_at")?,
    })
            })();
            result.map(Some)
        },
        None => Ok(None),
    }
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
pub async fn update_user_profile(pool: &sqlx::PgPool, profile: serde_json::Value, user_id: i32) -> Result<UpdateUserProfileResult, sqlx::Error> {
    let mut query = sqlx::query("UPDATE users SET profile = $1, updated_at = NOW() WHERE id = $2 RETURNING id, name, email, age, profile, updated_at");
    query = query.bind(profile);
    query = query.bind(user_id);
    let row = query.fetch_one(pool).await?;
    let result: Result<_, sqlx::Error> = (|| {
        Ok(UpdateUserProfileResult {
        id: row.try_get::<i32, _>("id")?,
        name: row.try_get::<String, _>("name")?,
        email: row.try_get::<String, _>("email")?,
        age: row.try_get::<Option<i32>, _>("age")?,
        profile: row.try_get::<Option<serde_json::Value>, _>("profile")?,
        updated_at: row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("updated_at")?,
    })
    })();
    result
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
pub async fn find_users_by_name_and_age(pool: &sqlx::PgPool, name_pattern: String, min_age: Option<i32>) -> Result<Vec<FindUsersByNameAndAgeResult>, sqlx::Error> {
    let mut query = sqlx::query("SELECT id, name, email, age FROM users WHERE name ILIKE $1 AND ($2::integer IS NULL OR age >= $3)");
    query = query.bind(&name_pattern);
    query = query.bind(min_age);
    query = query.bind(min_age);
    let rows = query.fetch_all(pool).await?;
    let result: Result<Vec<_>, sqlx::Error> = rows.iter().map(|row| {
        Ok(FindUsersByNameAndAgeResult {
        id: row.try_get::<i32, _>("id")?,
        name: row.try_get::<String, _>("name")?,
        email: row.try_get::<String, _>("email")?,
        age: row.try_get::<Option<i32>, _>("age")?,
    })
    }).collect();
    result
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
pub async fn get_recent_users(pool: &sqlx::PgPool, since: chrono::DateTime<chrono::Utc>) -> Result<Vec<GetRecentUsersResult>, sqlx::Error> {
    let mut query = sqlx::query("SELECT id, name, email, age, profile, created_at, updated_at FROM users WHERE created_at > $1 ORDER BY created_at DESC");
    query = query.bind(since);
    let rows = query.fetch_all(pool).await?;
    if rows.is_empty() {
        return Err(sqlx::Error::RowNotFound);
    }
    let result: Result<Vec<_>, sqlx::Error> = rows.iter().map(|row| {
        Ok(GetRecentUsersResult {
        id: row.try_get::<i32, _>("id")?,
        name: row.try_get::<String, _>("name")?,
        email: row.try_get::<String, _>("email")?,
        age: row.try_get::<Option<i32>, _>("age")?,
        profile: row.try_get::<Option<serde_json::Value>, _>("profile")?,
        created_at: row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("created_at")?,
        updated_at: row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("updated_at")?,
    })
    }).collect();
    result
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
pub async fn get_active_users_by_age_range(pool: &sqlx::PgPool, min_age: i32, max_age: i32) -> Result<Vec<GetActiveUsersByAgeRangeResult>, sqlx::Error> {
    let mut query = sqlx::query("SELECT id, name, email, age, profile, created_at FROM users WHERE age BETWEEN $1 AND $2 AND updated_at > NOW() - INTERVAL '30 days'");
    query = query.bind(min_age);
    query = query.bind(max_age);
    let rows = query.fetch_all(pool).await?;
    if rows.is_empty() {
        return Err(sqlx::Error::RowNotFound);
    }
    let result: Result<Vec<_>, sqlx::Error> = rows.iter().map(|row| {
        Ok(GetActiveUsersByAgeRangeResult {
        id: row.try_get::<i32, _>("id")?,
        name: row.try_get::<String, _>("name")?,
        email: row.try_get::<String, _>("email")?,
        age: row.try_get::<Option<i32>, _>("age")?,
        profile: row.try_get::<Option<serde_json::Value>, _>("profile")?,
        created_at: row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("created_at")?,
    })
    }).collect();
    result
}

#[derive(Debug, Clone)]
pub struct SearchUsersByNamePatternResult {
    pub id: i32,
    pub name: String,
    pub email: String,
}

/// Search users by name pattern - expects at least one match
/// Generated from SQL: SELECT id, name, email FROM users WHERE name ILIKE ${pattern} ORDER BY name
pub async fn search_users_by_name_pattern(pool: &sqlx::PgPool, pattern: String) -> Result<Vec<SearchUsersByNamePatternResult>, sqlx::Error> {
    let mut query = sqlx::query("SELECT id, name, email FROM users WHERE name ILIKE $1 ORDER BY name");
    query = query.bind(&pattern);
    let rows = query.fetch_all(pool).await?;
    if rows.is_empty() {
        return Err(sqlx::Error::RowNotFound);
    }
    let result: Result<Vec<_>, sqlx::Error> = rows.iter().map(|row| {
        Ok(SearchUsersByNamePatternResult {
        id: row.try_get::<i32, _>("id")?,
        name: row.try_get::<String, _>("name")?,
        email: row.try_get::<String, _>("email")?,
    })
    }).collect();
    result
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
pub async fn get_users_by_status(pool: &sqlx::PgPool, user_status: UserStatus) -> Result<Vec<GetUsersByStatusResult>, sqlx::Error> {
    let mut query = sqlx::query("SELECT id, name, email, status FROM users WHERE status = $1 ORDER BY name");
    query = query.bind(user_status);
    let rows = query.fetch_all(pool).await?;
    let result: Result<Vec<_>, sqlx::Error> = rows.iter().map(|row| {
        Ok(GetUsersByStatusResult {
        id: row.try_get::<i32, _>("id")?,
        name: row.try_get::<String, _>("name")?,
        email: row.try_get::<String, _>("email")?,
        status: row.try_get::<Option<UserStatus>, _>("status")?,
    })
    }).collect();
    result
}

#[derive(Debug, Clone)]
pub struct UpdateUserStatusResult {
    pub id: i32,
    pub status: Option<UserStatus>,
}

/// Update user status and return the new status
/// Generated from SQL: UPDATE users SET status = ${new_status} WHERE id = ${user_id} RETURNING id, status
pub async fn update_user_status(pool: &sqlx::PgPool, new_status: UserStatus, user_id: i32) -> Result<UpdateUserStatusResult, sqlx::Error> {
    let mut query = sqlx::query("UPDATE users SET status = $1 WHERE id = $2 RETURNING id, status");
    query = query.bind(new_status);
    query = query.bind(user_id);
    let row = query.fetch_one(pool).await?;
    let result: Result<_, sqlx::Error> = (|| {
        Ok(UpdateUserStatusResult {
        id: row.try_get::<i32, _>("id")?,
        status: row.try_get::<Option<UserStatus>, _>("status")?,
    })
    })();
    result
}

/// Get all possible user statuses currently in use
/// Generated from SQL: SELECT DISTINCT status FROM users ORDER BY status
pub async fn get_all_user_statuses(pool: &sqlx::PgPool) -> Result<Vec<Option<UserStatus>>, sqlx::Error> {
    let mut query = sqlx::query("SELECT DISTINCT status FROM users ORDER BY status");
    let rows = query.fetch_all(pool).await?;
    let result: Result<Vec<_>, sqlx::Error> = rows.iter().map(|row| {
        Ok(row.try_get::<Option<UserStatus>, _>("status")?)
    }).collect();
    result
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
pub async fn get_all_users_with_star(pool: &sqlx::PgPool) -> Result<Vec<GetAllUsersWithStarResult>, sqlx::Error> {
    let mut query = sqlx::query("SELECT * FROM users ORDER BY created_at DESC");
    let rows = query.fetch_all(pool).await?;
    let result: Result<Vec<_>, sqlx::Error> = rows.iter().map(|row| {
        Ok(GetAllUsersWithStarResult {
        id: row.try_get::<i32, _>("id")?,
        name: row.try_get::<String, _>("name")?,
        email: row.try_get::<String, _>("email")?,
        age: row.try_get::<Option<i32>, _>("age")?,
        profile: row.try_get::<Option<serde_json::Value>, _>("profile")?,
        created_at: row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("created_at")?,
        updated_at: row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("updated_at")?,
        status: row.try_get::<Option<UserStatus>, _>("status")?,
        referrer_id: row.try_get::<Option<i32>, _>("referrer_id")?,
    })
    }).collect();
    result
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
pub async fn get_user_by_id_with_star(pool: &sqlx::PgPool, user_id: i32) -> Result<Option<GetUserByIdWithStarResult>, sqlx::Error> {
    let mut query = sqlx::query("SELECT * FROM users WHERE id = $1");
    query = query.bind(user_id);
    let row = query.fetch_optional(pool).await?;
    match row {
        Some(row) => {
            let result: Result<_, sqlx::Error> = (|| {
                Ok(GetUserByIdWithStarResult {
        id: row.try_get::<i32, _>("id")?,
        name: row.try_get::<String, _>("name")?,
        email: row.try_get::<String, _>("email")?,
        age: row.try_get::<Option<i32>, _>("age")?,
        profile: row.try_get::<Option<serde_json::Value>, _>("profile")?,
        created_at: row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("created_at")?,
        updated_at: row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("updated_at")?,
        status: row.try_get::<Option<UserStatus>, _>("status")?,
        referrer_id: row.try_get::<Option<i32>, _>("referrer_id")?,
    })
            })();
            result.map(Some)
        },
        None => Ok(None),
    }
}

