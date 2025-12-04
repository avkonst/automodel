use anyhow::Result;
use automodel::*;
use clap::{Arg, ArgMatches, Command};

#[tokio::main]
async fn main() -> Result<()> {
    let matches = build_cli().get_matches();

    match matches.subcommand() {
        Some(("generate", sub_matches)) => {
            generate_command(sub_matches).await?;
        }
        _ => {
            build_cli().print_help()?;
            std::process::exit(1);
        }
    }

    Ok(())
}

fn build_cli() -> Command {
    Command::new("automodel")
        .version("0.1.0")
        .author("AutoModel Team")
        .about("Generate typed Rust functions from SQL query files")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("generate")
                .about("Generate Rust code from SQL query files")
                .arg(
                    Arg::new("database-url")
                        .short('d')
                        .long("database-url")
                        .value_name("URL")
                        .help("PostgreSQL database connection URL")
                        .required(true),
                )
                .arg(
                    Arg::new("queries-dir")
                        .short('q')
                        .long("queries-dir")
                        .value_name("DIR")
                        .help("Directory containing SQL query files (e.g., 'queries')")
                        .default_value("queries"),
                )
                .arg(
                    Arg::new("output")
                        .short('o')
                        .long("output")
                        .value_name("DIR")
                        .help("Output directory for generated Rust code")
                        .default_value("generated"),
                )
                .arg(
                    Arg::new("telemetry-level")
                        .long("telemetry-level")
                        .value_name("LEVEL")
                        .help("Global telemetry level: none, info, debug, trace")
                        .value_parser(["none", "info", "debug", "trace"])
                        .default_value("none"),
                )
                .arg(
                    Arg::new("telemetry-include-sql")
                        .long("telemetry-include-sql")
                        .help("Include SQL queries in telemetry spans")
                        .action(clap::ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("ensure-indexes")
                        .long("ensure-indexes")
                        .help("Enable query performance analysis and sequential scan detection")
                        .action(clap::ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("default-module")
                        .long("default-module")
                        .value_name("MODULE")
                        .help("Default module name for queries without explicit module"),
                ),
        )
}

async fn generate_command(matches: &ArgMatches) -> Result<()> {
    let database_url = matches.get_one::<String>("database-url").unwrap();
    let queries_dir = matches.get_one::<String>("queries-dir").unwrap();
    let output_dir = matches.get_one::<String>("output").unwrap();

    // Build defaults configuration from command-line arguments
    let telemetry_level = match matches
        .get_one::<String>("telemetry-level")
        .unwrap()
        .as_str()
    {
        "none" => TelemetryLevel::None,
        "info" => TelemetryLevel::Info,
        "debug" => TelemetryLevel::Debug,
        "trace" => TelemetryLevel::Trace,
        _ => TelemetryLevel::None,
    };

    let telemetry_include_sql = matches.get_flag("telemetry-include-sql");
    let ensure_indexes = matches.get_flag("ensure-indexes");

    let defaults = DefaultsConfig {
        telemetry: DefaultsTelemetryConfig {
            level: telemetry_level,
            include_sql: telemetry_include_sql,
        },
        ensure_indexes: ensure_indexes,
    };

    println!("Loading queries from: {}", queries_dir);
    println!("Output directory: {}", output_dir);
    println!("Default telemetry level: {:?}", telemetry_level);
    println!("Default ensure indexes: {}", ensure_indexes);

    // Use the same method as build.rs
    AutoModel::generate(
        || Ok(database_url.to_string()),
        queries_dir,
        output_dir,
        defaults,
    )
    .await
    .map_err(|e| anyhow::anyhow!("Code generation failed: {}", e))?;

    println!("✓ Code generation complete!");

    Ok(())
}
