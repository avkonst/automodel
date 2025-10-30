use sqlx::Row;

#[derive(Debug, Clone)]
pub struct GetUserActivitySummaryItem {
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
pub async fn get_user_activity_summary(executor: impl sqlx::Executor<'_, Database = sqlx::Postgres>) -> Result<Vec<GetUserActivitySummaryItem>, sqlx::Error> {
    let query = sqlx::query(
        r"WITH recent_users AS (
          SELECT id, name, email, created_at,
                 ROW_NUMBER() OVER (ORDER BY created_at DESC) as rank
          FROM users 
          WHERE created_at > NOW() - INTERVAL '30 days'
        ),
        user_stats AS (
          SELECT 
            COUNT(*) as total_users,
            COUNT(CASE WHEN created_at > NOW() - INTERVAL '7 days' THEN 1 END) as weekly_users,
            AVG(age)::float8 as avg_age
          FROM users
        )
        SELECT 
          ru.id,
          ru.name, 
          ru.email,
          ru.created_at,
          ru.rank,
          us.total_users,
          us.weekly_users,
          us.avg_age
        FROM recent_users ru
        CROSS JOIN user_stats us
        WHERE ru.rank <= 10
        ORDER BY ru.rank"
    );
    let rows = query.fetch_all(executor).await?;
    let result: Result<Vec<_>, sqlx::Error> = rows.iter().map(|row| {
        Ok(GetUserActivitySummaryItem {
        id: row.try_get::<i32, _>("id")?,
        name: row.try_get::<String, _>("name")?,
        email: row.try_get::<String, _>("email")?,
        created_at: row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("created_at")?,
        rank: row.try_get::<Option<i64>, _>("rank")?,
        total_users: row.try_get::<Option<i64>, _>("total_users")?,
        weekly_users: row.try_get::<Option<i64>, _>("weekly_users")?,
        avg_age: row.try_get::<Option<f64>, _>("avg_age")?,
    })
    }).collect();
    result
}

#[derive(Debug, Clone)]
pub struct GetHierarchicalUserDataItem {
    pub id: Option<i32>,
    pub name: Option<String>,
    pub email: Option<String>,
    pub referrer_id: Option<i32>,
    pub level: Option<i32>,
    pub path: Option</* Unknown type: _int4 */ String>,
    pub direct_referrals_count: Option<i64>,
}

/// Recursive CTE to build user hierarchy with referral relationships
pub async fn get_hierarchical_user_data(executor: impl sqlx::Executor<'_, Database = sqlx::Postgres>) -> Result<Vec<GetHierarchicalUserDataItem>, sqlx::Error> {
    let query = sqlx::query(
        r"WITH RECURSIVE user_hierarchy AS (
          -- Base case: users without referrers (or top-level users)
          SELECT 
            id, 
            name, 
            email, 
            NULL::integer as referrer_id,
            1 as level,
            ARRAY[id] as path
          FROM users 
          WHERE referrer_id IS NULL
          
          UNION ALL
          
          -- Recursive case: users with referrers
          SELECT 
            u.id,
            u.name,
            u.email,
            u.referrer_id,
            uh.level + 1,
            uh.path || u.id
          FROM users u
          INNER JOIN user_hierarchy uh ON u.referrer_id = uh.id
          WHERE u.id != ALL(uh.path) -- Prevent cycles
          AND uh.level < 5 -- Limit depth
        )
        SELECT 
          uh.id,
          uh.name,
          uh.email,
          uh.referrer_id,
          uh.level,
          uh.path,
          COUNT(referrals.id) as direct_referrals_count
        FROM user_hierarchy uh
        LEFT JOIN users referrals ON referrals.referrer_id = uh.id
        GROUP BY uh.id, uh.name, uh.email, uh.referrer_id, uh.level, uh.path
        ORDER BY uh.level, uh.name"
    );
    let rows = query.fetch_all(executor).await?;
    let result: Result<Vec<_>, sqlx::Error> = rows.iter().map(|row| {
        Ok(GetHierarchicalUserDataItem {
        id: row.try_get::<Option<i32>, _>("id")?,
        name: row.try_get::<Option<String>, _>("name")?,
        email: row.try_get::<Option<String>, _>("email")?,
        referrer_id: row.try_get::<Option<i32>, _>("referrer_id")?,
        level: row.try_get::<Option<i32>, _>("level")?,
        path: row.try_get::<Option</* Unknown type: _int4 */ String>, _>("path")?,
        direct_referrals_count: row.try_get::<Option<i64>, _>("direct_referrals_count")?,
    })
    }).collect();
    result
}

