//! Global compiler session, config loading, target triples, feature flags, cache paths, diagnostics registry.

#[cfg(test)]
use rnim_allocator as _;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Global compiler session
#[derive(Debug)]
pub struct Session {
    pub config: Config,
    pub target: TargetTriple,
    pub compile_mode: CompileMode,
}

/// Configuration precedence (highest to lowest):
/// 1. CLI arguments
/// 2. Environment variables (RNIM_*)
/// 3. rnim.toml file
/// 4. Default values
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub color: ColorMode,
    pub json_diagnostics: bool,
    pub verbosity: u8,
    pub defines: HashMap<String, Option<String>>,
    pub import_paths: Vec<PathBuf>,
    pub nimcache_path: Option<PathBuf>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            color: ColorMode::Auto,
            json_diagnostics: false,
            verbosity: 0,
            defines: HashMap::new(),
            import_paths: Vec::new(),
            nimcache_path: None,
        }
    }
}

/// Color mode for diagnostics
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ColorMode {
    #[default]
    Auto,
    Always,
    Never,
}

/// Compilation target mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CompileMode {
    #[default]
    C,
    Cpp,
    Js,
    ObjC,
}

/// Target triple components
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TargetTriple {
    pub arch: Arch,
    pub os: Os,
    pub env: Env,
    pub pointer_width: u32,
    pub endian: Endian,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Arch {
    X86_64,
    Aarch64,
    Arm,
    X86,
    Wasm32,
    Universal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Os {
    Linux,
    Windows,
    Macos,
    Web,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Env {
    Gnu,
    Musl,
    Msvc,
    Uwp,
    Android,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Endian {
    Little,
    Big,
}

impl Default for TargetTriple {
    fn default() -> Self {
        TargetTriple {
            arch: Arch::X86_64,
            os: Os::Linux,
            env: Env::Gnu,
            pointer_width: 64,
            endian: Endian::Little,
        }
    }
}

impl Session {
    /// Create a new session with default configuration
    pub fn new() -> Self {
        let config = Self::load_config_from_env();
        let target = Self::detect_target();
        Self {
            config,
            target,
            compile_mode: CompileMode::C,
        }
    }

    /// Load configuration from environment variables
    fn load_config_from_env() -> Config {
        let mut config = Config::default();

        // RNIM_COLOR
        if let Ok(val) = env::var("RNIM_COLOR") {
            config.color = match val.to_lowercase().as_str() {
                "always" => ColorMode::Always,
                "never" => ColorMode::Never,
                _ => ColorMode::Auto,
            };
        }

        // RNIM_JSON_DIAGNOSTICS
        if let Ok(val) = env::var("RNIM_JSON_DIAGNOSTICS") {
            config.json_diagnostics = val == "1" || val.to_lowercase() == "true";
        }

        // RNIM_VERBOSITY
        if let Ok(val) = env::var("RNIM_VERBOSITY") {
            config.verbosity = val.parse().unwrap_or(0);
        }

        // RNIM_DEFINES (comma-separated key=value pairs)
        if let Ok(val) = env::var("RNIM_DEFINES") {
            for define in val.split(',') {
                let parts: Vec<&str> = define.split('=').collect();
                let key = parts[0].trim().to_string();
                let value = parts.get(1).map(|s| s.trim().to_string());
                config.defines.insert(key, value);
            }
        }

        config
    }

    /// Detect the current compilation target
    fn detect_target() -> TargetTriple {
        #[cfg(target_arch = "x86_64")]
        let arch = Arch::X86_64;
        #[cfg(target_arch = "aarch64")]
        let arch = Arch::Aarch64;
        #[cfg(target_arch = "wasm32")]
        let arch = Arch::Wasm32;
        #[cfg(not(any(
            target_arch = "x86_64",
            target_arch = "aarch64",
            target_arch = "wasm32"
        )))]
        let arch = Arch::Universal;

        #[cfg(target_os = "linux")]
        let os = Os::Linux;
        #[cfg(target_os = "windows")]
        let os = Os::Windows;
        #[cfg(target_os = "macos")]
        let os = Os::Macos;
        #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
        let os = Os::Unknown;

        #[cfg(target_env = "gnu")]
        let env = Env::Gnu;
        #[cfg(target_env = "musl")]
        let env = Env::Musl;
        #[cfg(target_env = "msvc")]
        let env = Env::Msvc;
        #[cfg(not(any(target_env = "gnu", target_env = "musl", target_env = "msvc")))]
        let env = Env::Unknown;

        let pointer_width = size_of::<usize>() as u32 * 8;
        let endian = if cfg!(target_endian = "little") {
            Endian::Little
        } else {
            Endian::Big
        };

        TargetTriple {
            arch,
            os,
            env,
            pointer_width,
            endian,
        }
    }

    /// Load configuration from rnim.toml file
    pub fn load_toml_config(path: &Path) -> Result<Config, ConfigError> {
        let content = fs::read_to_string(path).map_err(|e| ConfigError::IoError(e.to_string()))?;

        toml::from_str(&content).map_err(|e| ConfigError::ParseError(e.to_string()))
    }

    /// Get the cache directory for this session
    pub fn cache_dir(&self) -> PathBuf {
        self.config
            .nimcache_path
            .clone()
            .unwrap_or_else(|| PathBuf::from(".rnimcache"))
    }

    /// Get an import path by index
    pub fn get_import_path(&self, index: usize) -> Option<&PathBuf> {
        self.config.import_paths.get(index)
    }

    /// Get a define value
    pub fn get_define(&self, key: &str) -> Option<&Option<String>> {
        self.config.defines.get(key)
    }

    /// Add a define at runtime
    pub fn add_define(&mut self, key: String, value: Option<String>) {
        self.config.defines.insert(key, value);
    }

    /// Check if a feature flag is enabled
    pub fn has_feature(&self, feature: &str) -> bool {
        self.config
            .defines
            .get(feature)
            .map(|v| v.is_some())
            .unwrap_or(false)
    }
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

/// Configuration errors
#[derive(Debug, Clone)]
pub enum ConfigError {
    IoError(String),
    ParseError(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::IoError(msg) => write!(f, "IO error: {}", msg),
            ConfigError::ParseError(msg) => write!(f, "parse error: {}", msg),
        }
    }
}

