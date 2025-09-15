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
/// Generated from SQL:
/// WITH recent_users AS (
/// SELECT id, name, email, created_at,
/// ROW_NUMBER() OVER (ORDER BY created_at DESC) as rank
/// FROM users
/// WHERE created_at > NOW() - INTERVAL '30 days'
/// ),
/// user_stats AS (
/// SELECT
/// COUNT(*) as total_users,
/// COUNT(CASE WHEN created_at > NOW() - INTERVAL '7 days' THEN 1 END) as weekly_users,
/// AVG(age)::float8 as avg_age
/// FROM users
/// )
/// SELECT
/// ru.id,
/// ru.name,
/// ru.email,
/// ru.created_at,
/// ru.rank,
/// us.total_users,
/// us.weekly_users,
/// us.avg_age
/// FROM recent_users ru
/// CROSS JOIN user_stats us
/// WHERE ru.rank <= 10
/// ORDER BY ru.rank
pub async fn get_user_activity_summary(client: &tokio_postgres::Client) -> Result<Vec<GetUserActivitySummaryResult>, tokio_postgres::Error> {
    let stmt = client.prepare("WITH recent_users AS (\n  SELECT id, name, email, created_at,\n         ROW_NUMBER() OVER (ORDER BY created_at DESC) as rank\n  FROM users \n  WHERE created_at > NOW() - INTERVAL '30 days'\n),\nuser_stats AS (\n  SELECT \n    COUNT(*) as total_users,\n    COUNT(CASE WHEN created_at > NOW() - INTERVAL '7 days' THEN 1 END) as weekly_users,\n    AVG(age)::float8 as avg_age\n  FROM users\n)\nSELECT \n  ru.id,\n  ru.name, \n  ru.email,\n  ru.created_at,\n  ru.rank,\n  us.total_users,\n  us.weekly_users,\n  us.avg_age\nFROM recent_users ru\nCROSS JOIN user_stats us\nWHERE ru.rank <= 10\nORDER BY ru.rank\n").await?;
    let rows = client.query(&stmt, &[]).await?;
    let result = rows.into_iter().map(|row| {
        GetUserActivitySummaryResult {
        id: row.get::<_, i32>(0),
        name: row.get::<_, String>(1),
        email: row.get::<_, String>(2),
        created_at: row.get::<_, Option<chrono::DateTime<chrono::Utc>>>(3),
        rank: row.get::<_, Option<i64>>(4),
        total_users: row.get::<_, Option<i64>>(5),
        weekly_users: row.get::<_, Option<i64>>(6),
        avg_age: row.get::<_, Option<f64>>(7),
    }
    }).collect();
    Ok(result)
}

#[derive(Debug, Clone)]
pub struct GetHierarchicalUserDataResult {
    pub id: Option<i32>,
    pub name: Option<String>,
    pub email: Option<String>,
    pub referrer_id: Option<i32>,
    pub level: Option<i32>,
    pub path: /* Unknown type: _int4 */ String,
    pub direct_referrals_count: Option<i64>,
}

