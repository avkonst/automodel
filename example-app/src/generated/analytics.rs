
#[derive(Debug, Clone)]
pub struct GetUserActivitySummaryResult {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub rank: Option<i64>,
    pub total_users: Option<i64>,
    pub weekly_users: Option<i64>,
    pub avg_age: Option<f64>,
}

/// Complex CTE query combining recent users with aggregate statistics
pub async fn get_user_activity_summary(pool: &sqlx::PgPool) -> Result<Vec<GetUserActivitySummaryResult>, sqlx::Error> {
    let query = sqlx::query("WITH recent_users AS (\n  SELECT id, name, email, created_at,\n         ROW_NUMBER() OVER (ORDER BY created_at DESC) as rank\n  FROM users \n  WHERE created_at > NOW() - INTERVAL '30 days'\n),\nuser_stats AS (\n  SELECT \n    COUNT(*) as total_users,\n    COUNT(CASE WHEN created_at > NOW() - INTERVAL '7 days' THEN 1 END) as weekly_users,\n    AVG(age)::float8 as avg_age\n  FROM users\n)\nSELECT \n  ru.id,\n  ru.name, \n  ru.email,\n  ru.created_at,\n  ru.rank,\n  us.total_users,\n  us.weekly_users,\n  us.avg_age\nFROM recent_users ru\nCROSS JOIN user_stats us\nWHERE ru.rank <= 10\nORDER BY ru.rank\n");
    let rows = query.fetch_all(pool).await?;
    let result: Result<Vec<_>, sqlx::Error> = rows.iter().map(|row| {
        Ok(GetUserActivitySummaryResult {
        id: sqlx::Row::try_get::<i32, _>(row, "id")?,
        name: sqlx::Row::try_get::<String, _>(row, "name")?,
        email: sqlx::Row::try_get::<String, _>(row, "email")?,
        created_at: sqlx::Row::try_get::<Option<chrono::DateTime<chrono::Utc>>, _>(row, "created_at")?,
        rank: sqlx::Row::try_get::<Option<i64>, _>(row, "rank")?,
        total_users: sqlx::Row::try_get::<Option<i64>, _>(row, "total_users")?,
        weekly_users: sqlx::Row::try_get::<Option<i64>, _>(row, "weekly_users")?,
        avg_age: sqlx::Row::try_get::<Option<f64>, _>(row, "avg_age")?,
    })
    }).collect();
    result
}

#[derive(Debug, Clone)]
pub struct GetHierarchicalUserDataResult {
    pub id: Option<i32>,
    pub name: Option<String>,
    pub email: Option<String>,
    pub referrer_id: Option<i32>,
    pub level: Option<i32>,
    pub path: Option</* Unknown type: _int4 */ String>,
    pub direct_referrals_count: Option<i64>,
}

/// Recursive CTE to build user hierarchy with referral relationships
pub async fn get_hierarchical_user_data(pool: &sqlx::PgPool) -> Result<Vec<GetHierarchicalUserDataResult>, sqlx::Error> {
    let query = sqlx::query("WITH RECURSIVE user_hierarchy AS (\n  -- Base case: users without referrers (or top-level users)\n  SELECT \n    id, \n    name, \n    email, \n    NULL::integer as referrer_id,\n    1 as level,\n    ARRAY[id] as path\n  FROM users \n  WHERE referrer_id IS NULL\n  \n  UNION ALL\n  \n  -- Recursive case: users with referrers\n  SELECT \n    u.id,\n    u.name,\n    u.email,\n    u.referrer_id,\n    uh.level + 1,\n    uh.path || u.id\n  FROM users u\n  INNER JOIN user_hierarchy uh ON u.referrer_id = uh.id\n  WHERE u.id != ALL(uh.path) -- Prevent cycles\n  AND uh.level < 5 -- Limit depth\n)\nSELECT \n  uh.id,\n  uh.name,\n  uh.email,\n  uh.referrer_id,\n  uh.level,\n  uh.path,\n  COUNT(referrals.id) as direct_referrals_count\nFROM user_hierarchy uh\nLEFT JOIN users referrals ON referrals.referrer_id = uh.id\nGROUP BY uh.id, uh.name, uh.email, uh.referrer_id, uh.level, uh.path\nORDER BY uh.level, uh.name\n");
    let rows = query.fetch_all(pool).await?;
    let result: Result<Vec<_>, sqlx::Error> = rows.iter().map(|row| {
        Ok(GetHierarchicalUserDataResult {
        id: sqlx::Row::try_get::<Option<i32>, _>(row, "id")?,
        name: sqlx::Row::try_get::<Option<String>, _>(row, "name")?,
        email: sqlx::Row::try_get::<Option<String>, _>(row, "email")?,
        referrer_id: sqlx::Row::try_get::<Option<i32>, _>(row, "referrer_id")?,
        level: sqlx::Row::try_get::<Option<i32>, _>(row, "level")?,
        path: sqlx::Row::try_get::<Option</* Unknown type: _int4 */ String>, _>(row, "path")?,
        direct_referrals_count: sqlx::Row::try_get::<Option<i64>, _>(row, "direct_referrals_count")?,
    })
    }).collect();
    result
}

