use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::models::AppConfig;

/// Returns the configuration directory: `~/.hype-trader/`
pub fn config_dir() -> PathBuf {
    dirs::home_dir()
        .expect("could not determine home directory")
        .join(".hype-trader")
}

/// Returns the path to the config file: `~/.hype-trader/config.toml`
pub fn config_path() -> PathBuf {
    config_dir().join("config.toml")
}

/// Loads the application config from disk. Returns default config if the file
/// does not exist.
pub fn load_config() -> Result<AppConfig> {
    let path = config_path();
    if !path.exists() {
        return Ok(AppConfig::default());
    }
    let contents = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read config file: {}", path.display()))?;
    let config: AppConfig = toml::from_str(&contents)
        .with_context(|| format!("failed to parse config file: {}", path.display()))?;
    Ok(config)
}

/// Saves the application config to disk. Creates the config directory if it
/// does not exist, and sets file permissions to 0o600.
pub fn save_config(config: &AppConfig) -> Result<()> {
    let dir = config_dir();
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("failed to create config directory: {}", dir.display()))?;

    let contents = toml::to_string_pretty(config).context("failed to serialize config")?;

    let path = config_path();
    std::fs::write(&path, &contents)
        .with_context(|| format!("failed to write config file: {}", path.display()))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o600);
        std::fs::set_permissions(&path, perms)
            .with_context(|| format!("failed to set permissions on {}", path.display()))?;
    }

    Ok(())
}