#[derive(Debug, Clone)]
pub struct GetUserActivityWithPostsItem {
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
pub async fn get_user_activity_with_posts(executor: impl sqlx::Executor<'_, Database = sqlx::Postgres>, since: chrono::DateTime<chrono::Utc>, start_date: chrono::DateTime<chrono::Utc>, end_date: chrono::DateTime<chrono::Utc>) -> Result<Vec<GetUserActivityWithPostsItem>, sqlx::Error> {
    let query = sqlx::query(
        r"SELECT 
          u.id as user_id,
          u.name,
          u.email,
          u.created_at as user_created_at,
          u.updated_at as user_updated_at,
          p.id as post_id,
          p.title,
          p.content,
          p.created_at as post_created_at,
          p.published_at,
          c.comment_count,
          EXTRACT(EPOCH FROM (NOW() - p.created_at))::float8/3600 as hours_since_post,
          DATE_TRUNC('day', p.created_at) as post_date
        FROM users u
        INNER JOIN posts p ON u.id = p.author_id
        LEFT JOIN (
          SELECT post_id, COUNT(*) as comment_count
          FROM comments 
          GROUP BY post_id
        ) c ON p.id = c.post_id
        WHERE u.created_at > $1
          AND p.published_at IS NOT NULL
          AND p.created_at BETWEEN $2 AND $3
        ORDER BY p.created_at DESC, u.name"
    );
    let query = query.bind(since);
    let query = query.bind(start_date);
    let query = query.bind(end_date);
    let rows = query.fetch_all(executor).await?;
    let result: Result<Vec<_>, sqlx::Error> = rows.iter().map(|row| {
        Ok(GetUserActivityWithPostsItem {
        user_id: row.try_get::<i32, _>("user_id")?,
        name: row.try_get::<String, _>("name")?,
        email: row.try_get::<String, _>("email")?,
        user_created_at: row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("user_created_at")?,
        user_updated_at: row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("user_updated_at")?,
        post_id: row.try_get::<i32, _>("post_id")?,
        title: row.try_get::<String, _>("title")?,
        content: row.try_get::<Option<String>, _>("content")?,
        post_created_at: row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("post_created_at")?,
        published_at: row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("published_at")?,
        comment_count: row.try_get::<Option<i64>, _>("comment_count")?,
        hours_since_post: row.try_get::<Option<f64>, _>("hours_since_post")?,
        post_date: row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("post_date")?,
    })
    }).collect();
    result
}

#[derive(Debug, Clone)]
pub struct GetUserEngagementMetricsItem {
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
pub async fn get_user_engagement_metrics(executor: impl sqlx::Executor<'_, Database = sqlx::Postgres>, min_engagement_score: i64, limit_results: i64) -> Result<Vec<GetUserEngagementMetricsItem>, sqlx::Error> {
    let query = sqlx::query(
        r"WITH user_activity AS (
          SELECT 
            u.id,
            u.name,
            u.email,
            u.created_at,
            COUNT(DISTINCT p.id) as post_count,
            COUNT(DISTINCT c.id) as comment_count,
            MAX(p.created_at) as last_post_date,
            MAX(c.created_at) as last_comment_date,
            AVG(EXTRACT(EPOCH FROM (p.published_at - p.created_at))::float8/3600) as avg_publish_delay_hours
          FROM users u
          LEFT JOIN posts p ON u.id = p.author_id 
            AND p.created_at >= DATE_TRUNC('month', NOW()) - INTERVAL '3 months'
          LEFT JOIN comments c ON u.id = c.author_id 
            AND c.created_at >= DATE_TRUNC('month', NOW()) - INTERVAL '3 months'
          GROUP BY u.id, u.name, u.email, u.created_at
        ),
        engagement_scores AS (
          SELECT 
            *,
            (post_count * 3 + comment_count) as engagement_score,
            CASE 
              WHEN last_post_date > NOW() - INTERVAL '7 days' OR 
                   last_comment_date > NOW() - INTERVAL '7 days' THEN 'active'
              WHEN last_post_date > NOW() - INTERVAL '30 days' OR 
                   last_comment_date > NOW() - INTERVAL '30 days' THEN 'semi_active'
              ELSE 'inactive'
            END as activity_status,
            EXTRACT(EPOCH FROM (NOW() - GREATEST(
              COALESCE(last_post_date, '1970-01-01'::timestamp), 
              COALESCE(last_comment_date, '1970-01-01'::timestamp)
            )))::float8/86400 as days_since_last_activity
          FROM user_activity
        )
        SELECT 
          es.*,
          RANK() OVER (ORDER BY engagement_score DESC) as engagement_rank,
          PERCENT_RANK() OVER (ORDER BY engagement_score) as engagement_percentile
        FROM engagement_scores es
        WHERE engagement_score > $1
        ORDER BY engagement_score DESC, name
        LIMIT $2"
    );
    let query = query.bind(min_engagement_score);
    let query = query.bind(limit_results);
    let rows = query.fetch_all(executor).await?;
    let result: Result<Vec<_>, sqlx::Error> = rows.iter().map(|row| {
        Ok(GetUserEngagementMetricsItem {
        id: row.try_get::<i32, _>("id")?,
        name: row.try_get::<String, _>("name")?,
        email: row.try_get::<String, _>("email")?,
        created_at: row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("created_at")?,
        post_count: row.try_get::<Option<i64>, _>("post_count")?,
        comment_count: row.try_get::<Option<i64>, _>("comment_count")?,
        last_post_date: row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("last_post_date")?,
        last_comment_date: row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("last_comment_date")?,
        avg_publish_delay_hours: row.try_get::<Option<f64>, _>("avg_publish_delay_hours")?,
        engagement_score: row.try_get::<Option<i64>, _>("engagement_score")?,
        activity_status: row.try_get::<Option<String>, _>("activity_status")?,
        days_since_last_activity: row.try_get::<Option<f64>, _>("days_since_last_activity")?,
        engagement_rank: row.try_get::<Option<i64>, _>("engagement_rank")?,
        engagement_percentile: row.try_get::<Option<f64>, _>("engagement_percentile")?,
    })
    }).collect();
    result
}

#[derive(Debug, Clone)]
pub struct GetTimeSeriesUserRegistrationsItem {
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
pub async fn get_time_series_user_registrations(executor: impl sqlx::Executor<'_, Database = sqlx::Postgres>, start_date: chrono::DateTime<chrono::Utc>, end_date: chrono::DateTime<chrono::Utc>, min_registrations: i64) -> Result<Vec<GetTimeSeriesUserRegistrationsItem>, sqlx::Error> {
    let query = sqlx::query(
        r"WITH time_series AS (
          SELECT 
            DATE_TRUNC('day', created_at) as period_start,
            COUNT(*) as registrations_count,
            COUNT(*) FILTER (WHERE age BETWEEN 18 AND 30) as young_adult_count,
            COUNT(*) FILTER (WHERE age BETWEEN 31 AND 50) as middle_aged_count, 
            COUNT(*) FILTER (WHERE age > 50) as senior_count,
            AVG(age) as avg_age,
            MIN(created_at) as first_registration,
            MAX(created_at) as last_registration
          FROM users
          WHERE created_at BETWEEN $1 AND $2
          GROUP BY DATE_TRUNC('day', created_at)
          HAVING COUNT(*) >= $3
        )
        SELECT 
          *,
          EXTRACT(EPOCH FROM (last_registration - first_registration))::float8/3600 as period_span_hours
        FROM time_series
        ORDER BY period_start DESC"
    );
    let query = query.bind(start_date);
    let query = query.bind(end_date);
    let query = query.bind(min_registrations);
    let rows = query.fetch_all(executor).await?;
    let result: Result<Vec<_>, sqlx::Error> = rows.iter().map(|row| {
        Ok(GetTimeSeriesUserRegistrationsItem {
        period_start: row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("period_start")?,
        registrations_count: row.try_get::<Option<i64>, _>("registrations_count")?,
        young_adult_count: row.try_get::<Option<i64>, _>("young_adult_count")?,
        middle_aged_count: row.try_get::<Option<i64>, _>("middle_aged_count")?,
        senior_count: row.try_get::<Option<i64>, _>("senior_count")?,
        avg_age: row.try_get::<Option<rust_decimal::Decimal>, _>("avg_age")?,
        first_registration: row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("first_registration")?,
        last_registration: row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("last_registration")?,
        period_span_hours: row.try_get::<Option<f64>, _>("period_span_hours")?,
    })
    }).collect();
    result
}

#[derive(Debug, Clone)]
pub struct GetUsersWithTimezoneInfoItem {
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
pub async fn get_users_with_timezone_info(executor: impl sqlx::Executor<'_, Database = sqlx::Postgres>, user_timezone: String, start_date: chrono::DateTime<chrono::Utc>, end_date: chrono::DateTime<chrono::Utc>, min_age_days: rust_decimal::Decimal, max_age_days: rust_decimal::Decimal) -> Result<Vec<GetUsersWithTimezoneInfoItem>, sqlx::Error> {
    let query = sqlx::query(
        r"SELECT 
          id,
          name,
          email,
          created_at,
          created_at AT TIME ZONE 'UTC' AT TIME ZONE $1 as created_at_user_tz,
          updated_at,
          updated_at AT TIME ZONE 'UTC' AT TIME ZONE $2 as updated_at_user_tz,
          AGE(NOW(), created_at) as account_age,
          EXTRACT(EPOCH FROM AGE(NOW(), created_at))/86400 as account_age_days,
          DATE_PART('dow', created_at) as created_day_of_week,
          DATE_PART('hour', created_at) as created_hour,
          TO_CHAR(created_at, 'Day, Month DD, YYYY at HH24:MI:SS TZ') as formatted_created_at
        FROM users 
        WHERE created_at BETWEEN $3 AND $4
          AND EXTRACT(EPOCH FROM AGE(NOW(), created_at))/86400 BETWEEN $5 AND $6
        ORDER BY created_at DESC"
    );
    let query = query.bind(&user_timezone);
    let query = query.bind(&user_timezone);
    let query = query.bind(start_date);
    let query = query.bind(end_date);
    let query = query.bind(min_age_days);
    let query = query.bind(max_age_days);
    let rows = query.fetch_all(executor).await?;
    let result: Result<Vec<_>, sqlx::Error> = rows.iter().map(|row| {
        Ok(GetUsersWithTimezoneInfoItem {
        id: row.try_get::<i32, _>("id")?,
        name: row.try_get::<String, _>("name")?,
        email: row.try_get::<String, _>("email")?,
        created_at: row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("created_at")?,
        created_at_user_tz: row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("created_at_user_tz")?,
        updated_at: row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("updated_at")?,
        updated_at_user_tz: row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("updated_at_user_tz")?,
        account_age: row.try_get::<Option</* Unknown type: interval */ String>, _>("account_age")?,
        account_age_days: row.try_get::<Option<rust_decimal::Decimal>, _>("account_age_days")?,
        created_day_of_week: row.try_get::<Option<f64>, _>("created_day_of_week")?,
        created_hour: row.try_get::<Option<f64>, _>("created_hour")?,
        formatted_created_at: row.try_get::<Option<String>, _>("formatted_created_at")?,
    })
    }).collect();
    result
}

