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
            // Default to generate command for backward compatibility
            if let (Some(database_url), Some(yaml_file)) = (
                matches.get_one::<String>("database-url"),
                matches.get_one::<String>("yaml-file"),
            ) {
                generate_with_args(database_url, yaml_file, &matches).await?;
            } else {
                build_cli().print_help()?;
                std::process::exit(1);
            }
        }
    }

    Ok(())
}

fn build_cli() -> Command {
    Command::new("automodel")
        .version("0.1.0")
        .author("AutoModel Team")
        .about("Generate typed Rust functions from YAML-defined SQL queries")
        .subcommand_required(false)
        .arg_required_else_help(false)
        .arg(
            Arg::new("database-url")
                .short('d')
                .long("database-url")
                .value_name("URL")
                .help("PostgreSQL database connection URL")
                .required(false),
        )
        .arg(
            Arg::new("yaml-file")
                .short('f')
                .long("file")
                .value_name("FILE")
                .help("YAML file containing query definitions")
                .required(false),
        )
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
                        .help("Output file for generated Rust code (defaults to <yaml_file>.rs)"),
                )
                .arg(
                    Arg::new("module-name")
                        .short('m')
                        .long("module")
                        .value_name("NAME")
                        .help("Module name for generated code"),
                )
                .arg(
                    Arg::new("dry-run")
                        .long("dry-run")
                        .help("Generate code but don't write to file")
                        .action(clap::ArgAction::SetTrue),
                ),
        )
}

async fn generate_command(matches: &ArgMatches) -> Result<()> {
    let database_url = matches.get_one::<String>("database-url").unwrap();
    let yaml_file = matches.get_one::<String>("yaml-file").unwrap();

    generate_with_args(database_url, yaml_file, matches).await
}

async fn generate_with_args(
    database_url: &str,
    yaml_file: &str,
    matches: &ArgMatches,
) -> Result<()> {
    let yaml_path = PathBuf::from(yaml_file);

    if !yaml_path.exists() {
        anyhow::bail!("YAML file '{}' does not exist", yaml_path.display());
    }

    let dry_run = matches.get_flag("dry-run");

    println!("AutoModel Code Generator");
    println!("=======================");
    println!("Database URL: {}", database_url);
    println!("YAML file: {}", yaml_path.display());
    if dry_run {
        println!("Mode: Dry run (no files will be written)");
    }
    println!();

    // Create AutoModel instance and load queries from YAML file
    println!("Loading queries from YAML file...");
    let automodel = AutoModel::new(&yaml_path).await?;

    println!(
        "✓ Successfully loaded {} queries",
        automodel.queries().len()
    );
    for (i, query) in automodel.queries().iter().enumerate() {
        println!(
            "  {}. {}: {}",
            i + 1,
            query.name,
            query.description.as_deref().unwrap_or("No description")
        );
    }
    println!();

    // Generate Rust code
    println!("Connecting to database and generating code...");
    let code = automodel.generate_code(database_url).await?;

    println!("✓ Successfully generated Rust code");

    if dry_run {
        println!("\nGenerated code (dry run):");
        println!("{}", code);
        return Ok(());
    }

    // Determine output file name
    let output_file = if let Some(output) = matches.get_one::<String>("output") {
        PathBuf::from(output)
    } else {
        yaml_path.with_extension("rs")
    };

    // Write the generated code to a file
    tokio::fs::write(&output_file, &code).await?;

    println!("✓ Generated code written to: {}", output_file.display());
    println!();
    println!("You can now include this file in your Rust project:");
    println!(
        "  mod {};",
        output_file.file_stem().unwrap().to_str().unwrap()
    );

    Ok(())
}
