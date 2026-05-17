#[cfg(test)]
use rnim_allocator as _;

use anyhow::{Context, Result};
use clap::{ArgAction, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about = "Rust Nim compiler", long_about = None)]
struct Cli {
    /// Enable verbose output
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Use ANSI colors (default: auto-detect)
    #[arg(long, global = true, value_name = "when")]
    color: Option<String>,

    /// Define a compile-time symbol
    #[arg(short = 'd', long, action = ArgAction::Append, global = true, value_name = "symbol[=value]")]
    define: Vec<String>,

    /// Set the backend (c, cpp, js, vm)
    #[arg(long, global = true, value_name = "backend")]
    backend: Option<String>,

    /// Enable threads
    #[arg(long, global = true)]
    threads: bool,

    /// Stack size for the VM backend (in KB)
    #[arg(long, global = true, value_name = "size")]
    stack: Option<String>,

    /// Style of exception handling (cpp, asm, python, at, setjmp, none)
    #[arg(long, global = true, value_name = "style")]
    exceptions: Option<String>,

    /// Additional import paths
    #[arg(short = 'p', long, action = ArgAction::Append, global = true, value_name = "path")]
    path: Vec<String>,

    /// Compile with specified Nim cache directory
    #[arg(long, global = true, value_name = "path")]
    nimcache: Option<String>,

    /// List of modules to compile
    files: Vec<PathBuf>,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Compile the given file to C
    C {
        /// Input file
        file: Option<PathBuf>,
    },
    /// Compile the given file (alias for c)
    Compile {
        /// Input file
        file: Option<PathBuf>,
    },
    /// Compile and run the given file
    R {
        /// Input file
        file: Option<PathBuf>,
    },
    /// Compile and run (alias for r)
    Run {
        /// Input file
        file: Option<PathBuf>,
    },
    /// Compile to JavaScript
    Js {
        /// Input file
        file: Option<PathBuf>,
    },
    /// Check/validate the given file without compiling
    Check {
        /// Input file
        file: Option<PathBuf>,
    },
    /// Generate documentation
    Doc {
        /// Input file
        file: Option<PathBuf>,
    },
    /// Format/pretty-print the given file
    Pretty {
        /// Input file
        file: Option<PathBuf>,
    },
    /// Dump the compiler's internal state
    Dump {
        /// Input file
        file: Option<PathBuf>,
    },
    /// Run nimsuggest for IDE integration
    Suggest {
        /// Input file
        file: Option<PathBuf>,
    },
}

impl Cli {
    /// Get the list of files to compile
    fn files_to_compile(&self) -> Vec<PathBuf> {
        let mut files = self.files.clone();
        if let Some(ref cmd) = self.command {
            if let Some(file) = cmd.file() {
                files.push(file);
            }
        }
        files
    }

    /// Get the backend to use
    fn backend(&self) -> &str {
        self.backend.as_deref().unwrap_or("c")
    }
}

trait CommandExt {
    fn file(&self) -> Option<PathBuf>;
}

impl CommandExt for Command {
    fn file(&self) -> Option<PathBuf> {
        match self {
            Command::C { file } => file.clone(),
            Command::Compile { file } => file.clone(),
            Command::R { file } => file.clone(),
            Command::Run { file } => file.clone(),
            Command::Js { file } => file.clone(),
            Command::Check { file } => file.clone(),
            Command::Doc { file } => file.clone(),
            Command::Pretty { file } => file.clone(),
            Command::Dump { file } => file.clone(),
            Command::Suggest { file } => file.clone(),
        }
    }
}

fn run_compile(files: &[PathBuf], backend: &str, defines: &[String]) -> Result<()> {
    if files.is_empty() {
        anyhow::bail!("No input files specified");
    }

    match backend {
        "c" | "cpp" => {
            for file in files {
                let path = camino::Utf8Path::from_path(file)
                    .ok_or_else(|| anyhow::anyhow!("Invalid path"))?;

                println!("Compiling {} to C...", file.display());

                match rnim_codegen_c::emit_c(path) {
                    Ok(module) => {
                        println!("  Generated: {} lines of C code", module.source.len());
                        println!("  Header: {} lines", module.header.len());
                    }
                    Err(e) => {
                        eprintln!("  Error: {}", e);
                    }
                }

                for define in defines {
                    println!("  define: {}", define);
                }
            }
        }
        _ => {
            for file in files {
                println!("Compiling {} (backend: {})", file.display(), backend);
                for define in defines {
                    println!("  define: {}", define);
                }
            }
        }
    }

    Ok(())
}

