#[allow(dead_code)]
mod generated;
mod models;

use sqlx::PgPool;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get database URL from environment
    let database_url = env::var("AUTOMODEL_DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:massword@localhost/postgres".to_string());

    // Connect to database
    match connect_to_database(&database_url).await {
        Ok(pool) => {
            println!("✓ Connected to database");
            run_examples(&pool).await?;
        }
        Err(e) => {
            println!("✗ Failed to connect to database: {}", e);
            println!("To run this example:");
            println!("1. Start a PostgreSQL database");
            println!("2. Run the sql queries in the ./migrations to create necessary tables");
            println!("3. Set AUTOMODEL_DATABASE_URL environment variable");
            println!("4. Run: cargo run");
        }
    }

    Ok(())
}

async fn connect_to_database(database_url: &str) -> Result<PgPool, Box<dyn std::error::Error>> {
    let pool = PgPool::connect(database_url).await?;
    Ok(pool)
}

async fn run_examples(pool: &PgPool) -> Result<(), Box<dyn std::error::Error>> {
    println!("\nRunning example queries...");

    // Test basic queries
    println!("\n=== Testing Basic Queries ===");
    test_basic_queries(pool).await?;

    // Test conditional update - only update fields that are provided
    println!("\n=== Testing Conditional Update ===");
    test_conditional_update(pool).await?;

    // Test conditional update with diff
    println!("\n=== Testing Conditional Update with Diff ===");
    test_conditional_update_diff(pool).await?;

    // Test structured parameters
    println!("\n=== Testing Structured Parameters ===");
    test_structured_parameters(pool).await?;

    // Test struct reuse
    println!("\n=== Testing Struct Reuse ===");
    test_struct_reuse(pool).await?;

    // Test all PostgreSQL types
    println!("\n=== Testing All PostgreSQL Types ===");
    test_all_types(pool).await?;

    println!("\nTo see the actual generated code, check src/generated/ directory");
    println!("Functions are organized into modules: admin.rs, setup.rs, users.rs, and mod.rs");
    println!(
        "The code is regenerated automatically when the build runs after queries.yaml changes!"
    );

    Ok(())
}

async fn test_basic_queries(pool: &PgPool) -> Result<(), Box<dyn std::error::Error>> {
    // Admin functions
    match generated::admin::get_current_time(pool).await {
        Ok(time) => println!("Current time: {:?}", time),
        Err(e) => println!("Error getting time: {}", e),
    }

    // Setup functions
    match generated::setup::create_users_table(pool).await {
        Ok(_) => println!("Users table created successfully"),
        Err(e) => println!("Error creating table: {}", e),
    }

    // Users functions
    match generated::users::get_all_users(pool).await {
        Ok(users) => println!("All users: {:?}", users),
        Err(e) => println!("Error listing users: {}", e),
    }

    Ok(())
}

