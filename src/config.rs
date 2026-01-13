use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::fs;
use std::io;
use thiserror::Error;

/// Project configuration structure
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProjectConfig {
    /// List of project root paths to scan for sessions
    #[serde(default)]
    pub project_root_paths: Vec<PathBuf>,
}

impl ProjectConfig {
    /// Load configuration from a TOML file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        
        // If file doesn't exist, return default config
        if !path.exists() {
            return Ok(Self::default());
        }
        
        let content = fs::read_to_string(path)
            .map_err(|e| ConfigError::IoError {
                path: path.to_path_buf(),
                source: e,
            })?;
        
        let config: Self = toml::from_str(&content)
            .map_err(|e| ConfigError::ParseError {
                path: path.to_path_buf(),
                source: e,
            })?;
        
        Ok(config)
    }
    
    /// Validate that all configured paths exist
    pub fn validate(&self) -> Result<(), ConfigError> {
        for path in &self.project_root_paths {
            if !path.exists() {
                return Err(ConfigError::PathNotFound {
                    path: path.clone(),
                });
            }
        }
        Ok(())
    }
    
    /// Save configuration to a TOML file
    pub fn to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), ConfigError> {
        let path = path.as_ref();
        
        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent()
            && !parent.exists()
        {
            fs::create_dir_all(parent)
                .map_err(|e| ConfigError::IoError {
                    path: parent.to_path_buf(),
                    source: e,
                })?;
        }
        
        let content = toml::to_string_pretty(self)
            .map_err(|e| ConfigError::SerializeError {
                source: e,
            })?;
        
        fs::write(path, content)
            .map_err(|e| ConfigError::IoError {
                path: path.to_path_buf(),
                source: e,
            })?;
        
        Ok(())
    }
}

/// Configuration errors
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Failed to read config file {path}: {source}")]
    IoError {
        path: PathBuf,
        source: io::Error,
    },
    #[error("Failed to parse config file {path}: {source}")]
    ParseError {
        path: PathBuf,
        source: toml::de::Error,
    },
    #[error("Failed to serialize config: {source}")]
    SerializeError {
        source: toml::ser::Error,
    },
    #[error("Configured path does not exist: {path}")]
    PathNotFound {
        path: PathBuf,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_default_config() {
        let config = ProjectConfig::default();
        assert!(config.project_root_paths.is_empty());
    }
    
    #[test]
    fn test_validate_empty_config() {
        let config = ProjectConfig::default();
        assert!(config.validate().is_ok());
    }
    
    #[test]
    fn test_validate_invalid_path() {
        let mut config = ProjectConfig::default();
        config.project_root_paths.push(PathBuf::from("/nonexistent/path/that/does/not/exist"));
        assert!(config.validate().is_err());
    }
}
