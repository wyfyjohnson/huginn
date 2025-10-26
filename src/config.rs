use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Main configuration structure for huginn
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub display: DisplayConfig,

    #[serde(default)]
    pub challenge: ChallengeConfig,

    #[serde(default)]
    pub logo: LogoConfig,

    #[serde(default)]
    pub scripts: ScriptsConfig,
}

/// Configuration for which fields to display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayConfig {
    #[serde(default = "default_mode")]
    pub mode: String, // "normal" or "challenge"

    #[serde(default = "default_true")]
    pub distro: bool,

    #[serde(default = "default_true")]
    pub age: bool,

    #[serde(default = "default_true")]
    pub kernel: bool,

    #[serde(default = "default_true")]
    pub packages: bool,

    #[serde(default = "default_true")]
    pub shell: bool,

    #[serde(default = "default_true")]
    pub term: bool,

    #[serde(default = "default_true")]
    pub wm: bool,

    #[serde(default = "default_true")]
    pub cpu: bool,

    #[serde(default = "default_true")]
    pub gpu: bool,

    #[serde(default = "default_true")]
    pub theme: bool,

    #[serde(default = "default_true")]
    pub nix: bool,
}

/// Configuration for the challenge mode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChallengeConfig {
    #[serde(default = "default_years")]
    pub years: i64,

    #[serde(default = "default_months")]
    pub months: i64,
}

/// Configuration for the logo display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogoConfig {
    #[serde(default)]
    pub custom_path: String,

    #[serde(default)]
    pub width: Option<u32>,

    #[serde(default)]
    pub height: Option<u32>,
}

/// Configuration for custom scripts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptsConfig {
    #[serde(default)]
    pub pre_fetch: String,

    #[serde(default)]
    pub post_fetch: String,
}

// These provide defaults if values aren't in the config file

fn default_mode() -> String {
    "normal".to_string()
}

fn default_true() -> bool {
    true
}

fn default_years() -> i64 {
    2
}

fn default_months() -> i64 {
    0
}

impl Default for Config {
    fn default() -> Self {
        Self {
            display: DisplayConfig::default(),
            challenge: ChallengeConfig::default(),
            logo: LogoConfig::default(),
            scripts: ScriptsConfig::default(),
        }
    }
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            mode: default_mode(),
            distro: true,
            age: true,
            kernel: true,
            packages: true,
            shell: true,
            term: true,
            wm: true,
            cpu: true,
            gpu: true,
            theme: true,
            nix: true,
        }
    }
}

impl Default for ChallengeConfig {
    fn default() -> Self {
        Self {
            years: default_years(),
            months: default_months(),
        }
    }
}

impl Default for LogoConfig {
    fn default() -> Self {
        Self {
            custom_path: String::new(),
            width: None,
            height: None,
        }
    }
}

impl Default for ScriptsConfig {
    fn default() -> Self {
        Self {
            pre_fetch: String::new(),
            post_fetch: String::new(),
        }
    }
}

// Config loading function

impl Config {
    /// Load configuration from the standard config file location
    /// Automatically creates default config on first run
    /// Falls back to defaults if config has errors
    pub fn load() -> Self {
        // Try to find existing config file
        if let Some(config_path) = Self::find_config_file() {
            // Config exists, try to read and parse it
            if let Ok(contents) = fs::read_to_string(&config_path) {
                if let Ok(config) = toml::from_str::<Config>(&contents) {
                    return config;
                } else {
                    eprintln!(
                        "Warning: Failed to parse config file at {}",
                        config_path.display()
                    );
                    eprintln!("Run 'huginn --generate-config' to reset it, or fix the syntax.");
                    eprintln!("Using default configuration for now.");
                }
            }
        } else {
            // Config doesn't exist - this is first run!
            Self::create_default_config_silently();
        }

        // Return defaults if config doesn't exist or failed to parse
        Config::default()
    }

    /// Silently create default config on first run
    fn create_default_config_silently() {
        if let Ok(home) = std::env::var("HOME") {
            let config_path = PathBuf::from(format!("{}/.config/huginn/config.toml", home));

            // Only create if it truly doesn't exist
            if !config_path.exists() {
                let default_config = Config::default();

                if let Err(e) = default_config.save(&config_path) {
                    // Only show error if creation failed
                    eprintln!("Note: Could not create config file: {}", e);
                    eprintln!("Huginn will use defaults. You can manually run:");
                    eprintln!("  huginn --generate-config");
                }
            }
        }
    }
    /// Find the config file in standard locations
    /// Checks in order: ~/.config/huginn/config.toml, ~/.huginn.toml
    fn find_config_file() -> Option<PathBuf> {
        // Try XDG config directory first
        if let Ok(home) = std::env::var("HOME") {
            let xdg_config = PathBuf::from(format!("{}/.config/huginn/config.toml", home));
            if xdg_config.exists() {
                return Some(xdg_config);
            }

            // Try home directory fallback
            let home_config = PathBuf::from(format!("{}/.huginn.toml", home));
            if home_config.exists() {
                return Some(home_config);
            }
        }

        None
    }

    /// Save the current configuration to file
    /// Useful for generating a default config file
    pub fn save(&self, path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Serialize to TOML string
        let toml_string = toml::to_string_pretty(self)?;

        // Write to file
        fs::write(path, toml_string)?;

        Ok(())
    }

    /// Generate a default config file at ~/.config/huginn/config.toml
    pub fn generate_default_config() -> Result<(), Box<dyn std::error::Error>> {
        let home = std::env::var("HOME")?;
        let config_path = PathBuf::from(format!("{}/.config/huginn/config.toml", home));

        let default_config = Config::default();
        default_config.save(&config_path)?;

        println!("Generated default config at: {}", config_path.display());
        Ok(())
    }
}