async fn test_conditional_update(pool: &PgPool) -> Result<(), Box<dyn std::error::Error>> {
    println!("Demonstrating conditional UPDATE with optional parameters...");

    // First, insert a test user with a unique email
    println!("\n1. Inserting a test user...");
    let timestamp = chrono::Utc::now().timestamp();
    let user = generated::users::insert_user(
        pool,
        "John Doe".to_string(),
        format!("john.doe.{}@example.com", timestamp),
        30,
        models::UserProfile {
            bio: Some("Test user for conditional update demo".to_string()),
            avatar_url: None,
            preferences: models::UserPreferences {
                theme: "dark".to_string(),
                language: "en".to_string(),
                notifications_enabled: true,
            },
            social_links: vec![],
        },
    )
    .await?;
    println!(
        "✓ Created user: ID={}, name={}, email={}, age={:?}",
        user.id, user.name, user.email, user.age
    );

    // Example 1: Update only the name
    println!("\n2. Updating only the name (email and age remain unchanged)...");
    let updated = generated::users::update_user_fields(
        pool,
        Some("Jane Doe".to_string()),
        None, // email not updated
        None, // age not updated
        user.id,
    )
    .await?;
    println!(
        "✓ After updating name: ID={}, name={}, email={}, age={:?}",
        updated.id, updated.name, updated.email, updated.age
    );

    // Example 2: Update only the age
    println!("\n3. Updating only the age (name and email remain unchanged)...");
    let updated = generated::users::update_user_fields(
        pool,
        None, // name not updated
        None, // email not updated
        Some(35),
        user.id,
    )
    .await?;
    println!(
        "✓ After updating age: ID={}, name={}, email={}, age={:?}",
        updated.id, updated.name, updated.email, updated.age
    );

    // Example 3: Update multiple fields at once
    println!("\n4. Updating both name and email (age remains unchanged)...");
    let unique_email = format!("jane.smith.{}@example.com", timestamp);
    let updated = generated::users::update_user_fields(
        pool,
        Some("Jane Smith".to_string()),
        Some(unique_email),
        None, // age not updated
        user.id,
    )
    .await?;
    println!(
        "✓ After updating name and email: ID={}, name={}, email={}, age={:?}",
        updated.id, updated.name, updated.email, updated.age
    );

    // Example 4: Update all fields
    println!("\n5. Updating all fields at once...");
    let unique_email2 = format!("janet.williams.{}@example.com", timestamp);
    let updated = generated::users::update_user_fields(
        pool,
        Some("Janet Williams".to_string()),
        Some(unique_email2),
        Some(40),
        user.id,
    )
    .await?;
    println!(
        "✓ After updating all fields: ID={}, name={}, email={}, age={:?}",
        updated.id, updated.name, updated.email, updated.age
    );

    println!("\n✓ Conditional update examples completed successfully!");
    println!("The conditional syntax $[, field = ${{param?}}] only includes the SET clause when the parameter is Some(value)");

    Ok(())
}

async fn test_conditional_update_diff(pool: &PgPool) -> Result<(), Box<dyn std::error::Error>> {
    println!("Demonstrating conditional UPDATE with diff-based comparison...");

    // First, insert a test user with a unique email
    println!("\n1. Inserting a test user...");
    let timestamp = chrono::Utc::now().timestamp();
    let email = format!("alice.cooper.{}@example.com", timestamp);
    let user = generated::users::insert_user(
        pool,
        "Alice Cooper".to_string(),
        email.clone(),
        28,
        models::UserProfile {
            bio: Some("Test user for diff-based conditional update demo".to_string()),
            avatar_url: None,
            preferences: models::UserPreferences {
                theme: "light".to_string(),
                language: "en".to_string(),
                notifications_enabled: true,
            },
            social_links: vec![],
        },
    )
    .await?;
    println!(
        "✓ Created user: ID={}, name={}, email={}, age={:?}",
        user.id, user.name, user.email, user.age
    );

    // Example 1: Update only the name (by passing different old/new values)
    println!("\n2. Updating only the name using diff (email and age stay the same)...");
    let old = generated::users::UpdateUserFieldsDiffParams {
        name: "Alice Cooper".to_string(),
        email: email.clone(),
        age: 28,
    };
    let new = generated::users::UpdateUserFieldsDiffParams {
        name: "Alice Smith".to_string(), // Changed
        email: email.clone(),            // Same
        age: 28,                         // Same
    };
    let updated = generated::users::update_user_fields_diff(pool, &old, &new, user.id).await?;
    println!(
        "✓ After updating name: ID={}, name={}, email={}, age={:?}",
        updated.id, updated.name, updated.email, updated.age
    );

    // Example 2: Update only the age
    println!("\n3. Updating only the age using diff (name and email stay the same)...");
    let old = generated::users::UpdateUserFieldsDiffParams {
        name: "Alice Smith".to_string(),
        email: email.clone(),
        age: 28,
    };
    let new = generated::users::UpdateUserFieldsDiffParams {
        name: "Alice Smith".to_string(), // Same
        email: email.clone(),            // Same
        age: 29,                         // Changed
    };
    let updated = generated::users::update_user_fields_diff(pool, &old, &new, user.id).await?;
    println!(
        "✓ After updating age: ID={}, name={}, email={}, age={:?}",
        updated.id, updated.name, updated.email, updated.age
    );

    // Example 3: Update multiple fields
    println!("\n4. Updating both name and email using diff (age stays the same)...");
    let email2 = format!("alicia.smith.{}@example.com", timestamp);
    let old = generated::users::UpdateUserFieldsDiffParams {
        name: "Alice Smith".to_string(),
        email: email.clone(),
        age: 29,
    };
    let new = generated::users::UpdateUserFieldsDiffParams {
        name: "Alicia Smith".to_string(), // Changed
        email: email2.clone(),            // Changed
        age: 29,                          // Same
    };
    let updated = generated::users::update_user_fields_diff(pool, &old, &new, user.id).await?;
    println!(
        "✓ After updating name and email: ID={}, name={}, email={}, age={:?}",
        updated.id, updated.name, updated.email, updated.age
    );

    // Example 4: Update all fields
    println!("\n5. Updating all fields using diff...");
    let email3 = format!("alicia.johnson.{}@example.com", timestamp);
    let old = generated::users::UpdateUserFieldsDiffParams {
        name: "Alicia Smith".to_string(),
        email: email2.clone(),
        age: 29,
    };
    let new = generated::users::UpdateUserFieldsDiffParams {
        name: "Alicia Johnson".to_string(), // Changed
        email: email3,                      // Changed
        age: 30,                            // Changed
    };
    let updated = generated::users::update_user_fields_diff(pool, &old, &new, user.id).await?;
    println!(
        "✓ After updating all fields: ID={}, name={}, email={}, age={:?}",
        updated.id, updated.name, updated.email, updated.age
    );

    println!("\n✓ Diff-based conditional update examples completed successfully!");
    println!("With conditional_diff: true, the function compares old.field != new.field to decide which SET clauses to include");

    Ok(())
}

