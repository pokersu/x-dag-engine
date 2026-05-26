use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::time::Duration;

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Server bind address
    pub address: SocketAddr,

    /// Graceful shutdown timeout in seconds
    pub graceful_shutdown_timeout: u64,

    /// Enable request logging
    pub request_logging: bool,

    /// Enable compression
    #[cfg(feature = "compression")]
    pub compression: bool,
}

impl ServerConfig {
    /// Create a new server configuration
    pub fn new(address: SocketAddr) -> Self {
        Self {
            address,
            graceful_shutdown_timeout: 30,
            request_logging: true,
            #[cfg(feature = "compression")]
            compression: true,
        }
    }

    /// Create a development configuration
    pub fn development() -> Self {
        Self::new(SocketAddr::from(([127, 0, 0, 1], 3000)))
    }

    /// Set the bind address
    pub fn with_address(mut self, address: SocketAddr) -> Self {
        self.address = address;
        self
    }

    /// Set graceful shutdown timeout
    pub fn with_shutdown_timeout(mut self, seconds: u64) -> Self {
        self.graceful_shutdown_timeout = seconds;
        self
    }
}

/// Server result type
pub type Result<T> = std::result::Result<T, ServerError>;

/// Server errors
#[derive(Debug, thiserror::Error)]
pub enum ServerError {
    #[error("Failed to bind to address: {0}")]
    BindError(String),

    #[error("Server error: {0}")]
    ServeError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),
}

// Ensure Default is implemented
impl Default for ServerConfig {
    fn default() -> Self {
        Self::new(SocketAddr::from(([127, 0, 0, 1], 3000)))
    }
}
