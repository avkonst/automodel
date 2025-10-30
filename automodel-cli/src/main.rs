use anyhow::Result;
use automodel::*;
use clap::{Arg, ArgMatches, Command};
use std::path::PathBuf;

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
        .about("Generate typed Rust functions from YAML-defined SQL queries")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("generate")
                .about("Generate Rust code from YAML queries")
                .arg(
                    Arg::new("database-url")
                        .short('d')
                        .long("database-url")
                        .value_name("URL")
                        .help("PostgreSQL database connection URL")
                        .required(true),
                )
                .arg(
                    Arg::new("yaml-file")
                        .short('f')
                        .long("file")
                        .value_name("FILE")
                        .help("YAML file containing query definitions")
                        .required(true),
                )
                .arg(
                    Arg::new("output")
                        .short('o')
                        .long("output")
                        .value_name("FILE")
                        .help("Output directory for generated Rust code"),
                ),
        )
}

async fn generate_command(matches: &ArgMatches) -> Result<()> {
    let database_url = matches.get_one::<String>("database-url").unwrap();
    let yaml_file = matches.get_one::<String>("yaml-file").unwrap();

    let automodel = AutoModel::new(&yaml_file).await?;

    // Determine output directory/file
    let output_path = if let Some(output) = matches.get_one::<String>("output") {
        PathBuf::from(output)
    } else {
        // Default to generated directory
        "generated".into()
    };

    // Use the new generate_to_directory method to reuse logic
    automodel
        .generate_to_directory(database_url, output_path.to_str().unwrap())
        .await?;

    Ok(())
}