async fn test_structured_parameters(pool: &PgPool) -> Result<(), Box<dyn std::error::Error>> {
    println!("Demonstrating structured parameters...");

    // Create a params struct and insert a user
    println!("\n1. Inserting a user using structured parameters...");
    let timestamp = chrono::Utc::now().timestamp();
    let params = generated::users::InsertUserStructuredParams {
        name: "Bob Builder".to_string(),
        email: format!("bob.builder.{}@example.com", timestamp),
        age: 42,
    };

    let user = generated::users::insert_user_structured(pool, &params).await?;
    println!(
        "✓ Created user: ID={}, name={}, email={}, age={:?}",
        user.id, user.name, user.email, user.age
    );

    // Demonstrate reusing the struct
    println!("\n2. Inserting another user with modified params...");
    let params2 = generated::users::InsertUserStructuredParams {
        name: "Alice Builder".to_string(),
        email: format!("alice.builder.{}@example.com", timestamp),
        age: 38,
    };

    let user2 = generated::users::insert_user_structured(pool, &params2).await?;
    println!(
        "✓ Created user: ID={}, name={}, email={}, age={:?}",
        user2.id, user2.name, user2.email, user2.age
    );

    println!("\n✓ Structured parameters examples completed successfully!");
    println!("With structured_parameters: true, all query parameters are passed as a single struct instead of individual parameters");

    Ok(())
}

