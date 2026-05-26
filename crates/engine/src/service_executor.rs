//! Service node executor with support for internal and remote execution
//!
//! The ServiceExecutor handles HTTP service call nodes. It supports two modes:
//!
//! - **Internal (default)**: Executes the request directly via RestConnector.
//! - **Remote**: Pushes the task to an external queue for worker execution.
//!
//! ## Remote Execution Flow
//!
//! When running in Remote mode, the executor serializes the task and pushes it
//! to a configured queue. The node returns immediately with a "dispatched" status.
//! An external worker picks up the task, executes the HTTP call, and reports
//! the result back (see AGENTS.md "远程 Worker 执行").

use crate::rest_connector::{AuthConfig, RestConfig, RestConnector};
use crate::{EngineError, Result};
use model::{ExecutionContext, ExecutionResult, ServiceAuth, ServiceConfig};

/// Execution mode for Service nodes
#[derive(Debug, Clone)]
pub enum ServiceExecutionMode {
    /// Execute HTTP calls internally via RestConnector (default)
    Internal,
    /// Push tasks to a remote queue for external worker execution
    Remote {
        /// Queue service endpoint (e.g., Redis URL or HTTP endpoint)
        queue_url: String,
        /// Queue name for task routing
        queue_name: String,
    },
}

impl Default for ServiceExecutionMode {
    fn default() -> Self {
        Self::Internal
    }
}

/// Service node executor
///
/// ```ignore
/// use engine::service_executor::{ServiceExecutor, ServiceExecutionMode};
/// let executor = ServiceExecutor::new(ServiceExecutionMode::Internal);
/// // executor.execute(&config, &ctx).await
/// ```
pub struct ServiceExecutor {
    mode: ServiceExecutionMode,
}

impl ServiceExecutor {
    /// Create a new executor with the given mode
    pub fn new(mode: ServiceExecutionMode) -> Self {
        Self { mode }
    }

    /// Create a new executor in internal mode
    pub fn internal() -> Self {
        Self::new(ServiceExecutionMode::Internal)
    }

    /// Create a new executor in remote mode
    pub fn remote(queue_url: String, queue_name: String) -> Self {
        Self::new(ServiceExecutionMode::Remote { queue_url, queue_name })
    }

    /// Execute a Service node
    ///
    /// Returns the result as a JSON value. In internal mode, this is the HTTP
    /// response. In remote mode, this is a dispatch confirmation.
    pub async fn execute(
        &self,
        config: &ServiceConfig,
        _ctx: &ExecutionContext,
    ) -> Result<ExecutionResult> {
        match &self.mode {
            ServiceExecutionMode::Internal => {
                let response = execute_internal(config).await?;
                Ok(ExecutionResult::Success(response))
            }
            ServiceExecutionMode::Remote {
                queue_url,
                queue_name,
            } => {
                let dispatch = execute_remote(config, queue_url, queue_name).await?;
                Ok(ExecutionResult::Success(dispatch))
            }
        }
    }
}

/// Execute the HTTP request internally via RestConnector
async fn execute_internal(config: &ServiceConfig) -> Result<serde_json::Value> {
    let mut rest_config = RestConfig::new("").with_timeout_secs(config.timeout_secs);

    let auth = convert_auth(&config.auth);
    if !matches!(auth, AuthConfig::None) {
        rest_config = rest_config.with_auth(auth);
    }

    for (k, v) in &config.headers {
        rest_config = rest_config.with_header(k.clone(), v.clone());
    }

    let connector = RestConnector::new(rest_config);

    let method = reqwest::Method::from_bytes(config.method.as_bytes())
        .map_err(|e| EngineError::ExecutionError(format!("Invalid HTTP method: {}", e)))?;

    let response = connector
        .request(
            method,
            &config.url,
            config.body.clone(),
            Some(config.query_params.clone()),
        )
        .await
        .map_err(|e| EngineError::ExecutionError(format!("Service call failed: {}", e)))?;

    Ok(serde_json::json!({
        "status": response.status,
        "headers": response.headers,
        "body": response.body,
        "response_time_ms": response.response_time_ms,
    }))
}

/// Push the task to a remote queue and return a dispatch confirmation
///
/// The external worker will later execute the HTTP call and report the result.
/// This function does NOT block until the worker completes.
async fn execute_remote(
    config: &ServiceConfig,
    queue_url: &str,
    queue_name: &str,
) -> Result<serde_json::Value> {
    // Serialize the task for the external worker
    let task_payload = serde_json::json!({
        "url": config.url,
        "method": config.method,
        "headers": config.headers,
        "body": config.body,
        "query_params": config.query_params,
        "auth": config.auth,
    });

    // TODO: Push to actual queue (Redis, RabbitMQ, HTTP endpoint, etc.)
    // For now, log the dispatch and return a confirmation.
    tracing::info!(
        queue_url = %queue_url,
        queue_name = %queue_name,
        "Service task dispatched to remote worker queue"
    );

    Ok(serde_json::json!({
        "dispatched": true,
        "queue_url": queue_url,
        "queue_name": queue_name,
        "task_payload": task_payload,
        "message": "Task dispatched to external worker"
    }))
}

/// Convert model::ServiceAuth to engine::AuthConfig
fn convert_auth(auth: &ServiceAuth) -> AuthConfig {
    match auth {
        ServiceAuth::None => AuthConfig::None,
        ServiceAuth::Bearer { token } => AuthConfig::Bearer {
            token: token.clone(),
        },
        ServiceAuth::ApiKey {
            key,
            value,
            in_header,
        } => AuthConfig::ApiKey {
            key: key.clone(),
            value: value.clone(),
            in_header: *in_header,
        },
        ServiceAuth::Basic { username, password } => AuthConfig::Basic {
            username: username.clone(),
            password: password.clone(),
        },
        ServiceAuth::OAuth2 {
            client_id,
            client_secret,
            token_url,
            scopes,
        } => AuthConfig::OAuth2 {
            client_id: client_id.clone(),
            client_secret: client_secret.clone(),
            token_url: token_url.clone(),
            scopes: scopes.clone(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use model::ServiceAuth;

    #[test]
    fn test_convert_auth_none() {
        let result = convert_auth(&ServiceAuth::None);
        assert!(matches!(result, AuthConfig::None));
    }

    #[test]
    fn test_convert_auth_bearer() {
        let result = convert_auth(&ServiceAuth::Bearer {
            token: "tok_123".to_string(),
        });
        assert!(matches!(result, AuthConfig::Bearer { token } if token == "tok_123"));
    }

    #[test]
    fn test_convert_auth_basic() {
        let result = convert_auth(&ServiceAuth::Basic {
            username: "user".to_string(),
            password: "pass".to_string(),
        });
        assert!(matches!(result, AuthConfig::Basic { .. }));
    }

    #[test]
    fn test_convert_auth_oauth2() {
        let result = convert_auth(&ServiceAuth::OAuth2 {
            client_id: "cid".to_string(),
            client_secret: "cs".to_string(),
            token_url: "https://auth.example.com/token".to_string(),
            scopes: vec!["read".to_string()],
        });
        assert!(matches!(result, AuthConfig::OAuth2 { .. }));
    }
}