#[derive(Debug, Clone)]
pub struct GetUserActivityWithPostsResult {
    pub user_id: i32,
    pub name: String,
    pub email: String,
    pub user_created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub user_updated_at: Option<chrono::DateTime<chrono::Utc>>,
    pub post_id: i32,
    pub title: String,
    pub content: Option<String>,
    pub post_created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub published_at: Option<chrono::DateTime<chrono::Utc>>,
    pub comment_count: Option<i64>,
    pub hours_since_post: Option<f64>,
    pub post_date: Option<chrono::DateTime<chrono::Utc>>,
}

/// Complex JOIN query with temporal filtering across multiple tables
pub async fn get_user_activity_with_posts(pool: &sqlx::PgPool, since: chrono::DateTime<chrono::Utc>, start_date: chrono::DateTime<chrono::Utc>, end_date: chrono::DateTime<chrono::Utc>) -> Result<Vec<GetUserActivityWithPostsResult>, sqlx::Error> {
    let query = sqlx::query("SELECT \n  u.id as user_id,\n  u.name,\n  u.email,\n  u.created_at as user_created_at,\n  u.updated_at as user_updated_at,\n  p.id as post_id,\n  p.title,\n  p.content,\n  p.created_at as post_created_at,\n  p.published_at,\n  c.comment_count,\n  EXTRACT(EPOCH FROM (NOW() - p.created_at))::float8/3600 as hours_since_post,\n  DATE_TRUNC('day', p.created_at) as post_date\nFROM users u\nINNER JOIN posts p ON u.id = p.author_id\nLEFT JOIN (\n  SELECT post_id, COUNT(*) as comment_count\n  FROM comments \n  GROUP BY post_id\n) c ON p.id = c.post_id\nWHERE u.created_at > $1\n  AND p.published_at IS NOT NULL\n  AND p.created_at BETWEEN $2 AND $3\nORDER BY p.created_at DESC, u.name\n");
    let query = query.bind(since);
    let query = query.bind(start_date);
    let query = query.bind(end_date);
    let rows = query.fetch_all(pool).await?;
    let result: Result<Vec<_>, sqlx::Error> = rows.iter().map(|row| {
        Ok(GetUserActivityWithPostsResult {
        user_id: sqlx::Row::try_get::<i32, _>(row, "user_id")?,
        name: sqlx::Row::try_get::<String, _>(row, "name")?,
        email: sqlx::Row::try_get::<String, _>(row, "email")?,
        user_created_at: sqlx::Row::try_get::<Option<chrono::DateTime<chrono::Utc>>, _>(row, "user_created_at")?,
        user_updated_at: sqlx::Row::try_get::<Option<chrono::DateTime<chrono::Utc>>, _>(row, "user_updated_at")?,
        post_id: sqlx::Row::try_get::<i32, _>(row, "post_id")?,
        title: sqlx::Row::try_get::<String, _>(row, "title")?,
        content: sqlx::Row::try_get::<Option<String>, _>(row, "content")?,
        post_created_at: sqlx::Row::try_get::<Option<chrono::DateTime<chrono::Utc>>, _>(row, "post_created_at")?,
        published_at: sqlx::Row::try_get::<Option<chrono::DateTime<chrono::Utc>>, _>(row, "published_at")?,
        comment_count: sqlx::Row::try_get::<Option<i64>, _>(row, "comment_count")?,
        hours_since_post: sqlx::Row::try_get::<Option<f64>, _>(row, "hours_since_post")?,
        post_date: sqlx::Row::try_get::<Option<chrono::DateTime<chrono::Utc>>, _>(row, "post_date")?,
    })
    }).collect();
    result
}

