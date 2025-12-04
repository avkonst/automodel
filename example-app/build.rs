use automodel::{AutoModel, DefaultsConfig, DefaultsTelemetryConfig, TelemetryLevel};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let defaults = DefaultsConfig {
        telemetry: DefaultsTelemetryConfig {
            level: Some(TelemetryLevel::Debug),
            include_sql: Some(true),
        },
        ensure_indexes: Some(true),
        module: None,
    };

    AutoModel::generate_from_queries_dir("queries", "src/generated", defaults).await?;

    Ok(())
}
