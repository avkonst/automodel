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
                        .help("Output file for generated Rust code (defaults to <yaml_file>.rs)"),
                )
                .arg(
                    Arg::new("module-name")
                        .short('m')
                        .long("module")
                        .value_name("NAME")
                        .help("Module name for generated code"),
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

    println!("AutoModel Code Generator");
    println!("=======================");
    println!("Database URL: {}", database_url);
    println!("YAML file: {}", yaml_path.display());
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

    // Get modules for modular generation
    let modules = automodel.get_modules();

    // Determine output directory/file
    let output_path = if let Some(output) = matches.get_one::<String>("output") {
        PathBuf::from(output)
    } else {
        // Default to generated directory
        "generated".into()
    };

    // Check if we should generate modular structure or single file
    if !modules.is_empty() {
        println!(
            "Generating modular structure with {} modules: {}",
            modules.len(),
            modules.join(", ")
        );

        // Use the new generate_to_directory method to reuse logic
        automodel
            .generate_to_directory(database_url, output_path.to_str().unwrap(), yaml_file)
            .await?;

        println!("✓ Modular code generation complete!");
        println!("Generated in directory: {}", output_path.display());
        println!("Include in your Rust project with:");
        println!(
            "  mod {};",
            output_path.file_name().unwrap().to_str().unwrap()
        );
    } else {
        // No modules, generate single file
        println!("No modules defined, generating single file...");

        println!("Connecting to database and generating code...");
        let code = automodel.generate_code(database_url).await?;

        println!("✓ Successfully generated Rust code");

        // Use output_path but as a file (add .rs extension if it's a directory name)
        let output_file = if output_path.extension().is_none() {
            output_path.with_extension("rs")
        } else {
            output_path
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
    }

    Ok(())
}
