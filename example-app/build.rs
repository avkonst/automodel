use automodel_lib::generate_at_build_time;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=queries.yaml");

    generate_at_build_time("queries.yaml", "src/generated.rs").await?;

    Ok(())
}
