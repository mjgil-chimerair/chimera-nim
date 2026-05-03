//! Chimera-Nim Unified Build Orchestration
//!
//! This xtask provides a single entrypoint for building all language components,
//! running tests, and executing conformance suites.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process::Command as ProcessCommand;

#[derive(Parser, Debug)]
#[command(name = "xtask")]
#[command(about = "Chimera-Nim unified build orchestration")]
struct Args {
    #[command(subcommand)]
    command: XtaskCommand,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Subcommand, Debug)]
enum XtaskCommand {
    /// Build all language components
    Build {
        /// Build only the specified language (rust, ocaml, zig, cpp, or all)
        #[arg(short, long, default_value = "all")]
        lang: String,
    },
    /// Run all tests
    Test {
        /// Run only tests for the specified language
        #[arg(short, long, default_value = "all")]
        lang: String,
    },
    /// Run conformance suites
    Conformance {
        /// Specific suite to run (lexing, parsing, sema, macro, mir, runtime, or all)
        #[arg(short, long, default_value = "all")]
        suite: String,
    },
    /// Check independent configurability of all language roots
    CheckIndependent {
        /// Language to check (rust, ocaml, zig, cpp, or all)
        #[arg(short, long, default_value = "all")]
        lang: String,
    },
}

fn build_rust(verbose: bool) -> Result<()> {
    if verbose {
        println!("[xtask] Building Rust workspace...");
    }

    let mut cmd = ProcessCommand::new("cargo");
    cmd.arg("build").arg("--workspace");

    if !verbose {
        cmd.arg("-q");
    }

    let status = cmd.status().context("Rust build failed")?;
    if !status.success() {
        anyhow::bail!("Rust build failed with exit code: {}", status);
    }

    println!("[xtask] Rust build complete");
    Ok(())
}

fn build_ocaml(verbose: bool) -> Result<()> {
    if verbose {
        println!("[xtask] Building OCaml labs...");
    }

    let mut cmd = ProcessCommand::new("make");
    cmd.current_dir("ocaml");

    if !verbose {
        cmd.arg("-s");
    }

    let status = cmd.status().context("OCaml build failed")?;
    if !status.success() {
        anyhow::bail!("OCaml build failed with exit code: {}", status);
    }

    println!("[xtask] OCaml build complete");
    Ok(())
}

fn build_zig(verbose: bool) -> Result<()> {
    if verbose {
        println!("[xtask] Building Zig helpers...");
    }

    let mut cmd = ProcessCommand::new("zig");
    cmd.arg("build");

    if !verbose {
        cmd.arg("-q");
    }

    let status = cmd.status().context("Zig build failed")?;
    if !status.success() {
        anyhow::bail!("Zig build failed with exit code: {}", status);
    }

    println!("[xtask] Zig build complete");
    Ok(())
}

fn build_cpp(verbose: bool) -> Result<()> {
    if verbose {
        println!("[xtask] Building C++ bridge...");
    }

    let build_dir = PathBuf::from("cpp/build");
    if !build_dir.exists() {
        std::fs::create_dir_all(&build_dir).context("Failed to create cpp/build directory")?;
    }

    let mut cmake = ProcessCommand::new("cmake");
    cmake.arg("-S").arg("cpp");
    cmake.arg("-B").arg(&build_dir);

    let status = cmake.status().context("CMake configure failed")?;
    if !status.success() {
        anyhow::bail!("CMake configure failed with exit code: {}", status);
    }

    let mut make = ProcessCommand::new("make");
    make.arg("-C").arg(&build_dir);

    if !verbose {
        make.arg("-s");
    }

    let status = make.status().context("C++ build failed")?;
    if !status.success() {
        anyhow::bail!("C++ build failed with exit code: {}", status);
    }

    println!("[xtask] C++ build complete");
    Ok(())
}

fn test_rust(verbose: bool) -> Result<()> {
    if verbose {
        println!("[xtask] Running Rust tests...");
    }

    let mut cmd = ProcessCommand::new("cargo");
    cmd.arg("test").arg("--workspace");

    if !verbose {
        cmd.arg("-q");
    }

    let status = cmd.status().context("Rust tests failed")?;
    if !status.success() {
        anyhow::bail!("Rust tests failed with exit code: {}", status);
    }

    println!("[xtask] Rust tests complete");
    Ok(())
}

