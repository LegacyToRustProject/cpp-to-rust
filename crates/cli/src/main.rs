use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};
use tracing::info;

#[derive(Parser)]
#[command(name = "cpp-to-rust")]
#[command(about = "AI-powered C/C++ to Rust conversion")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(long, default_value = "info")]
    log_level: String,
}

#[derive(Subcommand)]
enum Commands {
    /// Analyze a C/C++ project and output a structure report
    Analyze {
        /// Path to the C/C++ project directory
        path: PathBuf,
    },
    /// Convert a C/C++ project to Rust
    Convert {
        /// Path to the C/C++ project directory
        path: PathBuf,

        /// Output directory for generated Rust code
        #[arg(short, long, default_value = "output")]
        output: PathBuf,

        /// Conversion profile (c99, c11, cpp11, cpp17, embedded, generic)
        #[arg(long, default_value = "generic")]
        profile: String,

        /// Run verification after conversion (cargo check + output comparison)
        #[arg(long)]
        verify: bool,

        /// LLM provider to use
        #[arg(long, default_value = "claude")]
        llm: String,

        /// Model name
        #[arg(long, default_value = "claude-sonnet-4-20250514")]
        model: String,

        /// Maximum fix loop iterations
        #[arg(long, default_value = "10")]
        max_fix_iterations: usize,
    },
    /// Convert a single C/C++ file to Rust
    ConvertFile {
        /// Path to the C/C++ source file
        path: PathBuf,

        /// Conversion profile
        #[arg(long, default_value = "generic")]
        profile: String,

        /// LLM provider to use
        #[arg(long, default_value = "claude")]
        llm: String,

        /// Model name
        #[arg(long, default_value = "claude-sonnet-4-20250514")]
        model: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(&cli.log_level));
    tracing_subscriber::fmt().with_env_filter(filter).init();

    match cli.command {
        Commands::Analyze { path } => cmd_analyze(&path)?,
        Commands::Convert {
            path,
            output,
            profile,
            verify,
            llm,
            model,
            max_fix_iterations,
        } => {
            cmd_convert(
                &path,
                &output,
                &profile,
                verify,
                &llm,
                &model,
                max_fix_iterations,
            )
            .await?
        }
        Commands::ConvertFile {
            path,
            profile,
            llm,
            model,
        } => cmd_convert_file(&path, &profile, &llm, &model).await?,
    }

    Ok(())
}

fn cmd_analyze(path: &Path) -> Result<()> {
    info!("Analyzing project: {}", path.display());

    let project = cpp_parser::analyzer::analyze_project(path)?;
    let report = cpp_parser::analyzer::generate_report(&project);

    println!("{}", report);
    Ok(())
}

async fn cmd_convert(
    path: &Path,
    output: &Path,
    profile_name: &str,
    verify: bool,
    llm_name: &str,
    model: &str,
    max_fix_iterations: usize,
) -> Result<()> {
    info!("Converting project: {}", path.display());

    let project = cpp_parser::analyzer::analyze_project(path)?;
    let profile = load_profile(profile_name)?;
    let llm = create_llm_provider(llm_name, model)?;

    let generator = rust_generator::generator::Generator::new(llm, profile);
    let result = generator.convert_project(&project, output).await?;

    println!("Conversion complete:");
    println!("  Files converted: {}", result.files.len());
    println!("  Errors: {}", result.errors.len());

    for file in &result.files {
        println!(
            "  {} -> {}",
            file.original_path.display(),
            file.rust_path.display()
        );
    }

    for error in &result.errors {
        eprintln!("  ERROR: {}", error);
    }

    if verify && !result.files.is_empty() {
        println!("\nRunning verification...");
        let verify_llm = create_llm_provider(llm_name, model)?;
        let fix_loop = verifier::fix_loop::FixLoop::new(verify_llm, max_fix_iterations);

        for file in &result.files {
            println!("  Checking: {}", file.rust_path.display());
            let fix_result = fix_loop
                .fix_compile_errors(file.rust_code.clone(), output, &file.rust_path)
                .await?;

            if fix_result.success {
                println!("    OK (fixed in {} iterations)", fix_result.iterations);
            } else {
                println!("    FAILED after {} iterations:", fix_result.iterations);
                for err in &fix_result.final_errors {
                    println!("      {}", err);
                }
            }
        }
    }

    Ok(())
}

async fn cmd_convert_file(
    path: &Path,
    profile_name: &str,
    llm_name: &str,
    model: &str,
) -> Result<()> {
    info!("Converting file: {}", path.display());

    let source = std::fs::read_to_string(path)?;
    let file = cpp_parser::analyzer::analyze_file(path, &source);
    let profile = load_profile(profile_name)?;
    let llm = create_llm_provider(llm_name, model)?;

    let generator = rust_generator::generator::Generator::new(llm, profile);
    let output_dir = PathBuf::from(".");
    let result = generator.convert_file(&file, &output_dir).await?;

    println!("// Converted from: {}", path.display());
    println!("{}", result.rust_code);

    Ok(())
}

fn load_profile(name: &str) -> Result<rust_generator::prompt::ConversionProfile> {
    let profile_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.join("profiles").join(name))
        .unwrap_or_else(|| PathBuf::from("profiles").join(name));

    let mut profile = rust_generator::prompt::ConversionProfile {
        name: name.to_string(),
        ..Default::default()
    };

    // Try to load profile config
    let config_path = profile_dir.join("config.toml");
    if config_path.exists() {
        let config_str = std::fs::read_to_string(&config_path)?;
        let config: toml::Table = config_str.parse::<toml::Table>()?;

        if let Some(toml::Value::String(instructions)) = config.get("additional_instructions") {
            profile.additional_instructions = instructions.clone();
        }

        if let Some(toml::Value::Table(types)) = config.get("type_mappings") {
            for (k, v) in types {
                if let toml::Value::String(v_str) = v {
                    profile.type_mappings.insert(k.clone(), v_str.clone());
                }
            }
        }

        if let Some(toml::Value::Table(apis)) = config.get("api_mappings") {
            for (k, v) in apis {
                if let toml::Value::String(v_str) = v {
                    profile.api_mappings.insert(k.clone(), v_str.clone());
                }
            }
        }
    }

    Ok(profile)
}

fn create_llm_provider(
    name: &str,
    model: &str,
) -> Result<Box<dyn rust_generator::llm::LlmProvider>> {
    match name {
        "claude" => {
            let api_key = std::env::var("ANTHROPIC_API_KEY").map_err(|_| {
                anyhow::anyhow!(
                    "ANTHROPIC_API_KEY environment variable required for Claude provider"
                )
            })?;
            Ok(Box::new(rust_generator::llm::ClaudeProvider::new(
                api_key,
                model.to_string(),
            )))
        }
        "gemini" => {
            let api_key = std::env::var("GOOGLE_API_KEY").map_err(|_| {
                anyhow::anyhow!("GOOGLE_API_KEY environment variable required for Gemini provider")
            })?;
            let resolved_model = if model == "claude-sonnet-4-20250514" {
                "gemini-2.0-flash"
            } else {
                model
            };
            Ok(Box::new(rust_generator::llm::GeminiProvider::new(
                api_key,
                resolved_model.to_string(),
            )))
        }
        _ => anyhow::bail!("Unknown LLM provider: {}. Supported: claude, gemini", name),
    }
}