#[derive(Debug, Clone)]
pub struct GetUserEngagementMetricsResult {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub post_count: Option<i64>,
    pub comment_count: Option<i64>,
    pub last_post_date: Option<chrono::DateTime<chrono::Utc>>,
    pub last_comment_date: Option<chrono::DateTime<chrono::Utc>>,
    pub avg_publish_delay_hours: Option<f64>,
    pub engagement_score: Option<i64>,
    pub activity_status: Option<String>,
    pub days_since_last_activity: Option<f64>,
    pub engagement_rank: Option<i64>,
    pub engagement_percentile: Option<f64>,
}

/// Complex multi-CTE query calculating user engagement metrics with temporal analysis
pub async fn get_user_engagement_metrics(pool: &sqlx::PgPool, min_engagement_score: i64, limit_results: i64) -> Result<Vec<GetUserEngagementMetricsResult>, sqlx::Error> {
    let query = sqlx::query("WITH user_activity AS (\n  SELECT \n    u.id,\n    u.name,\n    u.email,\n    u.created_at,\n    COUNT(DISTINCT p.id) as post_count,\n    COUNT(DISTINCT c.id) as comment_count,\n    MAX(p.created_at) as last_post_date,\n    MAX(c.created_at) as last_comment_date,\n    AVG(EXTRACT(EPOCH FROM (p.published_at - p.created_at))::float8/3600) as avg_publish_delay_hours\n  FROM users u\n  LEFT JOIN posts p ON u.id = p.author_id \n    AND p.created_at >= DATE_TRUNC('month', NOW()) - INTERVAL '3 months'\n  LEFT JOIN comments c ON u.id = c.author_id \n    AND c.created_at >= DATE_TRUNC('month', NOW()) - INTERVAL '3 months'\n  GROUP BY u.id, u.name, u.email, u.created_at\n),\nengagement_scores AS (\n  SELECT \n    *,\n    (post_count * 3 + comment_count) as engagement_score,\n    CASE \n      WHEN last_post_date > NOW() - INTERVAL '7 days' OR \n           last_comment_date > NOW() - INTERVAL '7 days' THEN 'active'\n      WHEN last_post_date > NOW() - INTERVAL '30 days' OR \n           last_comment_date > NOW() - INTERVAL '30 days' THEN 'semi_active'\n      ELSE 'inactive'\n    END as activity_status,\n    EXTRACT(EPOCH FROM (NOW() - GREATEST(\n      COALESCE(last_post_date, '1970-01-01'::timestamp), \n      COALESCE(last_comment_date, '1970-01-01'::timestamp)\n    )))::float8/86400 as days_since_last_activity\n  FROM user_activity\n)\nSELECT \n  es.*,\n  RANK() OVER (ORDER BY engagement_score DESC) as engagement_rank,\n  PERCENT_RANK() OVER (ORDER BY engagement_score) as engagement_percentile\nFROM engagement_scores es\nWHERE engagement_score > $1\nORDER BY engagement_score DESC, name\nLIMIT $2\n");
    let query = query.bind(min_engagement_score);
    let query = query.bind(limit_results);
    let rows = query.fetch_all(pool).await?;
    let result: Result<Vec<_>, sqlx::Error> = rows.iter().map(|row| {
        Ok(GetUserEngagementMetricsResult {
        id: sqlx::Row::try_get::<i32, _>(row, "id")?,
        name: sqlx::Row::try_get::<String, _>(row, "name")?,
        email: sqlx::Row::try_get::<String, _>(row, "email")?,
        created_at: sqlx::Row::try_get::<Option<chrono::DateTime<chrono::Utc>>, _>(row, "created_at")?,
        post_count: sqlx::Row::try_get::<Option<i64>, _>(row, "post_count")?,
        comment_count: sqlx::Row::try_get::<Option<i64>, _>(row, "comment_count")?,
        last_post_date: sqlx::Row::try_get::<Option<chrono::DateTime<chrono::Utc>>, _>(row, "last_post_date")?,
        last_comment_date: sqlx::Row::try_get::<Option<chrono::DateTime<chrono::Utc>>, _>(row, "last_comment_date")?,
        avg_publish_delay_hours: sqlx::Row::try_get::<Option<f64>, _>(row, "avg_publish_delay_hours")?,
        engagement_score: sqlx::Row::try_get::<Option<i64>, _>(row, "engagement_score")?,
        activity_status: sqlx::Row::try_get::<Option<String>, _>(row, "activity_status")?,
        days_since_last_activity: sqlx::Row::try_get::<Option<f64>, _>(row, "days_since_last_activity")?,
        engagement_rank: sqlx::Row::try_get::<Option<i64>, _>(row, "engagement_rank")?,
        engagement_percentile: sqlx::Row::try_get::<Option<f64>, _>(row, "engagement_percentile")?,
    })
    }).collect();
    result
}

