use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use scene_builder_core::project::behavior_gen;
use scene_builder_core::project::package::Package;

#[derive(Parser)]
#[command(
    name = "scene_builder_cli",
    about = "CLI for SLSB",
    long_about = "Can convert and serialize traditional SLAL packs to SLSB."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Convert a SLAL JSON pack to an SLSB project
    Convert {
        #[arg(short = 'i', long = "in")]
        in_path: PathBuf,
        #[arg(short = 'd', long = "out")]
        out_dir: PathBuf,
    },
    /// Build/compile an SLSB project
    Build {
        #[arg(short = 'i', long = "in")]
        in_path: PathBuf,
        #[arg(short = 'o', long = "out")]
        out_dir: PathBuf,
        /// Also write SLAnims/json SLAL registration (linear scenes only)
        #[arg(long)]
        slal: bool,
    },
    /// Export classic SLAL JSON from an SLSB project (refuses branching scenes)
    #[command(name = "export-slal")]
    ExportSlal {
        #[arg(short = 'i', long = "in")]
        in_path: PathBuf,
        #[arg(short = 'o', long = "out")]
        out_dir: PathBuf,
    },
    /// Generate FNIS_*_Behavior.hkx from FNIS AnimLists under a folder
    #[command(name = "generate-behaviors")]
    GenerateBehaviors {
        /// Root folder containing FNIS_*_List.txt files
        #[arg(short = 'i', long = "in")]
        in_path: PathBuf,
    },
}

fn convert(in_path: PathBuf, out_dir: PathBuf) -> Result<(), String> {
    if !in_path.exists() || !in_path.is_file() || in_path.extension().unwrap() != "json" {
        return Err("input slal file is invalid".to_string());
    }
    if !out_dir.exists() || !out_dir.is_dir() {
        return Err("output dir is invalid".to_string());
    }

    let mut out_path = out_dir;
    out_path.push(in_path.file_stem().unwrap());
    out_path.set_extension("slsb.json");
    println!(
        "Converting {} to {}",
        in_path.display(),
        out_path.display()
    );

    let mut project = Package::from_slal(in_path)?;
    project.write(out_path)
}

fn build(in_path: PathBuf, out_dir: PathBuf, with_slal: bool) -> Result<(), String> {
    if !in_path.exists() || !in_path.is_file() || in_path.extension().unwrap() != "json" {
        return Err("input project file is invalid".to_string());
    }
    if !out_dir.exists() || !out_dir.is_dir() {
        return Err("output dir is invalid".to_string());
    }

    let file = std::fs::File::open(&in_path).map_err(|e| e.to_string())?;
    let project = Package::from_file(file)?;
    project.build(out_dir.clone()).map_err(|e| e.to_string())?;
    if with_slal {
        project.write_slal_pack(&out_dir.join("SLAL"))?;
    }
    Ok(())
}

fn export_slal(in_path: PathBuf, out_dir: PathBuf) -> Result<(), String> {
    if !in_path.exists() || !in_path.is_file() || in_path.extension().unwrap() != "json" {
        return Err("input project file is invalid".to_string());
    }
    if !out_dir.exists() || !out_dir.is_dir() {
        return Err("output dir is invalid".to_string());
    }

    let file = std::fs::File::open(&in_path).map_err(|e| e.to_string())?;
    let project = Package::from_file(file)?;
    project.write_slal_pack(&out_dir)
}

fn generate_behaviors(in_path: PathBuf) -> Result<(), String> {
    if !in_path.exists() || !in_path.is_dir() {
        return Err("input dir is invalid".to_string());
    }
    let paths = behavior_gen::generate_behaviors_under(&in_path).map_err(|e| e.to_string())?;
    println!("Generated {} behavior file(s)", paths.len());
    for p in paths {
        println!("  {}", p.display());
    }
    Ok(())
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match cli.command {
        Commands::Convert { in_path, out_dir } => convert(in_path, out_dir),
        Commands::Build {
            in_path,
            out_dir,
            slal,
        } => build(in_path, out_dir, slal),
        Commands::ExportSlal { in_path, out_dir } => export_slal(in_path, out_dir),
        Commands::GenerateBehaviors { in_path } => generate_behaviors(in_path),
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("Error: {e}");
            ExitCode::FAILURE
        }
    }
}
