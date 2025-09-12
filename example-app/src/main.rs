mod generated;

use std::env;
use tokio_postgres::{Client, NoTls};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get database URL from environment
    let database_url =
        env::var("DATABASE_URL").unwrap_or_else(|_| "postgresql://localhost/test".to_string());

    println!("Example App - Using AutoModel Generated Functions");
    println!("==============================================");

    // Connect to database
    match connect_to_database(&database_url).await {
        Ok(client) => {
            println!("✓ Connected to database");
            run_examples(&client).await?;
        }
        Err(e) => {
            println!("✗ Failed to connect to database: {}", e);
            println!("To run this example:");
            println!("1. Start a PostgreSQL database");
            println!("2. Run the schema.sql file to create tables");
            println!("3. Set DATABASE_URL environment variable");
            println!("4. Run: cargo run");
        }
    }

    Ok(())
}

async fn connect_to_database(database_url: &str) -> Result<Client, Box<dyn std::error::Error>> {
    let (client, connection) = tokio_postgres::connect(database_url, NoTls).await?;

    // Spawn the connection in the background
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("Database connection error: {}", e);
        }
    });

    Ok(client)
}

async fn run_examples(_client: &Client) -> Result<(), Box<dyn std::error::Error>> {
    println!("\nRunning example queries...");

    // Note: The actual generated functions would be used here
    // For now, we'll show how they would be called:

    println!("Generated functions available:");
    println!("- get_user_by_id(client, id)");
    println!("- list_active_users(client)");
    println!("- create_user(client, name, email, is_active)");
    println!("- update_user_email(client, id, email)");
    println!("- delete_user(client, id)");

    // Example of how you would use the generated functions:
    /*
    match generated::get_user_by_id(client, 1).await {
        Ok(user) => println!("Found user: {:?}", user),
        Err(e) => println!("Error getting user: {}", e),
    }

    match generated::list_active_users(client).await {
        Ok(users) => println!("Active users: {:?}", users),
        Err(e) => println!("Error listing users: {}", e),
    }
    */

    println!("\nTo see the actual generated code, check src/generated.rs");
    println!("The code is regenerated automatically when queries.yaml changes!");

    Ok(())
}