async fn test_struct_reuse(pool: &PgPool) -> Result<(), Box<dyn std::error::Error>> {
    println!("Demonstrating struct reuse across queries...");
    println!("This feature allows queries to reuse structs defined by previous queries,");
    println!("eliminating code duplication when queries share the same parameter structure.\n");

    let timestamp = chrono::Utc::now().timestamp();

    // Example 1: Reusing a Params struct
    println!("1. Query with structured_parameters: true generates GetUserByIdAndEmailParams");
    let email = format!("struct.reuse.{}@example.com", timestamp);

    // First insert a test user
    let user = generated::users::insert_user(
        pool,
        "Struct Reuse Test".to_string(),
        email.clone(),
        35,
        models::UserProfile {
            bio: Some("Test user for struct reuse demo".to_string()),
            avatar_url: None,
            preferences: models::UserPreferences {
                theme: "light".to_string(),
                language: "en".to_string(),
                notifications_enabled: true,
            },
            social_links: vec![],
        },
    )
    .await?;
    println!(
        "   ✓ Inserted test user: ID={}, email={}",
        user.id, user.email
    );

    // Use get_user_by_id_and_email which generates GetUserByIdAndEmailParams
    let params = generated::users::GetUserByIdAndEmailParams {
        id: user.id,
        email: email.clone(),
    };

    match generated::users::get_user_by_id_and_email(pool, &params).await? {
        Some(found) => println!(
            "   ✓ Found user: ID={}, name={}, email={}",
            found.id, found.name, found.email
        ),
        None => println!("   ✗ User not found"),
    }

    // Example 2: Reusing the same Params struct in delete query
    println!("\n2. Another query reuses GetUserByIdAndEmailParams (structured_parameters: \"GetUserByIdAndEmailParams\")");
    println!("   No new struct is generated - it reuses the existing one!");

    let deleted = generated::users::delete_user_by_id_and_email(pool, &params).await?;
    println!(
        "   ✓ Deleted user: ID={}, email={}",
        deleted.id, deleted.email
    );

    // Example 3: Reusing a return type struct (Item) as params
    println!("\n3. Reusing a return type struct (GetUserByIdAndEmailItem) as params");

    // Insert another test user
    let email2 = format!("struct.item.reuse.{}@example.com", timestamp);
    let user2 = generated::users::insert_user(
        pool,
        "Item Struct Reuse".to_string(),
        email2.clone(),
        40,
        models::UserProfile {
            bio: Some("Test user for item struct reuse demo".to_string()),
            avatar_url: None,
            preferences: models::UserPreferences {
                theme: "dark".to_string(),
                language: "en".to_string(),
                notifications_enabled: false,
            },
            social_links: vec![],
        },
    )
    .await?;
    println!(
        "   ✓ Inserted test user: ID={}, email={}",
        user2.id, user2.email
    );

    // Get the user (returns GetUserByIdAndEmailItem with id, name, email fields)
    let get_params = generated::users::GetUserByIdAndEmailParams {
        id: user2.id,
        email: email2.clone(),
    };

    let user_item = generated::users::get_user_by_id_and_email(pool, &get_params)
        .await?
        .expect("User should exist");
    println!(
        "   ✓ Retrieved user item: ID={}, name={}, email={}",
        user_item.id, user_item.name, user_item.email
    );

    // Update using the same struct type (GetUserByIdAndEmailItem)
    // This demonstrates reusing a return type struct as input params
    let update_params = generated::users::GetUserByIdAndEmailItem {
        id: user_item.id,
        name: "Updated Name".to_string(),
        email: user_item.email.clone(),
    };

    let updated = generated::users::update_user_contact_info(pool, &update_params).await?;
    println!(
        "   ✓ Updated user: ID={}, name={}, email={}",
        updated.id, updated.name, updated.email
    );

    println!("\n✓ Struct reuse examples completed successfully!");
    println!("\nKey benefits:");
    println!("  • Reduces code duplication - one struct definition serves multiple queries");
    println!("  • Type safety - compiler ensures fields match across queries");
    println!("  • Can reuse both Params structs and Item (return type) structs");
    println!("  • Validation at build time ensures referenced structs exist and match");

    Ok(())
}

