use automodel::AutoModel;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    AutoModel::generate_at_build_time("queries.yaml", "src/generated.rs").await?;

    Ok(())
}