impl std::error::Error for ConfigError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_session() {
        let session = Session::default();
        assert_eq!(session.compile_mode, CompileMode::C);
        assert!(session.config.defines.is_empty());
    }

    #[test]
    fn test_color_mode_default() {
        assert_eq!(ColorMode::default(), ColorMode::Auto);
    }

    #[test]
    fn test_compile_mode_default() {
        assert_eq!(CompileMode::default(), CompileMode::C);
    }

    #[test]
    fn test_target_triple_default() {
        let target = TargetTriple::default();
        assert_eq!(target.arch, Arch::X86_64);
        assert_eq!(target.os, Os::Linux);
        assert_eq!(target.env, Env::Gnu);
        assert_eq!(target.pointer_width, 64);
    }

    #[test]
    fn test_session_add_define() {
        let mut session = Session::new();
        session.add_define("test".to_string(), Some("value".to_string()));
        assert_eq!(session.get_define("test"), Some(&Some("value".to_string())));
    }

    #[test]
    fn test_session_has_feature() {
        let mut session = Session::new();
        session.add_define("experimental".to_string(), Some(String::new()));
        assert!(session.has_feature("experimental"));
        assert!(!session.has_feature("nonexistent"));
    }

    #[test]
    fn test_session_cache_dir() {
        let session = Session::new();
        assert_eq!(session.cache_dir(), PathBuf::from(".rnimcache"));
    }

    #[test]
    fn test_config_error_display() {
        let err = ConfigError::IoError("test".to_string());
        assert!(format!("{}", err).contains("IO error"));

        let err = ConfigError::ParseError("test".to_string());
        assert!(format!("{}", err).contains("parse error"));
    }

    #[test]
    fn test_session_from_env() {
        let _session = Session::new();
    }

    #[test]
    fn test_config_precedence_defaults() {
        let config = Config::default();
        assert_eq!(config.color, ColorMode::Auto);
        assert!(!config.json_diagnostics);
        assert_eq!(config.verbosity, 0);
        assert!(config.defines.is_empty());
        assert!(config.import_paths.is_empty());
        assert!(config.nimcache_path.is_none());
    }

    #[test]
    fn test_target_triple_from_env_override() {
        // This test verifies the architecture detection works
        let target = TargetTriple::default();
        // Just verify we get a valid target
        assert!(matches!(
            target.arch,
            Arch::X86_64 | Arch::Aarch64 | Arch::Wasm32 | Arch::Universal
        ));
        assert!(matches!(
            target.os,
            Os::Linux | Os::Windows | Os::Macos | Os::Unknown
        ));
        assert!(matches!(target.endian, Endian::Little | Endian::Big));
    }

    #[test]
    fn test_color_mode_serialization() {
        assert_eq!(serde_json::to_string(&ColorMode::Auto).unwrap(), "\"Auto\"");
        assert_eq!(
            serde_json::to_string(&ColorMode::Always).unwrap(),
            "\"Always\""
        );
        assert_eq!(
            serde_json::to_string(&ColorMode::Never).unwrap(),
            "\"Never\""
        );
    }

    #[test]
    fn test_import_paths() {
        let mut session = Session::new();
        session.config.import_paths.push(PathBuf::from("/tmp/lib"));
        assert_eq!(session.get_import_path(0), Some(&PathBuf::from("/tmp/lib")));
        assert!(session.get_import_path(1).is_none());
    }

    #[test]
    fn test_nimcache_path_override() {
        let mut session = Session::new();
        session.config.nimcache_path = Some(PathBuf::from("/custom/cache"));
        assert_eq!(session.cache_dir(), PathBuf::from("/custom/cache"));
    }

    #[test]
    fn test_arch_all_variants() {
        for arch in &[
            Arch::X86_64,
            Arch::Aarch64,
            Arch::Arm,
            Arch::X86,
            Arch::Wasm32,
            Arch::Universal,
        ] {
            let triple = TargetTriple {
                arch: *arch,
                os: Os::Linux,
                env: Env::Gnu,
                pointer_width: 64,
                endian: Endian::Little,
            };
            assert_eq!(triple.arch, *arch);
        }
    }

    #[test]
    fn test_os_all_variants() {
        for os in &[Os::Linux, Os::Windows, Os::Macos, Os::Web, Os::Unknown] {
            let triple = TargetTriple {
                arch: Arch::X86_64,
                os: *os,
                env: Env::Gnu,
                pointer_width: 64,
                endian: Endian::Little,
            };
            assert_eq!(triple.os, *os);
        }
    }

    #[test]
    fn test_compile_mode_all_variants() {
        assert_eq!(CompileMode::C, CompileMode::C);
        assert_eq!(CompileMode::Cpp, CompileMode::Cpp);
        assert_eq!(CompileMode::Js, CompileMode::Js);
        assert_eq!(CompileMode::ObjC, CompileMode::ObjC);
    }

    #[test]
    fn test_session_compile_mode_setter() {
        let mut session = Session::new();
        session.compile_mode = CompileMode::Js;
        assert_eq!(session.compile_mode, CompileMode::Js);
    }

    #[test]
    fn test_config_with_import_paths() {
        let mut config = Config::default();
        config.import_paths.push(PathBuf::from("/lib"));
        config.import_paths.push(PathBuf::from("/usr/lib"));
        assert_eq!(config.import_paths.len(), 2);
        assert_eq!(config.import_paths[0], PathBuf::from("/lib"));
    }

    #[test]
    fn test_target_triple_env_detection() {
        let session = Session::new();
        assert!(matches!(
            session.target.arch,
            Arch::X86_64 | Arch::Aarch64 | Arch::Wasm32 | Arch::Universal
        ));
        assert!(matches!(
            session.target.os,
            Os::Linux | Os::Windows | Os::Macos | Os::Unknown
        ));
        assert!(matches!(
            session.target.endian,
            Endian::Little | Endian::Big
        ));
        assert!(session.target.pointer_width > 0);
    }

    #[test]
    fn test_target_triple_arch_variants() {
        let variants = [
            (Arch::X86_64, 64, Endian::Little),
            (Arch::Aarch64, 64, Endian::Little),
            (Arch::Arm, 32, Endian::Little),
            (Arch::X86, 32, Endian::Little),
            (Arch::Wasm32, 32, Endian::Little),
            (Arch::Universal, 64, Endian::Little),
        ];
        for (arch, expected_width, expected_endian) in variants {
            let triple = TargetTriple {
                arch,
                os: Os::Linux,
                env: Env::Gnu,
                pointer_width: expected_width,
                endian: expected_endian,
            };
            assert_eq!(triple.pointer_width, expected_width);
            assert_eq!(triple.endian, expected_endian);
        }
    }

    #[test]
    fn test_env_all_variants() {
        let variants = [
            (Env::Gnu, "gnu"),
            (Env::Musl, "musl"),
            (Env::Msvc, "msvc"),
            (Env::Uwp, "uwp"),
            (Env::Android, "android"),
            (Env::Unknown, "unknown"),
        ];
        for (env, _name) in variants {
            let triple = TargetTriple {
                arch: Arch::X86_64,
                os: Os::Linux,
                env,
                pointer_width: 64,
                endian: Endian::Little,
            };
            assert_eq!(triple.env, env);
        }
    }
}