async fn test_all_types(pool: &PgPool) -> Result<(), Box<dyn std::error::Error>> {
    use chrono::{NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};
    use rust_decimal::Decimal;
    use sqlx::postgres::types::{PgInterval, PgRange, PgTimeTz};
    use std::str::FromStr;
    use uuid::Uuid;

    println!("Inserting test row with all PostgreSQL types...");

    // Prepare test data
    let bool_col = true;
    let char_col = "A".to_string(); // CHAR(1) in PostgreSQL
    let int2_col: i16 = 32767;
    let int4_col: i32 = 2147483647;
    let int8_col: i64 = 9223372036854775807;
    let float4_col: f32 = 3.14159;
    let float8_col: f64 = 2.718281828459045;
    let numeric_col = Decimal::from_str("12345.67")?;

    let name_col = "test_name".to_string();
    let text_col = "This is a test text".to_string();
    let varchar_col = "varchar test".to_string();
    let bpchar_col = "bpchar    ".to_string(); // Will be padded to 10 chars

    let bytea_col = vec![0xDE, 0xAD, 0xBE, 0xEF];
    // Bit types using bit_vec::BitVec
    let mut bit_col = bit_vec::BitVec::from_elem(8, false);
    bit_col.set(0, true);
    bit_col.set(2, true);
    bit_col.set(4, true);
    bit_col.set(6, true);

    let mut varbit_col = bit_vec::BitVec::from_elem(16, false);
    varbit_col.set(0, true);
    varbit_col.set(2, true);
    varbit_col.set(4, true);
    varbit_col.set(6, true);
    varbit_col.set(8, true);
    varbit_col.set(10, true);
    varbit_col.set(12, true);
    varbit_col.set(14, true);

    let date_col = NaiveDate::from_ymd_opt(2025, 11, 20).unwrap();
    let time_col = NaiveTime::from_hms_opt(14, 30, 0).unwrap();
    let timestamp_col = NaiveDateTime::parse_from_str("2025-11-20 14:30:00", "%Y-%m-%d %H:%M:%S")?;
    let timestamptz_col = Utc.with_ymd_and_hms(2025, 11, 20, 14, 30, 0).unwrap();
    let interval_col = PgInterval {
        months: 0,
        days: 1,
        microseconds: (2 * 3600 + 30 * 60) * 1_000_000,
    };
    let timetz_col = PgTimeTz {
        time: NaiveTime::from_hms_opt(14, 30, 0).unwrap(),
        offset: chrono::FixedOffset::east_opt(0).unwrap(),
    };

    // Range types
    let int4_range_col =
        PgRange::from((std::ops::Bound::Included(1), std::ops::Bound::Excluded(10)));
    let int8_range_col = PgRange::from((
        std::ops::Bound::Included(100i64),
        std::ops::Bound::Included(200i64),
    ));
    let num_range_col = PgRange::from((
        std::ops::Bound::Included(Decimal::from_str("0.5")?),
        std::ops::Bound::Included(Decimal::from_str("99.9")?),
    ));
    let ts_range_col = PgRange::from((
        std::ops::Bound::Included(
            NaiveDate::from_ymd_opt(2025, 1, 1)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap(),
        ),
        std::ops::Bound::Included(
            NaiveDate::from_ymd_opt(2025, 12, 31)
                .unwrap()
                .and_hms_opt(23, 59, 59)
                .unwrap(),
        ),
    ));
    let tstz_range_col = PgRange::from((
        std::ops::Bound::Included(Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap()),
        std::ops::Bound::Included(Utc.with_ymd_and_hms(2025, 12, 31, 23, 59, 59).unwrap()),
    ));
    let date_range_col = PgRange::from((
        std::ops::Bound::Included(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap()),
        std::ops::Bound::Included(NaiveDate::from_ymd_opt(2025, 12, 31).unwrap()),
    ));

    // Network types - using std::net::IpAddr as sqlx maps INET/CIDR to IpAddr with ipnet feature
    let inet_col: std::net::IpAddr = "192.168.1.1".parse()?;
    let cidr_col: std::net::IpAddr = "192.168.1.0".parse()?;
    let macaddr_col = mac_address::MacAddress::new([0x08, 0x00, 0x2b, 0x01, 0x02, 0x03]);

    // JSON types
    let json_col = serde_json::json!({"key": "value", "number": 42});
    let jsonb_col = serde_json::json!({"name": "test", "tags": ["tag1", "tag2"]});

    // UUID
    let uuid_col = Uuid::parse_str("a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a11")?;

    // Array types
    let bool_array_col = vec![true, false, true];
    let int4_array_col = vec![1, 2, 3, 4, 5];
    let int8_array_col = vec![100i64, 200i64, 300i64];
    let text_array_col = vec!["one".to_string(), "two".to_string(), "three".to_string()];
    let float8_array_col = vec![1.1, 2.2, 3.3];

    // Range array types
    let int4_range_array_col = vec![
        PgRange::from((std::ops::Bound::Included(1), std::ops::Bound::Excluded(5))),
        PgRange::from((std::ops::Bound::Included(10), std::ops::Bound::Excluded(20))),
    ];
    let date_range_array_col = vec![
        PgRange::from((
            std::ops::Bound::Included(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap()),
            std::ops::Bound::Included(NaiveDate::from_ymd_opt(2025, 1, 31).unwrap()),
        )),
        PgRange::from((
            std::ops::Bound::Included(NaiveDate::from_ymd_opt(2025, 6, 1).unwrap()),
            std::ops::Bound::Included(NaiveDate::from_ymd_opt(2025, 6, 30).unwrap()),
        )),
    ];

    // Insert the test row
    let result = generated::admin::insert_all_types_test(
        pool,
        bool_col,
        char_col.clone(),
        int2_col,
        int4_col,
        int8_col,
        float4_col,
        float8_col,
        numeric_col,
        name_col.clone(),
        text_col.clone(),
        varchar_col.clone(),
        bpchar_col.clone(),
        bytea_col.clone(),
        bit_col.clone(),
        varbit_col.clone(),
        date_col,
        time_col,
        timestamp_col,
        timestamptz_col,
        interval_col,
        timetz_col,
        int4_range_col,
        int8_range_col,
        num_range_col,
        ts_range_col,
        tstz_range_col,
        date_range_col,
        inet_col,
        cidr_col,
        macaddr_col,
        json_col.clone(),
        jsonb_col.clone(),
        uuid_col,
        bool_array_col.clone(),
        int4_array_col.clone(),
        int8_array_col.clone(),
        text_array_col.clone(),
        float8_array_col.clone(),
        int4_range_array_col.clone(),
        date_range_array_col.clone(),
    )
    .await?;

    println!("✓ Inserted row with ID: {}", result);

    // Read back the inserted row
    println!("Reading back the inserted row...");
    let retrieved = generated::admin::get_all_types_test(pool, result).await?;

    println!("✓ Successfully retrieved row:");
    println!("  ID: {:?}", retrieved.id);
    println!("  Boolean: {:?}", retrieved.bool_col);
    println!("  Int4: {:?}", retrieved.int4_col);
    println!("  Int8: {:?}", retrieved.int8_col);
    println!("  Float8: {:?}", retrieved.float8_col);
    println!("  Text: {:?}", retrieved.text_col);
    println!("  UUID: {:?}", retrieved.uuid_col);
    println!("  JSON: {:?}", retrieved.jsonb_col);
    println!("  Date: {:?}", retrieved.date_col);
    println!("  Timestamp: {:?}", retrieved.timestamp_col);
    println!("  Int4 Range: {:?}", retrieved.int4_range_col);
    println!("  Date Range: {:?}", retrieved.date_range_col);
    println!("  Int4 Array: {:?}", retrieved.int4_array_col);
    println!("  Text Array: {:?}", retrieved.text_array_col);
    println!("  Int4 Range Array: {:?}", retrieved.int4_range_array_col);
    println!("  Date Range Array: {:?}", retrieved.date_range_array_col);

    println!("\n✓ All PostgreSQL types test completed successfully!");

    Ok(())
}
