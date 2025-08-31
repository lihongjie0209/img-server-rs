use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub compression: CompressionConfig,
    pub logging: LoggingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub max_file_size_mb: usize,
    pub worker_threads: Option<usize>,
    pub enable_cors: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionConfig {
    pub default_quality: u8,
    pub default_algorithm: String,
    pub enable_cache: bool,
    pub cache_ttl_minutes: u32,
    pub max_concurrent_jobs: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub enable_request_logging: bool,
    pub log_compression_stats: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            compression: CompressionConfig::default(),
            logging: LoggingConfig::default(),
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 3030,
            max_file_size_mb: 100,
            worker_threads: None, // Use system default
            enable_cors: true,
        }
    }
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            default_quality: 80,
            default_algorithm: "mozjpeg".to_string(),
            enable_cache: false,
            cache_ttl_minutes: 60,
            max_concurrent_jobs: 10,
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            enable_request_logging: true,
            log_compression_stats: true,
        }
    }
}

impl Config {
    /// Load configuration from file, falling back to defaults if file doesn't exist
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        
        if !path.exists() {
            log::info!("Config file {:?} not found, using defaults", path);
            return Ok(Self::default());
        }

        let content = fs::read_to_string(path)
            .map_err(|e| ConfigError::IoError(format!("Failed to read config file: {}", e)))?;

        let config: Config = toml::from_str(&content)
            .map_err(|e| ConfigError::ParseError(format!("Failed to parse config: {}", e)))?;

        // Validate configuration
        config.validate()?;

        log::info!("Loaded configuration from {:?}", path);
        Ok(config)
    }

    /// Load configuration from environment variables and file
    pub fn load() -> Result<Self, ConfigError> {
        let config_path = std::env::var("IMG_SERVER_CONFIG")
            .unwrap_or_else(|_| "config.toml".to_string());

        let mut config = Self::load_from_file(&config_path)?;

        // Override with environment variables if present
        config.apply_env_overrides();

        Ok(config)
    }

    /// Apply environment variable overrides
    fn apply_env_overrides(&mut self) {
        if let Ok(host) = std::env::var("IMG_SERVER_HOST") {
            self.server.host = host;
        }

        if let Ok(port) = std::env::var("IMG_SERVER_PORT") {
            if let Ok(port_num) = port.parse::<u16>() {
                self.server.port = port_num;
            }
        }

        if let Ok(max_size) = std::env::var("IMG_SERVER_MAX_FILE_SIZE_MB") {
            if let Ok(size) = max_size.parse::<usize>() {
                self.server.max_file_size_mb = size;
            }
        }

        if let Ok(quality) = std::env::var("IMG_SERVER_DEFAULT_QUALITY") {
            if let Ok(q) = quality.parse::<u8>() {
                if q >= 1 && q <= 100 {
                    self.compression.default_quality = q;
                }
            }
        }

        if let Ok(algorithm) = std::env::var("IMG_SERVER_DEFAULT_ALGORITHM") {
            self.compression.default_algorithm = algorithm;
        }

        if let Ok(log_level) = std::env::var("RUST_LOG") {
            self.logging.level = log_level;
        }
    }

    /// Validate configuration values
    fn validate(&self) -> Result<(), ConfigError> {
        if self.server.port == 0 {
            return Err(ConfigError::ValidationError("Port cannot be 0".to_string()));
        }

        if self.server.max_file_size_mb == 0 {
            return Err(ConfigError::ValidationError("Max file size cannot be 0".to_string()));
        }

        if !(1..=100).contains(&self.compression.default_quality) {
            return Err(ConfigError::ValidationError(
                "Default quality must be between 1 and 100".to_string()
            ));
        }

        let valid_algorithms = ["mozjpeg", "jpeg-encoder", "png-quantized"];
        if !valid_algorithms.contains(&self.compression.default_algorithm.as_str()) {
            return Err(ConfigError::ValidationError(
                format!("Invalid default algorithm. Must be one of: {:?}", valid_algorithms)
            ));
        }

        Ok(())
    }

    /// Generate a sample configuration file
    pub fn generate_sample_config<P: AsRef<Path>>(path: P) -> Result<(), ConfigError> {
        let config = Self::default();
        let toml_content = toml::to_string_pretty(&config)
            .map_err(|e| ConfigError::SerializeError(format!("Failed to serialize config: {}", e)))?;

        fs::write(path, toml_content)
            .map_err(|e| ConfigError::IoError(format!("Failed to write config file: {}", e)))?;

        Ok(())
    }

    /// Get the bind address for the server
    pub fn bind_address(&self) -> String {
        format!("{}:{}", self.server.host, self.server.port)
    }

    /// Get max file size in bytes
    pub fn max_file_size_bytes(&self) -> usize {
        self.server.max_file_size_mb * 1024 * 1024
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    IoError(String),
    
    #[error("Parse error: {0}")]
    ParseError(String),
    
    #[error("Validation error: {0}")]
    ValidationError(String),
    
    #[error("Serialization error: {0}")]
    SerializeError(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.server.host, "0.0.0.0");
        assert_eq!(config.server.port, 3030);
        assert_eq!(config.compression.default_quality, 80);
    }

    #[test]
    fn test_config_validation() {
        let mut config = Config::default();
        
        // Valid config should pass
        assert!(config.validate().is_ok());
        
        // Invalid quality should fail
        config.compression.default_quality = 0;
        assert!(config.validate().is_err());
        
        config.compression.default_quality = 101;
        assert!(config.validate().is_err());
        
        // Invalid algorithm should fail
        config.compression.default_quality = 80;
        config.compression.default_algorithm = "invalid".to_string();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_env_overrides() {
        env::set_var("IMG_SERVER_HOST", "127.0.0.1");
        env::set_var("IMG_SERVER_PORT", "8080");
        env::set_var("IMG_SERVER_DEFAULT_QUALITY", "90");
        
        let mut config = Config::default();
        config.apply_env_overrides();
        
        assert_eq!(config.server.host, "127.0.0.1");
        assert_eq!(config.server.port, 8080);
        assert_eq!(config.compression.default_quality, 90);
        
        // Clean up
        env::remove_var("IMG_SERVER_HOST");
        env::remove_var("IMG_SERVER_PORT");
        env::remove_var("IMG_SERVER_DEFAULT_QUALITY");
    }

    #[test]
    fn test_bind_address() {
        let config = Config::default();
        assert_eq!(config.bind_address(), "0.0.0.0:3030");
    }

    #[test]
    fn test_max_file_size_bytes() {
        let config = Config::default();
        assert_eq!(config.max_file_size_bytes(), 100 * 1024 * 1024);
    }
}
