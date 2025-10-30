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
                )
                .arg(
                    Arg::new("analysis-only")
                        .long("analysis-only")
                        .help("Only run query performance analysis, don't generate code")
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
    let analysis_only = matches.get_flag("analysis-only");

    if analysis_only {
        println!("AutoModel Query Analyzer");
        println!("=======================");
    } else {
        println!("AutoModel Code Generator");
        println!("=======================");
    }
    println!("Database URL: {}", database_url);
    println!("YAML file: {}", yaml_path.display());
    if dry_run {
        println!("Mode: Dry run (no files will be written)");
    }
    if analysis_only {
        println!("Mode: Analysis only (no code generation)");
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

    // Run query performance analysis
    println!("Running query performance analysis...");
    match automodel.analyze_all_queries(database_url).await {
        Ok(analysis) => {
            println!("✓ Query analysis completed successfully");
            println!("Analysis Results:");
            println!("================");

            for result in &analysis.query_results {
                println!("\nQuery: {}", result.query_name);

                let status = if result.errors.iter().any(|e| e.contains("Analysis skipped")) {
                    "⏭️  SKIPPED BY CONFIGURATION"
                } else if !result.errors.is_empty() {
                    "❌ ANALYSIS FAILED"
                } else if result.has_sequential_scan {
                    "⚠️  SEQUENTIAL SCAN DETECTED"
                } else {
                    "✓ No sequential scans"
                };

                println!("  Status: {}", status);

                if !result.warnings.is_empty() {
                    println!("  Warnings:");
                    for warning in &result.warnings {
                        println!("    - {}", warning);
                    }
                }

                if !result.errors.is_empty() {
                    println!("  Errors:");
                    for error in &result.errors {
                        println!("    - {}", error);
                    }
                }
            }

            let total_queries = analysis.query_results.len();
            let skipped_queries = analysis
                .query_results
                .iter()
                .filter(|r| r.errors.iter().any(|e| e.contains("Analysis skipped")))
                .count();
            let failed_queries = analysis
                .query_results
                .iter()
                .filter(|r| {
                    !r.errors.is_empty() && !r.errors.iter().any(|e| e.contains("Analysis skipped"))
                })
                .count();
            let analyzed_queries = total_queries - skipped_queries - failed_queries;
            let queries_with_seq_scan = analysis
                .query_results
                .iter()
                .filter(|r| r.has_sequential_scan)
                .count();

            println!("\nSummary:");
            println!("========");
            println!("Total queries: {}", total_queries);
            println!("Successfully analyzed: {}", analyzed_queries);
            println!("Skipped by configuration: {}", skipped_queries);
            println!("Failed analysis: {}", failed_queries);
            println!("Queries with sequential scans: {}", queries_with_seq_scan);

            if queries_with_seq_scan > 0 {
                println!("⚠️  Consider adding indexes for queries with sequential scans");
            } else if analyzed_queries > 0 {
                println!("✓ All analyzed queries appear to be using indexes efficiently");
            }
        }
        Err(e) => {
            println!("❌ Query analysis failed: {}", e);
            println!("Note: Analysis requires a live database connection");
        }
    }
    println!();

    // If analysis-only mode, exit here
    if analysis_only {
        println!("Analysis complete. Skipping code generation.");
        return Ok(());
    }

    // Get modules for modular generation
    let modules = automodel.get_modules();

    // Determine output directory/file
    let output_path = if let Some(output) = matches.get_one::<String>("output") {
        PathBuf::from(output)
    } else {
        // Default to yaml filename with _generated suffix as directory
        "generated".into()
    };

    // If modules exist, generate modular structure like generate_at_build_time
    if !modules.is_empty() {
        println!(
            "Generating modular structure with {} modules: {}",
            modules.len(),
            modules.join(", ")
        );

        if !dry_run {
            // Create output directory
            tokio::fs::create_dir_all(&output_path).await?;
        }

        // Calculate hash of YAML file for proper file headers
        let yaml_hash = AutoModel::calculate_file_hash(yaml_file)?;

        // Generate code for queries without a module (main mod.rs content)
        println!("Generating main module code...");
        let main_module_code = automodel
            .generate_code_for_module(database_url, None)
            .await?;

        let mut mod_declarations = Vec::new();

        // Generate separate files for each named module
        for module in &modules {
            println!("Generating module: {}", module);
            let module_code = automodel
                .generate_code_for_module_with_hash(database_url, Some(module), Some(yaml_hash))
                .await?;

            if dry_run {
                println!("\n--- Module: {} ---", module);
                println!("{}", module_code);
            } else {
                let module_file = output_path.join(format!("{}.rs", module));
                tokio::fs::write(&module_file, &module_code).await?;
                println!("  ✓ Generated: {}", module_file.display());
            }

            mod_declarations.push(format!("pub mod {};", module));
        }

        // Create the main mod.rs file
        let mut mod_content = String::new();

        // Add hash comment at the top for consistency with build-time generation
        mod_content.push_str(&format!("// AUTOMODEL_HASH: {}\n", yaml_hash));
        mod_content.push_str(
            "// This file was automatically generated by AutoModel. Do not edit manually.\n\n",
        );

        // Add module declarations first
        if !mod_declarations.is_empty() {
            for declaration in mod_declarations {
                mod_content.push_str(&declaration);
                mod_content.push('\n');
            }
            mod_content.push('\n');
        }

        // Add the main module code (functions without a specific module)
        let trimmed_main_code = main_module_code.trim();
        if !trimmed_main_code.is_empty()
            && trimmed_main_code
                .lines()
                .any(|line| !line.starts_with("//") && !line.trim().is_empty())
        {
            mod_content.push_str(&main_module_code);
        }

        if dry_run {
            println!("\n--- mod.rs ---");
            println!("{}", mod_content);
        } else {
            let mod_file = output_path.join("mod.rs");
            tokio::fs::write(&mod_file, &mod_content).await?;
            println!("  ✓ Generated: {}", mod_file.display());
        }

        if !dry_run {
            println!("\n✓ Modular code generation complete!");
            println!("Generated in directory: {}", output_path.display());
            println!("Include in your Rust project with:");
            println!(
                "  mod {};",
                output_path.file_name().unwrap().to_str().unwrap()
            );
        }
    } else {
        // No modules, generate single file like before
        println!("No modules defined, generating single file...");

        // Generate Rust code
        println!("Connecting to database and generating code...");
        let code = automodel.generate_code(database_url).await?;

        println!("✓ Successfully generated Rust code");

        if dry_run {
            println!("\nGenerated code (dry run):");
            println!("{}", code);
            return Ok(());
        }

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