#[derive(Debug, Clone)]
pub struct GetTimeSeriesUserRegistrationsResult {
    pub period_start: Option<chrono::DateTime<chrono::Utc>>,
    pub registrations_count: Option<i64>,
    pub young_adult_count: Option<i64>,
    pub middle_aged_count: Option<i64>,
    pub senior_count: Option<i64>,
    pub avg_age: Option<rust_decimal::Decimal>,
    pub first_registration: Option<chrono::DateTime<chrono::Utc>>,
    pub last_registration: Option<chrono::DateTime<chrono::Utc>>,
    pub period_span_hours: Option<f64>,
}

/// Time series analysis of user registrations with age demographics
pub async fn get_time_series_user_registrations(pool: &sqlx::PgPool, start_date: chrono::DateTime<chrono::Utc>, end_date: chrono::DateTime<chrono::Utc>, min_registrations: i64) -> Result<Vec<GetTimeSeriesUserRegistrationsResult>, sqlx::Error> {
    let query = sqlx::query("WITH time_series AS (\n  SELECT \n    DATE_TRUNC('day', created_at) as period_start,\n    COUNT(*) as registrations_count,\n    COUNT(*) FILTER (WHERE age BETWEEN 18 AND 30) as young_adult_count,\n    COUNT(*) FILTER (WHERE age BETWEEN 31 AND 50) as middle_aged_count, \n    COUNT(*) FILTER (WHERE age > 50) as senior_count,\n    AVG(age) as avg_age,\n    MIN(created_at) as first_registration,\n    MAX(created_at) as last_registration\n  FROM users\n  WHERE created_at BETWEEN $1 AND $2\n  GROUP BY DATE_TRUNC('day', created_at)\n  HAVING COUNT(*) >= $3\n)\nSELECT \n  *,\n  EXTRACT(EPOCH FROM (last_registration - first_registration))::float8/3600 as period_span_hours\nFROM time_series\nORDER BY period_start DESC\n");
    let query = query.bind(start_date);
    let query = query.bind(end_date);
    let query = query.bind(min_registrations);
    let rows = query.fetch_all(pool).await?;
    let result: Result<Vec<_>, sqlx::Error> = rows.iter().map(|row| {
        Ok(GetTimeSeriesUserRegistrationsResult {
        period_start: sqlx::Row::try_get::<Option<chrono::DateTime<chrono::Utc>>, _>(row, "period_start")?,
        registrations_count: sqlx::Row::try_get::<Option<i64>, _>(row, "registrations_count")?,
        young_adult_count: sqlx::Row::try_get::<Option<i64>, _>(row, "young_adult_count")?,
        middle_aged_count: sqlx::Row::try_get::<Option<i64>, _>(row, "middle_aged_count")?,
        senior_count: sqlx::Row::try_get::<Option<i64>, _>(row, "senior_count")?,
        avg_age: sqlx::Row::try_get::<Option<rust_decimal::Decimal>, _>(row, "avg_age")?,
        first_registration: sqlx::Row::try_get::<Option<chrono::DateTime<chrono::Utc>>, _>(row, "first_registration")?,
        last_registration: sqlx::Row::try_get::<Option<chrono::DateTime<chrono::Utc>>, _>(row, "last_registration")?,
        period_span_hours: sqlx::Row::try_get::<Option<f64>, _>(row, "period_span_hours")?,
    })
    }).collect();
    result
}