fn run_check(files: &[PathBuf]) -> Result<()> {
    if files.is_empty() {
        anyhow::bail!("No input files specified");
    }

    let _db = rnim_sema::SemanticDb::new();

    for file in files {
        println!("Checking {}", file.display());

        match std::fs::read_to_string(file) {
            Ok(content) => {
                // Use the check module for content validation
                let result = rnim_sema::check::check_content(&content, &file.to_string_lossy());
                println!("  Status: {:?}", result.status);
                println!("  Errors: {}", result.error_count);
                println!("  Warnings: {}", result.warning_count);
            }
            Err(e) => {
                eprintln!("  Error reading file: {}", e);
            }
        }
    }

    Ok(())
}

fn run_doc(files: &[PathBuf]) -> Result<()> {
    use rnim_docgen::{DocBuilder, DocConfig, DocFormat, DocSymbol, SymbolKind};
    use rnim_span::{FileId, Span};

    if files.is_empty() {
        anyhow::bail!("No input files specified");
    }

    for file in files {
        println!("Generating docs for {}", file.display());

        let content = std::fs::read_to_string(file)?;
        let file_name = file
            .file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "module".to_string());

        let config = DocConfig {
            format: DocFormat::Html,
            ..Default::default()
        };
        let mut builder = DocBuilder::new(config);
        builder.set_module(&file_name, FileId(0));

        let symbol = DocSymbol {
            name: file_name.clone(),
            kind: SymbolKind::Module,
            span: Span::new(FileId(0), 0, 0),
            brief: None,
            description: Some(content.clone()),
            parameters: Vec::new(),
            returns: None,
            examples: Vec::new(),
            see_also: Vec::new(),
        };

        builder.add_module_doc(&content);
        builder.add_symbol(symbol);

        match builder.build() {
            Ok(html) => {
                let out_file = file.with_extension("html");
                std::fs::write(&out_file, &html)?;
                println!("  Wrote: {}", out_file.display());
            }
            Err(e) => {
                eprintln!("  Error generating docs: {}", e);
            }
        }
    }

    Ok(())
}

fn run_pretty(files: &[PathBuf]) -> Result<()> {
    use rnim_lexer::Lexer;
    use rnim_parser::Parser;
    use rnim_span::FileId;

    if files.is_empty() {
        anyhow::bail!("No input files specified");
    }

    for file in files {
        println!("Pretty-printing {}", file.display());

        let content = std::fs::read_to_string(file)?;
        let mut lexer = Lexer::new(&content, FileId(0));
        let mut token_count = 0;
        while lexer.next_token().is_some() {
            token_count += 1;
        }

        let mut parser = Parser::new(&content, FileId(0));
        let cst = parser.parse_cst();
        println!("  Parsed successfully, {} tokens", token_count);
        println!("  CST node kind: {:?}", cst.kind());
        println!("  Note: Pretty-print output not yet fully implemented");
    }

    Ok(())
}

fn run_suggest(file: &PathBuf) -> Result<()> {
    use rnim_suggest::SuggestServer;

    println!("Running nimsuggest for {}", file.display());

    let content = std::fs::read_to_string(file)?;
    let mut server = SuggestServer::new();

    let query = format!("suggest {}", file.to_string_lossy());
    let result = server.handle_query(&query);
    if result.is_empty() {
        println!("  No suggestions available");
    } else {
        println!("  Suggestions: {}", result);
    }

    println!("  File: {} ({} bytes)", file.display(), content.len());
    Ok(())
}

fn run_js(files: &[PathBuf]) -> Result<()> {
    if files.is_empty() {
        anyhow::bail!("No input files specified");
    }

    for file in files {
        let path =
            camino::Utf8Path::from_path(file).ok_or_else(|| anyhow::anyhow!("Invalid path"))?;

        println!("Compiling {} to JavaScript...", file.display());

        match rnim_codegen_js::emit_js_api(path) {
            Ok(module) => {
                println!("  Generated: {} lines of JS", module.source.len());
            }
            Err(e) => {
                eprintln!("  Error: {}", e);
            }
        }
    }

    Ok(())
}

