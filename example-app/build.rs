use automodel::{AutoModel, DefaultsConfig, DefaultsTelemetryConfig, TelemetryLevel};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let defaults = DefaultsConfig {
        telemetry: DefaultsTelemetryConfig {
            level: Some(TelemetryLevel::Debug),
            include_sql: Some(true),
        },
        ensure_indexes: Some(true),
    };

    AutoModel::generate_at_build_time("queries", "src/generated", defaults).await?;

    Ok(())
}