fn test_ocaml(verbose: bool) -> Result<()> {
    if verbose {
        println!("[xtask] Running OCaml tests...");
    }

    let mut cmd = ProcessCommand::new("make");
    cmd.arg("test").current_dir("ocaml");

    if !verbose {
        cmd.arg("-s");
    }

    let status = cmd.status().context("OCaml tests failed")?;
    if !status.success() {
        anyhow::bail!("OCaml tests failed with exit code: {}", status);
    }

    println!("[xtask] OCaml tests complete");
    Ok(())
}

fn check_rust_independent() -> Result<()> {
    let mut cmd = ProcessCommand::new("cargo");
    cmd.arg("build").arg("--workspace");
    let status = cmd.status().context("Rust independent check failed")?;
    if !status.success() {
        anyhow::bail!("Rust workspace failed to build independently");
    }
    Ok(())
}

fn check_ocaml_independent() -> Result<()> {
    let mut cmd = ProcessCommand::new("dune");
    cmd.arg("build").arg("--root").arg("ocaml");
    let status = cmd.status().context("OCaml independent check failed")?;
    if !status.success() {
        anyhow::bail!("OCaml workspace failed to build independently");
    }
    Ok(())
}

fn check_zig_independent() -> Result<()> {
    let mut cmd = ProcessCommand::new("zig");
    cmd.arg("build").arg("--root").arg("zig");
    let status = cmd.status().context("Zig independent check failed")?;
    if !status.success() {
        anyhow::bail!("Zig workspace failed to build independently");
    }
    Ok(())
}

fn check_cpp_independent() -> Result<()> {
    let build_dir = PathBuf::from("cpp/build");
    if !build_dir.exists() {
        std::fs::create_dir_all(&build_dir).context("Failed to create cpp/build directory")?;
    }

    let mut cmake = ProcessCommand::new("cmake");
    cmake.arg("-S").arg("cpp").arg("-B").arg(&build_dir);
    let status = cmake.status().context("CMake configure failed")?;
    if !status.success() {
        anyhow::bail!("C++ workspace failed to configure independently");
    }

    let mut make = ProcessCommand::new("make");
    make.arg("-C").arg(&build_dir);
    let status = make.status().context("C++ build failed")?;
    if !status.success() {
        anyhow::bail!("C++ workspace failed to build independently");
    }
    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();

    match args.command {
        XtaskCommand::Build { lang } => {
            if lang == "all" || lang == "rust" {
                build_rust(args.verbose)?;
            }
            if lang == "all" || lang == "ocaml" {
                build_ocaml(args.verbose)?;
            }
            if lang == "all" || lang == "zig" {
                build_zig(args.verbose)?;
            }
            if lang == "all" || lang == "cpp" {
                build_cpp(args.verbose)?;
            }
        }
        XtaskCommand::Test { lang } => {
            if lang == "all" || lang == "rust" {
                test_rust(args.verbose)?;
            }
            if lang == "all" || lang == "ocaml" {
                test_ocaml(args.verbose)?;
            }
            if lang == "all" || lang == "zig" || lang == "cpp" {
                println!("[xtask] Warning: Zig and C++ use Rust test infrastructure");
            }
        }
        XtaskCommand::Conformance { suite: _ } => {
            println!("[xtask] Conformance suites require fixture corpus (Task 7)");
        }
        XtaskCommand::CheckIndependent { lang } => {
            if lang == "all" || lang == "rust" {
                check_rust_independent()?;
                println!("[xtask] Rust: OK");
            }
            if lang == "all" || lang == "ocaml" {
                check_ocaml_independent()?;
                println!("[xtask] OCaml: OK");
            }
            if lang == "all" || lang == "zig" {
                check_zig_independent()?;
                println!("[xtask] Zig: OK");
            }
            if lang == "all" || lang == "cpp" {
                check_cpp_independent()?;
                println!("[xtask] C++: OK");
            }
        }
    }

    Ok(())
}