/// Recursive CTE to build user hierarchy with referral relationships
/// Generated from SQL:
/// WITH RECURSIVE user_hierarchy AS (
/// -- Base case: users without referrers (or top-level users)
/// SELECT
/// id,
/// name,
/// email,
/// NULL::integer as referrer_id,
/// 1 as level,
/// ARRAY[id] as path
/// FROM users
/// WHERE referrer_id IS NULL
/// 
/// UNION ALL
/// 
/// -- Recursive case: users with referrers
/// SELECT
/// u.id,
/// u.name,
/// u.email,
/// u.referrer_id,
/// uh.level + 1,
/// uh.path || u.id
/// FROM users u
/// INNER JOIN user_hierarchy uh ON u.referrer_id = uh.id
/// WHERE u.id != ALL(uh.path) -- Prevent cycles
/// AND uh.level < 5 -- Limit depth
/// )
/// SELECT
/// uh.id,
/// uh.name,
/// uh.email,
/// uh.referrer_id,
/// uh.level,
/// uh.path,
/// COUNT(referrals.id) as direct_referrals_count
/// FROM user_hierarchy uh
/// LEFT JOIN users referrals ON referrals.referrer_id = uh.id
/// GROUP BY uh.id, uh.name, uh.email, uh.referrer_id, uh.level, uh.path
/// ORDER BY uh.level, uh.name
pub async fn get_hierarchical_user_data(client: &tokio_postgres::Client) -> Result<Vec<GetHierarchicalUserDataResult>, tokio_postgres::Error> {
    let stmt = client.prepare("WITH RECURSIVE user_hierarchy AS (\n  -- Base case: users without referrers (or top-level users)\n  SELECT \n    id, \n    name, \n    email, \n    NULL::integer as referrer_id,\n    1 as level,\n    ARRAY[id] as path\n  FROM users \n  WHERE referrer_id IS NULL\n  \n  UNION ALL\n  \n  -- Recursive case: users with referrers\n  SELECT \n    u.id,\n    u.name,\n    u.email,\n    u.referrer_id,\n    uh.level + 1,\n    uh.path || u.id\n  FROM users u\n  INNER JOIN user_hierarchy uh ON u.referrer_id = uh.id\n  WHERE u.id != ALL(uh.path) -- Prevent cycles\n  AND uh.level < 5 -- Limit depth\n)\nSELECT \n  uh.id,\n  uh.name,\n  uh.email,\n  uh.referrer_id,\n  uh.level,\n  uh.path,\n  COUNT(referrals.id) as direct_referrals_count\nFROM user_hierarchy uh\nLEFT JOIN users referrals ON referrals.referrer_id = uh.id\nGROUP BY uh.id, uh.name, uh.email, uh.referrer_id, uh.level, uh.path\nORDER BY uh.level, uh.name\n").await?;
    let rows = client.query(&stmt, &[]).await?;
    let result = rows.into_iter().map(|row| {
        GetHierarchicalUserDataResult {
        id: row.get::<_, Option<i32>>(0),
        name: row.get::<_, Option<String>>(1),
        email: row.get::<_, Option<String>>(2),
        referrer_id: row.get::<_, Option<i32>>(3),
        level: row.get::<_, Option<i32>>(4),
        path: row.get::<_, /* Unknown type: _int4 */ String>(5),
        direct_referrals_count: row.get::<_, Option<i64>>(6),
    }
    }).collect();
    Ok(result)
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
/// Generated from SQL:
/// SELECT
/// u.id as user_id,
/// u.name,
/// u.email,
/// u.created_at as user_created_at,
/// u.updated_at as user_updated_at,
/// p.id as post_id,
/// p.title,
/// p.content,
/// p.created_at as post_created_at,
/// p.published_at,
/// c.comment_count,
/// EXTRACT(EPOCH FROM (NOW() - p.created_at))::float8/3600 as hours_since_post,
/// DATE_TRUNC('day', p.created_at) as post_date
/// FROM users u
/// INNER JOIN posts p ON u.id = p.author_id
/// LEFT JOIN (
/// SELECT post_id, COUNT(*) as comment_count
/// FROM comments
/// GROUP BY post_id
/// ) c ON p.id = c.post_id
/// WHERE u.created_at > ${since}
/// AND p.published_at IS NOT NULL
/// AND p.created_at BETWEEN ${start_date} AND ${end_date}
/// ORDER BY p.created_at DESC, u.name
pub async fn get_user_activity_with_posts(client: &tokio_postgres::Client, since: chrono::DateTime<chrono::Utc>, start_date: chrono::DateTime<chrono::Utc>, end_date: chrono::DateTime<chrono::Utc>) -> Result<Vec<GetUserActivityWithPostsResult>, tokio_postgres::Error> {
    let stmt = client.prepare("SELECT \n  u.id as user_id,\n  u.name,\n  u.email,\n  u.created_at as user_created_at,\n  u.updated_at as user_updated_at,\n  p.id as post_id,\n  p.title,\n  p.content,\n  p.created_at as post_created_at,\n  p.published_at,\n  c.comment_count,\n  EXTRACT(EPOCH FROM (NOW() - p.created_at))::float8/3600 as hours_since_post,\n  DATE_TRUNC('day', p.created_at) as post_date\nFROM users u\nINNER JOIN posts p ON u.id = p.author_id\nLEFT JOIN (\n  SELECT post_id, COUNT(*) as comment_count\n  FROM comments \n  GROUP BY post_id\n) c ON p.id = c.post_id\nWHERE u.created_at > $1\n  AND p.published_at IS NOT NULL\n  AND p.created_at BETWEEN $2 AND $3\nORDER BY p.created_at DESC, u.name\n").await?;
    let rows = client.query(&stmt, &[&since, &start_date, &end_date]).await?;
    let result = rows.into_iter().map(|row| {
        GetUserActivityWithPostsResult {
        user_id: row.get::<_, i32>(0),
        name: row.get::<_, String>(1),
        email: row.get::<_, String>(2),
        user_created_at: row.get::<_, Option<chrono::DateTime<chrono::Utc>>>(3),
        user_updated_at: row.get::<_, Option<chrono::DateTime<chrono::Utc>>>(4),
        post_id: row.get::<_, i32>(5),
        title: row.get::<_, String>(6),
        content: row.get::<_, Option<String>>(7),
        post_created_at: row.get::<_, Option<chrono::DateTime<chrono::Utc>>>(8),
        published_at: row.get::<_, Option<chrono::DateTime<chrono::Utc>>>(9),
        comment_count: row.get::<_, Option<i64>>(10),
        hours_since_post: row.get::<_, Option<f64>>(11),
        post_date: row.get::<_, Option<chrono::DateTime<chrono::Utc>>>(12),
    }
    }).collect();
    Ok(result)
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
/// Generated from SQL:
/// WITH user_activity AS (
/// SELECT
/// u.id,
/// u.name,
/// u.email,
/// u.created_at,
/// COUNT(DISTINCT p.id) as post_count,
/// COUNT(DISTINCT c.id) as comment_count,
/// MAX(p.created_at) as last_post_date,
/// MAX(c.created_at) as last_comment_date,
/// AVG(EXTRACT(EPOCH FROM (p.published_at - p.created_at))::float8/3600) as avg_publish_delay_hours
/// FROM users u
/// LEFT JOIN posts p ON u.id = p.author_id
/// AND p.created_at >= DATE_TRUNC('month', NOW()) - INTERVAL '3 months'
/// LEFT JOIN comments c ON u.id = c.author_id
/// AND c.created_at >= DATE_TRUNC('month', NOW()) - INTERVAL '3 months'
/// GROUP BY u.id, u.name, u.email, u.created_at
/// ),
/// engagement_scores AS (
/// SELECT
/// *,
/// (post_count * 3 + comment_count) as engagement_score,
/// CASE
/// WHEN last_post_date > NOW() - INTERVAL '7 days' OR
/// last_comment_date > NOW() - INTERVAL '7 days' THEN 'active'
/// WHEN last_post_date > NOW() - INTERVAL '30 days' OR
/// last_comment_date > NOW() - INTERVAL '30 days' THEN 'semi_active'
/// ELSE 'inactive'
/// END as activity_status,
/// EXTRACT(EPOCH FROM (NOW() - GREATEST(
/// COALESCE(last_post_date, '1970-01-01'::timestamp),
/// COALESCE(last_comment_date, '1970-01-01'::timestamp)
/// )))::float8/86400 as days_since_last_activity
/// FROM user_activity
/// )
/// SELECT
/// es.*,
/// RANK() OVER (ORDER BY engagement_score DESC) as engagement_rank,
/// PERCENT_RANK() OVER (ORDER BY engagement_score) as engagement_percentile
/// FROM engagement_scores es
/// WHERE engagement_score > ${min_engagement_score}
/// ORDER BY engagement_score DESC, name
/// LIMIT ${limit_results}
pub async fn get_user_engagement_metrics(client: &tokio_postgres::Client, min_engagement_score: i64, limit_results: i64) -> Result<Vec<GetUserEngagementMetricsResult>, tokio_postgres::Error> {
    let stmt = client.prepare("WITH user_activity AS (\n  SELECT \n    u.id,\n    u.name,\n    u.email,\n    u.created_at,\n    COUNT(DISTINCT p.id) as post_count,\n    COUNT(DISTINCT c.id) as comment_count,\n    MAX(p.created_at) as last_post_date,\n    MAX(c.created_at) as last_comment_date,\n    AVG(EXTRACT(EPOCH FROM (p.published_at - p.created_at))::float8/3600) as avg_publish_delay_hours\n  FROM users u\n  LEFT JOIN posts p ON u.id = p.author_id \n    AND p.created_at >= DATE_TRUNC('month', NOW()) - INTERVAL '3 months'\n  LEFT JOIN comments c ON u.id = c.author_id \n    AND c.created_at >= DATE_TRUNC('month', NOW()) - INTERVAL '3 months'\n  GROUP BY u.id, u.name, u.email, u.created_at\n),\nengagement_scores AS (\n  SELECT \n    *,\n    (post_count * 3 + comment_count) as engagement_score,\n    CASE \n      WHEN last_post_date > NOW() - INTERVAL '7 days' OR \n           last_comment_date > NOW() - INTERVAL '7 days' THEN 'active'\n      WHEN last_post_date > NOW() - INTERVAL '30 days' OR \n           last_comment_date > NOW() - INTERVAL '30 days' THEN 'semi_active'\n      ELSE 'inactive'\n    END as activity_status,\n    EXTRACT(EPOCH FROM (NOW() - GREATEST(\n      COALESCE(last_post_date, '1970-01-01'::timestamp), \n      COALESCE(last_comment_date, '1970-01-01'::timestamp)\n    )))::float8/86400 as days_since_last_activity\n  FROM user_activity\n)\nSELECT \n  es.*,\n  RANK() OVER (ORDER BY engagement_score DESC) as engagement_rank,\n  PERCENT_RANK() OVER (ORDER BY engagement_score) as engagement_percentile\nFROM engagement_scores es\nWHERE engagement_score > $1\nORDER BY engagement_score DESC, name\nLIMIT $2\n").await?;
    let rows = client.query(&stmt, &[&min_engagement_score, &limit_results]).await?;
    let result = rows.into_iter().map(|row| {
        GetUserEngagementMetricsResult {
        id: row.get::<_, i32>(0),
        name: row.get::<_, String>(1),
        email: row.get::<_, String>(2),
        created_at: row.get::<_, Option<chrono::DateTime<chrono::Utc>>>(3),
        post_count: row.get::<_, Option<i64>>(4),
        comment_count: row.get::<_, Option<i64>>(5),
        last_post_date: row.get::<_, Option<chrono::DateTime<chrono::Utc>>>(6),
        last_comment_date: row.get::<_, Option<chrono::DateTime<chrono::Utc>>>(7),
        avg_publish_delay_hours: row.get::<_, Option<f64>>(8),
        engagement_score: row.get::<_, Option<i64>>(9),
        activity_status: row.get::<_, Option<String>>(10),
        days_since_last_activity: row.get::<_, Option<f64>>(11),
        engagement_rank: row.get::<_, Option<i64>>(12),
        engagement_percentile: row.get::<_, Option<f64>>(13),
    }
    }).collect();
    Ok(result)
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
/// Generated from SQL:
/// WITH time_series AS (
/// SELECT
/// DATE_TRUNC('day', created_at) as period_start,
/// COUNT(*) as registrations_count,
/// COUNT(*) FILTER (WHERE age BETWEEN 18 AND 30) as young_adult_count,
/// COUNT(*) FILTER (WHERE age BETWEEN 31 AND 50) as middle_aged_count,
/// COUNT(*) FILTER (WHERE age > 50) as senior_count,
/// AVG(age) as avg_age,
/// MIN(created_at) as first_registration,
/// MAX(created_at) as last_registration
/// FROM users
/// WHERE created_at BETWEEN ${start_date} AND ${end_date}
/// GROUP BY DATE_TRUNC('day', created_at)
/// HAVING COUNT(*) >= ${min_registrations}
/// )
/// SELECT
/// *,
/// EXTRACT(EPOCH FROM (last_registration - first_registration))::float8/3600 as period_span_hours
/// FROM time_series
/// ORDER BY period_start DESC
pub async fn get_time_series_user_registrations(client: &tokio_postgres::Client, start_date: chrono::DateTime<chrono::Utc>, end_date: chrono::DateTime<chrono::Utc>, min_registrations: i64) -> Result<Vec<GetTimeSeriesUserRegistrationsResult>, tokio_postgres::Error> {
    let stmt = client.prepare("WITH time_series AS (\n  SELECT \n    DATE_TRUNC('day', created_at) as period_start,\n    COUNT(*) as registrations_count,\n    COUNT(*) FILTER (WHERE age BETWEEN 18 AND 30) as young_adult_count,\n    COUNT(*) FILTER (WHERE age BETWEEN 31 AND 50) as middle_aged_count, \n    COUNT(*) FILTER (WHERE age > 50) as senior_count,\n    AVG(age) as avg_age,\n    MIN(created_at) as first_registration,\n    MAX(created_at) as last_registration\n  FROM users\n  WHERE created_at BETWEEN $1 AND $2\n  GROUP BY DATE_TRUNC('day', created_at)\n  HAVING COUNT(*) >= $3\n)\nSELECT \n  *,\n  EXTRACT(EPOCH FROM (last_registration - first_registration))::float8/3600 as period_span_hours\nFROM time_series\nORDER BY period_start DESC\n").await?;
    let rows = client.query(&stmt, &[&start_date, &end_date, &min_registrations]).await?;
    let result = rows.into_iter().map(|row| {
        GetTimeSeriesUserRegistrationsResult {
        period_start: row.get::<_, Option<chrono::DateTime<chrono::Utc>>>(0),
        registrations_count: row.get::<_, Option<i64>>(1),
        young_adult_count: row.get::<_, Option<i64>>(2),
        middle_aged_count: row.get::<_, Option<i64>>(3),
        senior_count: row.get::<_, Option<i64>>(4),
        avg_age: row.get::<_, Option<rust_decimal::Decimal>>(5),
        first_registration: row.get::<_, Option<chrono::DateTime<chrono::Utc>>>(6),
        last_registration: row.get::<_, Option<chrono::DateTime<chrono::Utc>>>(7),
        period_span_hours: row.get::<_, Option<f64>>(8),
    }
    }).collect();
    Ok(result)
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
    pub account_age: /* Unknown type: interval */ String,
    pub account_age_days: Option<rust_decimal::Decimal>,
    pub created_day_of_week: Option<f64>,
    pub created_hour: Option<f64>,
    pub formatted_created_at: Option<String>,
}

/// Users with comprehensive timezone and temporal information
/// Generated from SQL:
/// SELECT
/// id,
/// name,
/// email,
/// created_at,
/// created_at AT TIME ZONE 'UTC' AT TIME ZONE ${user_timezone} as created_at_user_tz,
/// updated_at,
/// updated_at AT TIME ZONE 'UTC' AT TIME ZONE ${user_timezone} as updated_at_user_tz,
/// AGE(NOW(), created_at) as account_age,
/// EXTRACT(EPOCH FROM AGE(NOW(), created_at))/86400 as account_age_days,
/// DATE_PART('dow', created_at) as created_day_of_week,
/// DATE_PART('hour', created_at) as created_hour,
/// TO_CHAR(created_at, 'Day, Month DD, YYYY at HH24:MI:SS TZ') as formatted_created_at
/// FROM users
/// WHERE created_at BETWEEN ${start_date} AND ${end_date}
/// AND EXTRACT(EPOCH FROM AGE(NOW(), created_at))/86400 BETWEEN ${min_age_days} AND ${max_age_days}
/// ORDER BY created_at DESC
pub async fn get_users_with_timezone_info(client: &tokio_postgres::Client, user_timezone: String, start_date: chrono::DateTime<chrono::Utc>, end_date: chrono::DateTime<chrono::Utc>, min_age_days: rust_decimal::Decimal, max_age_days: rust_decimal::Decimal) -> Result<Vec<GetUsersWithTimezoneInfoResult>, tokio_postgres::Error> {
    let stmt = client.prepare("SELECT \n  id,\n  name,\n  email,\n  created_at,\n  created_at AT TIME ZONE 'UTC' AT TIME ZONE $1 as created_at_user_tz,\n  updated_at,\n  updated_at AT TIME ZONE 'UTC' AT TIME ZONE $2 as updated_at_user_tz,\n  AGE(NOW(), created_at) as account_age,\n  EXTRACT(EPOCH FROM AGE(NOW(), created_at))/86400 as account_age_days,\n  DATE_PART('dow', created_at) as created_day_of_week,\n  DATE_PART('hour', created_at) as created_hour,\n  TO_CHAR(created_at, 'Day, Month DD, YYYY at HH24:MI:SS TZ') as formatted_created_at\nFROM users \nWHERE created_at BETWEEN $3 AND $4\n  AND EXTRACT(EPOCH FROM AGE(NOW(), created_at))/86400 BETWEEN $5 AND $6\nORDER BY created_at DESC\n").await?;
    let rows = client.query(&stmt, &[&user_timezone, &user_timezone, &start_date, &end_date, &min_age_days, &max_age_days]).await?;
    let result = rows.into_iter().map(|row| {
        GetUsersWithTimezoneInfoResult {
        id: row.get::<_, i32>(0),
        name: row.get::<_, String>(1),
        email: row.get::<_, String>(2),
        created_at: row.get::<_, Option<chrono::DateTime<chrono::Utc>>>(3),
        created_at_user_tz: row.get::<_, Option<chrono::DateTime<chrono::Utc>>>(4),
        updated_at: row.get::<_, Option<chrono::DateTime<chrono::Utc>>>(5),
        updated_at_user_tz: row.get::<_, Option<chrono::DateTime<chrono::Utc>>>(6),
        account_age: row.get::<_, /* Unknown type: interval */ String>(7),
        account_age_days: row.get::<_, Option<rust_decimal::Decimal>>(8),
        created_day_of_week: row.get::<_, Option<f64>>(9),
        created_hour: row.get::<_, Option<f64>>(10),
        formatted_created_at: row.get::<_, Option<String>>(11),
    }
    }).collect();
    Ok(result)
}

