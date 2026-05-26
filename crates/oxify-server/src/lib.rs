//! HTTP server for DAG-based API orchestration engine

pub mod error;
pub mod rate_limit;
pub mod server;
pub mod shutdown;
pub mod sse;
pub mod tls;
pub mod tracing_config;
pub mod types;

pub use error::{AppError, ProblemDetails};
pub use rate_limit::{RateLimitConfig, RateLimiter};
pub use server::ServerRuntime;
pub use shutdown::shutdown_signal;
pub use sse::{SseConnectionManager, SseEventBroadcaster, SseEventType};
pub use tls::{CertificateInfo, CertificateMonitor, TlsConfig, TlsError, TlsVersion};
pub use tracing_config::{TracingConfig, TracingGuard};
pub use types::{Result, ServerConfig, ServerError};
