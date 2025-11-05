use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub general: GeneralConfig,
    #[serde(default)]
    pub providers: ProvidersConfig,
    #[serde(default)]
    pub auth: AuthConfig,
    #[serde(default)]
    pub gitea: GiteaConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    #[serde(default = "default_max_per_provider")]
    pub max_per_provider: usize,
    #[serde(default = "default_timeout_secs")]
    pub timeout_secs: u64,
    #[serde(default = "default_cache_ttl_mins")]
    pub cache_ttl_mins: u64,
    #[serde(default)]
    pub language_filter: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvidersConfig {
    #[serde(default = "default_true")]
    pub github: bool,
    #[serde(default = "default_true")]
    pub gitlab: bool,
    #[serde(default = "default_true")]
    pub gitea: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[allow(clippy::struct_field_names)]
pub struct AuthConfig {
    pub github_token: Option<String>,
    pub gitlab_token: Option<String>,
    pub gitea_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GiteaConfig {
    #[serde(default = "default_gitea_url")]
    pub base_url: String,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            max_per_provider: default_max_per_provider(),
            timeout_secs: default_timeout_secs(),
            cache_ttl_mins: default_cache_ttl_mins(),
            language_filter: vec![],
        }
    }
}

impl Default for ProvidersConfig {
    fn default() -> Self {
        Self {
            github: true,
            gitlab: true,
            gitea: true,
        }
    }
}

impl Default for GiteaConfig {
    fn default() -> Self {
        Self {
            base_url: default_gitea_url(),
        }
    }
}

fn default_max_per_provider() -> usize {
    3
}

fn default_timeout_secs() -> u64 {
    6
}

fn default_cache_ttl_mins() -> u64 {
    60
}

fn default_gitea_url() -> String {
    "https://gitea.com".to_string()
}

fn default_true() -> bool {
    true
}

impl Config {
    /// Load configuration from file, with XDG config directory support
    pub fn load() -> Result<Self> {
        // Try XDG config directory first, then current directory
        let config_paths = [
            dirs::config_dir().map(|p| p.join("trotd").join("trotd.toml")),
            Some(PathBuf::from("trotd.toml")),
        ];

        for path in config_paths.iter().flatten() {
            if path.exists() {
                let content = std::fs::read_to_string(path)
                    .with_context(|| format!("Failed to read config file: {}", path.display()))?;

                let mut config: Config = toml::from_str(&content)
                    .with_context(|| format!("Failed to parse config file: {}", path.display()))?;

                // Apply environment variable overrides
                config.apply_env_overrides();

                return Ok(config);
            }
        }

        // No config file found, use defaults with env overrides
        let mut config = Config::default();
        config.apply_env_overrides();
        Ok(config)
    }

    /// Apply environment variable overrides
    fn apply_env_overrides(&mut self) {
        if let Ok(val) = std::env::var("TROTD_MAX_PER_PROVIDER") {
            if let Ok(max) = val.parse() {
                self.general.max_per_provider = max;
            }
        }

        if let Ok(val) = std::env::var("TROTD_LANGUAGE_FILTER") {
            self.general.language_filter = val.split(',').map(|s| s.trim().to_string()).collect();
        }

        if let Ok(val) = std::env::var("TROTD_GITEA_BASE_URL") {
            self.gitea.base_url = val;
        }

        if let Ok(val) = std::env::var("TROTD_GITHUB_TOKEN") {
            self.auth.github_token = Some(val);
        }

        if let Ok(val) = std::env::var("TROTD_GITLAB_TOKEN") {
            self.auth.gitlab_token = Some(val);
        }

        if let Ok(val) = std::env::var("TROTD_GITEA_TOKEN") {
            self.auth.gitea_token = Some(val);
        }
    }

    /// Get list of enabled providers
    pub fn enabled_providers(&self) -> Vec<&str> {
        let mut providers = Vec::new();
        if self.providers.github {
            providers.push("github");
        }
        if self.providers.gitlab {
            providers.push("gitlab");
        }
        if self.providers.gitea {
            providers.push("gitea");
        }
        providers
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.general.max_per_provider, 3);
        assert_eq!(config.general.timeout_secs, 6);
        assert_eq!(config.general.cache_ttl_mins, 60);
        assert!(config.providers.github);
        assert!(config.providers.gitlab);
        assert!(config.providers.gitea);
        assert_eq!(config.gitea.base_url, "https://gitea.com");
    }

    #[test]
    fn test_enabled_providers() {
        let mut config = Config::default();
        config.providers.gitlab = false;
        let enabled = config.enabled_providers();
        assert_eq!(enabled, vec!["github", "gitea"]);
    }

    #[test]
    fn test_config_parsing() {
        let toml_str = r#"
            [general]
            max_per_provider = 5
            timeout_secs = 10
            cache_ttl_mins = 30
            language_filter = ["rust", "go"]

            [providers]
            github = true
            gitlab = false
            gitea = true

            [gitea]
            base_url = "https://codeberg.org"
        "#;

        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.general.max_per_provider, 5);
        assert_eq!(config.general.timeout_secs, 10);
        assert_eq!(config.general.language_filter, vec!["rust", "go"]);
        assert!(!config.providers.gitlab);
        assert_eq!(config.gitea.base_url, "https://codeberg.org");
    }
}
