use automodel::AutoModel;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Running code generation...");
    AutoModel::generate_at_build_time("queries.yaml", "src/generated").await?;
    println!("Code generation completed!");
    Ok(())
}