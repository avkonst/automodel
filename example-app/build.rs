use automodel::generate_at_build_time;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    generate_at_build_time("queries.yaml", "src/generated.rs").await?;

    Ok(())
}
