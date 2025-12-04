#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let defaults = automodel::DefaultsConfig {
        telemetry: automodel::DefaultsTelemetryConfig {
            level: automodel::TelemetryLevel::Debug,
            include_sql: true,
        },
        ensure_indexes: true,
    };
    automodel::AutoModel::generate(
        || {
            if std::env::var("CI").is_err() {
                std::env::var("AUTOMODEL_DATABASE_URL").map_err(|_| {
                    "AUTOMODEL_DATABASE_URL environment variable must be set for code generation"
                        .to_string()
                })
            } else {
                Err(
                    "Detecting not up to date AutoModel generated code in CI environment"
                        .to_string(),
                )
            }
        },
        "queries",
        "src/generated",
        defaults,
    )
    .await
}
