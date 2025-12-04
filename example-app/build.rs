#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let defaults = automodel::DefaultsConfig {
        telemetry: automodel::DefaultsTelemetryConfig {
            level: automodel::TelemetryLevel::Debug,
            include_sql: true,
        },
        ensure_indexes: true,
    };
    automodel::AutoModel::generate_at_build_time("queries", "src/generated", defaults).await?;
    Ok(())
}