fn run_run(files: &[PathBuf], backend: &str) -> Result<()> {
    if files.is_empty() {
        anyhow::bail!("No input files specified");
    }

    for file in files {
        println!(
            "Compiling and running {} (backend: {})",
            file.display(),
            backend
        );

        let path =
            camino::Utf8Path::from_path(file).ok_or_else(|| anyhow::anyhow!("Invalid path"))?;

        match backend {
            "c" | "cpp" => match rnim_codegen_c::emit_c(path) {
                Ok(module) => {
                    println!("  Compiled {} lines of C code", module.source.len());
                    println!("  Note: Full compilation+run requires external C compiler");
                }
                Err(e) => {
                    eprintln!("  Compilation error: {}", e);
                }
            },
            "js" => match rnim_codegen_js::emit_js_api(path) {
                Ok(module) => {
                    println!("  Compiled {} lines of JS", module.source.len());
                    println!("  Note: Full run requires Node.js runtime");
                }
                Err(e) => {
                    eprintln!("  Compilation error: {}", e);
                }
            },
            _ => {
                println!("  Backend '{}' not yet supported for run", backend);
            }
        }
    }

    Ok(())
}

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();
    let files = cli.files_to_compile();
    let backend = cli.backend();

    if cli.verbose {
        eprintln!("chimera verbose: backend={}, files={:?}", backend, files);
    }

    match cli.command {
        Some(Command::C { .. }) | Some(Command::Compile { .. }) => {
            run_compile(&files, backend, &cli.define).context("compile failed")
        }
        Some(Command::R { .. }) | Some(Command::Run { .. }) => {
            run_run(&files, backend).context("run failed")
        }
        Some(Command::Js { .. }) => run_js(&files).context("JS compilation failed"),
        Some(Command::Check { .. }) => run_check(&files).context("check failed"),
        Some(Command::Doc { .. }) => run_doc(&files).context("doc generation failed"),
        Some(Command::Pretty { .. }) => run_pretty(&files).context("pretty-print failed"),
        Some(Command::Dump { file }) => {
            if let Some(ref f) = file {
                println!("Dumping state for {}", f.display());
            } else {
                println!("Dumping global compiler state");
            }
            Ok(())
        }
        Some(Command::Suggest { file }) => {
            let f = file.context("suggest requires a file")?;
            run_suggest(&f).context("nimsuggest failed")
        }
        None => {
            println!("chimera - Rust implementation of the Nim compiler");
            println!("\nUsage: chimera <command> [options] [files...]");
            println!("\nCommands:");
            println!("  c, compile    Compile to C/C++");
            println!("  r, run         Compile and run");
            println!("  js             Compile to JavaScript");
            println!("  check          Check/validate without compiling");
            println!("  doc            Generate documentation");
            println!("  pretty         Format/pretty-print");
            println!("  suggest        Run nimsuggest for IDE integration");
            println!("  dump           Dump compiler internal state");
            println!("\nUse 'chimera <command> --help' for command-specific options.");
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_no_args() {
        let cli = Cli::try_parse_from(["chimera"]).unwrap();
        assert!(cli.command.is_none());
        assert!(cli.files.is_empty());
        assert!(!cli.verbose);
    }

    #[test]
    fn test_cli_verbose_flag() {
        let cli = Cli::try_parse_from(["chimera", "-v", "check"]).unwrap();
        assert!(cli.verbose);
    }

    #[test]
    fn test_cli_check_with_file() {
        let cli = Cli::try_parse_from(["chimera", "check", "test.nim"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Check { file: Some(_) })
        ));
        let files = cli.files_to_compile();
        assert!(files.iter().any(|f| f.to_string_lossy() == "test.nim"));
    }

    #[test]
    fn test_cli_define_flag() {
        let cli = Cli::try_parse_from(["chimera", "-d", "release", "-d", "debug=true", "test.nim"])
            .unwrap();
        assert_eq!(cli.define, vec!["release", "debug=true"]);
    }

    #[test]
    fn test_cli_backend_flag() {
        let cli = Cli::try_parse_from(["chimera", "--backend", "js", "test.nim"]).unwrap();
        assert_eq!(cli.backend.as_deref(), Some("js"));
    }

    #[test]
    fn test_cli_multiple_files() {
        let cli = Cli::try_parse_from(["chimera", "file1.nim", "file2.nim", "file3.nim"]).unwrap();
        assert_eq!(cli.files.len(), 3);
    }

    #[test]
    fn test_cli_files_to_compile_with_command() {
        let cli = Cli::try_parse_from(["chimera", "check", "file1.nim"]).unwrap();
        let files = cli.files_to_compile();
        // When check command has file, it becomes the command's file, not a positional
        assert_eq!(files.len(), 1);
    }

    #[test]
    fn test_cli_files_to_compile_without_command() {
        let cli = Cli::try_parse_from(["chimera", "file1.nim", "file2.nim"]).unwrap();
        let files = cli.files_to_compile();
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn test_cli_default_backend() {
        let cli = Cli::try_parse_from(["chimera"]).unwrap();
        assert_eq!(cli.backend(), "c");
    }

    #[test]
    fn test_cli_threads_flag() {
        let cli = Cli::try_parse_from(["chimera", "--threads", "compile"]).unwrap();
        assert!(cli.threads);
    }

    #[test]
    fn test_cli_path_flag() {
        let cli = Cli::try_parse_from([
            "chimera",
            "-p",
            "/path/to/lib",
            "-p",
            "/other/path",
            "test.nim",
        ])
        .unwrap();
        assert_eq!(cli.path.len(), 2);
    }

    #[test]
    fn test_cli_command_c_alias() {
        let cli = Cli::try_parse_from(["chimera", "c", "test.nim"]).unwrap();
        assert!(matches!(cli.command, Some(Command::C { .. })));
    }

    #[test]
    fn test_cli_command_run_alias() {
        let cli = Cli::try_parse_from(["chimera", "r", "test.nim"]).unwrap();
        assert!(matches!(cli.command, Some(Command::R { .. })));
    }

    #[test]
    fn test_cli_command_js() {
        let cli = Cli::try_parse_from(["chimera", "js", "test.nim"]).unwrap();
        assert!(matches!(cli.command, Some(Command::Js { .. })));
    }

    #[test]
    fn test_cli_command_doc() {
        let cli = Cli::try_parse_from(["chimera", "doc", "test.nim"]).unwrap();
        assert!(matches!(cli.command, Some(Command::Doc { .. })));
    }

    #[test]
    fn test_cli_command_check() {
        let cli = Cli::try_parse_from(["chimera", "check", "test.nim"]).unwrap();
        assert!(matches!(cli.command, Some(Command::Check { .. })));
    }

    #[test]
    fn test_cli_command_pretty() {
        let cli = Cli::try_parse_from(["chimera", "pretty", "test.nim"]).unwrap();
        assert!(matches!(cli.command, Some(Command::Pretty { .. })));
    }

    #[test]
    fn test_cli_command_suggest() {
        let cli = Cli::try_parse_from(["chimera", "suggest", "test.nim"]).unwrap();
        assert!(matches!(cli.command, Some(Command::Suggest { .. })));
    }

    #[test]
    fn test_cli_command_dump() {
        let cli = Cli::try_parse_from(["chimera", "dump"]).unwrap();
        assert!(matches!(cli.command, Some(Command::Dump { .. })));
    }

    #[test]
    fn test_cli_nimcache_flag() {
        let cli =
            Cli::try_parse_from(["chimera", "--nimcache", "/tmp/nimcache", "test.nim"]).unwrap();
        assert_eq!(cli.nimcache.as_deref(), Some("/tmp/nimcache"));
    }

    #[test]
    fn test_cli_stack_flag() {
        let cli = Cli::try_parse_from(["chimera", "--stack", "8192", "run", "test.nim"]).unwrap();
        assert_eq!(cli.stack.as_deref(), Some("8192"));
    }

    #[test]
    fn test_cli_exceptions_flag() {
        let cli =
            Cli::try_parse_from(["chimera", "--exceptions", "cpp", "compile", "test.nim"]).unwrap();
        assert_eq!(cli.exceptions.as_deref(), Some("cpp"));
    }

    #[test]
    fn test_command_file_extractors() {
        let cmd = Command::Check {
            file: Some(PathBuf::from("test.nim")),
        };
        assert_eq!(cmd.file(), Some(PathBuf::from("test.nim")));

        let cmd = Command::Suggest { file: None };
        assert!(cmd.file().is_none());
    }
}