#[derive(Debug, Clone)]
pub struct GetUsersWithTimezoneInfoResult {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at_user_tz: Option<chrono::DateTime<chrono::Utc>>,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
    pub updated_at_user_tz: Option<chrono::DateTime<chrono::Utc>>,
    pub account_age: Option</* Unknown type: interval */ String>,
    pub account_age_days: Option<rust_decimal::Decimal>,
    pub created_day_of_week: Option<f64>,
    pub created_hour: Option<f64>,
    pub formatted_created_at: Option<String>,
}

/// Users with comprehensive timezone and temporal information
pub async fn get_users_with_timezone_info(pool: &sqlx::PgPool, user_timezone: String, start_date: chrono::DateTime<chrono::Utc>, end_date: chrono::DateTime<chrono::Utc>, min_age_days: rust_decimal::Decimal, max_age_days: rust_decimal::Decimal) -> Result<Vec<GetUsersWithTimezoneInfoResult>, sqlx::Error> {
    let query = sqlx::query("SELECT \n  id,\n  name,\n  email,\n  created_at,\n  created_at AT TIME ZONE 'UTC' AT TIME ZONE $1 as created_at_user_tz,\n  updated_at,\n  updated_at AT TIME ZONE 'UTC' AT TIME ZONE $2 as updated_at_user_tz,\n  AGE(NOW(), created_at) as account_age,\n  EXTRACT(EPOCH FROM AGE(NOW(), created_at))/86400 as account_age_days,\n  DATE_PART('dow', created_at) as created_day_of_week,\n  DATE_PART('hour', created_at) as created_hour,\n  TO_CHAR(created_at, 'Day, Month DD, YYYY at HH24:MI:SS TZ') as formatted_created_at\nFROM users \nWHERE created_at BETWEEN $3 AND $4\n  AND EXTRACT(EPOCH FROM AGE(NOW(), created_at))/86400 BETWEEN $5 AND $6\nORDER BY created_at DESC\n");
    let query = query.bind(&user_timezone);
    let query = query.bind(&user_timezone);
    let query = query.bind(start_date);
    let query = query.bind(end_date);
    let query = query.bind(min_age_days);
    let query = query.bind(max_age_days);
    let rows = query.fetch_all(pool).await?;
    let result: Result<Vec<_>, sqlx::Error> = rows.iter().map(|row| {
        Ok(GetUsersWithTimezoneInfoResult {
        id: sqlx::Row::try_get::<i32, _>(row, "id")?,
        name: sqlx::Row::try_get::<String, _>(row, "name")?,
        email: sqlx::Row::try_get::<String, _>(row, "email")?,
        created_at: sqlx::Row::try_get::<Option<chrono::DateTime<chrono::Utc>>, _>(row, "created_at")?,
        created_at_user_tz: sqlx::Row::try_get::<Option<chrono::DateTime<chrono::Utc>>, _>(row, "created_at_user_tz")?,
        updated_at: sqlx::Row::try_get::<Option<chrono::DateTime<chrono::Utc>>, _>(row, "updated_at")?,
        updated_at_user_tz: sqlx::Row::try_get::<Option<chrono::DateTime<chrono::Utc>>, _>(row, "updated_at_user_tz")?,
        account_age: sqlx::Row::try_get::<Option</* Unknown type: interval */ String>, _>(row, "account_age")?,
        account_age_days: sqlx::Row::try_get::<Option<rust_decimal::Decimal>, _>(row, "account_age_days")?,
        created_day_of_week: sqlx::Row::try_get::<Option<f64>, _>(row, "created_day_of_week")?,
        created_hour: sqlx::Row::try_get::<Option<f64>, _>(row, "created_hour")?,
        formatted_created_at: sqlx::Row::try_get::<Option<String>, _>(row, "formatted_created_at")?,
    })
    }).collect();
    result
}

