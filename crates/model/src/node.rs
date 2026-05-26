use serde::{Deserialize, Serialize};
use uuid::Uuid;



/// Unique identifier for a node
pub type NodeId = Uuid;

/// Node in the workflow DAG
#[derive(Debug, Clone, Serialize, Deserialize)]

pub struct Node {
    /// Unique node identifier
    #[cfg_attr(feature = "openapi", schema(value_type = String))]
    pub id: NodeId,

    /// Display name of the node
    pub name: String,

    /// Type and configuration of the node
    pub kind: NodeKind,

    /// Position in the visual editor (optional)
    pub position: Option<(f64, f64)>,

    /// Retry configuration for this node (optional)
    #[serde(default)]
    pub retry_config: Option<RetryConfig>,

    /// Timeout configuration for this node (optional)
    #[serde(default)]
    pub timeout_config: Option<TimeoutConfig>,
}

/// Retry configuration for nodes
#[derive(Debug, Clone, Serialize, Deserialize)]

pub struct RetryConfig {
    /// Maximum number of retry attempts (default: 3)
    pub max_retries: u32,

    /// Initial delay before first retry in milliseconds (default: 1000ms)
    pub initial_delay_ms: u64,

    /// Backoff multiplier for exponential backoff (default: 2.0)
    pub backoff_multiplier: f64,

    /// Maximum delay between retries in milliseconds (default: 30000ms = 30s)
    pub max_delay_ms: u64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay_ms: 1000,
            backoff_multiplier: 2.0,
            max_delay_ms: 30000,
        }
    }
}

/// Timeout configuration for nodes
#[derive(Debug, Clone, Serialize, Deserialize)]

pub struct TimeoutConfig {
    /// Maximum execution time in milliseconds
    pub execution_timeout_ms: u64,

    /// Idle timeout - max time with no progress in milliseconds (optional)
    #[serde(default)]
    pub idle_timeout_ms: Option<u64>,

    /// Action to take on timeout
    #[serde(default)]
    pub timeout_action: TimeoutAction,
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            execution_timeout_ms: 60000, // 1 minute default
            idle_timeout_ms: None,
            timeout_action: TimeoutAction::Fail,
        }
    }
}

/// Action to take when a timeout occurs
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]

pub enum TimeoutAction {
    /// Fail the node execution (default)
    #[default]
    Fail,

    /// Skip this node and continue with default/empty output
    Skip,

    /// Use a default value for the output
    UseDefault(String),
}

impl Node {
    pub fn new(name: String, kind: NodeKind) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            kind,
            position: None,
            retry_config: None,
            timeout_config: None,
        }
    }

    pub fn with_retry(mut self, retry_config: RetryConfig) -> Self {
        self.retry_config = Some(retry_config);
        self
    }

    pub fn with_timeout(mut self, timeout_config: TimeoutConfig) -> Self {
        self.timeout_config = Some(timeout_config);
        self
    }

    pub fn with_position(mut self, x: f64, y: f64) -> Self {
        self.position = Some((x, y));
        self
    }
}

/// Types of nodes available in the workflow
#[derive(Debug, Clone, Serialize, Deserialize)]

#[serde(tag = "type", content = "config")]
pub enum NodeKind {
    /// Start node - entry point of the workflow
    Start,

    /// End node - exit point of the workflow
    End,

    /// Conditional branching node
    IfElse(Condition),

    /// Loop/iteration node
    Loop(LoopConfig),

    /// Error handling node (try-catch-finally)
    TryCatch(TryCatchConfig),

    /// Sub-workflow execution node
    SubWorkflow(SubWorkflowConfig),

    /// Switch/Case multi-branch routing
    Switch(SwitchConfig),

    /// Parallel execution node (fan-out/fan-in)
    Parallel(ParallelConfig),

    /// HTTP service call node
    Service(ServiceConfig),
}

/// Configuration for conditional nodes
#[derive(Debug, Clone, Serialize, Deserialize)]

pub struct Condition {
    /// JavaScript-like expression to evaluate
    pub expression: String,

    /// Node to execute if condition is true
    #[cfg_attr(feature = "openapi", schema(value_type = String))]
    pub true_branch: NodeId,

    /// Node to execute if condition is false
    #[cfg_attr(feature = "openapi", schema(value_type = String))]
    pub false_branch: NodeId,
}

/// Configuration for loop nodes
#[derive(Debug, Clone, Serialize, Deserialize)]

pub struct LoopConfig {
    /// Type of loop to execute
    pub loop_type: LoopType,

    /// Maximum iterations allowed (safety limit)
    #[serde(default = "default_max_iterations")]
    pub max_iterations: usize,
}

fn default_max_iterations() -> usize {
    1000
}

/// Types of loops supported
#[derive(Debug, Clone, Serialize, Deserialize)]

#[serde(tag = "variant")]
pub enum LoopType {
    /// Iterate over a collection (like map/forEach)
    ForEach {
        /// Path to array variable in context (e.g., "items", "results")
        collection_path: String,

        /// Variable name to bind each item (e.g., "item")
        item_variable: String,

        /// Optional index variable (e.g., "index")
        #[serde(default)]
        index_variable: Option<String>,

        /// Expression or template to execute for each item
        /// Can reference {{item}}, {{index}}, and other context variables
        body_expression: String,

        /// Enable parallel execution of loop iterations
        #[serde(default)]
        parallel: bool,

        /// Maximum number of concurrent iterations (only used when parallel=true)
        /// If None, uses default concurrency limit
        #[serde(default)]
        max_concurrency: Option<usize>,
    },

    /// Iterate while condition is true
    While {
        /// Condition expression (evaluated each iteration)
        condition: String,

        /// Expression to execute each iteration
        body_expression: String,

        /// Optional counter variable name
        #[serde(default)]
        counter_variable: Option<String>,
    },

    /// Repeat N times
    Repeat {
        /// Number of iterations (can be template like "{{count}}")
        count: String,

        /// Expression to execute each iteration
        body_expression: String,

        /// Variable name for iteration index (0-based)
        #[serde(default)]
        index_variable: Option<String>,
    },
}

/// Configuration for try-catch error handling nodes
#[derive(Debug, Clone, Serialize, Deserialize)]

pub struct TryCatchConfig {
    /// Expression or template to try executing
    pub try_expression: String,

    /// Optional expression to execute if try fails
    /// Can access {{error}} variable containing the error message
    #[serde(default)]
    pub catch_expression: Option<String>,

    /// Optional expression to always execute (after try or catch)
    #[serde(default)]
    pub finally_expression: Option<String>,

    /// Whether to re-throw the error after catch (default: false)
    /// If true, the node will still fail even after executing catch
    #[serde(default)]
    pub rethrow: bool,

    /// Variable name to store the error in catch block (default: "error")
    #[serde(default = "default_error_variable")]
    pub error_variable: String,
}

fn default_error_variable() -> String {
    "error".to_string()
}

/// Configuration for sub-workflow execution nodes
#[derive(Debug, Clone, Serialize, Deserialize)]

pub struct SubWorkflowConfig {
    /// Path to JSON file containing the workflow to execute
    pub workflow_path: String,

    /// Input mappings: map parent context variables to sub-workflow variables
    /// Format: {"sub_var_name": "{{parent_var_name}}"}
    #[serde(default)]
    pub input_mappings: std::collections::HashMap<String, String>,

    /// Output variable name to extract from sub-workflow results
    /// If not specified, all sub-workflow results are returned
    #[serde(default)]
    pub output_variable: Option<String>,

    /// Whether to inherit parent context variables (default: false)
    #[serde(default)]
    pub inherit_context: bool,
}

/// Configuration for switch/case multi-branch routing
#[derive(Debug, Clone, Serialize, Deserialize)]

pub struct SwitchConfig {
    /// Expression to evaluate for routing (e.g., "{{status}}", "{{node_x.result}}")
    pub switch_on: String,

    /// List of cases to match against
    pub cases: Vec<SwitchCase>,

    /// Default case if no matches (optional)
    /// If None and no match, the node fails
    #[serde(default)]
    pub default_case: Option<String>,
}

/// A single case in a switch statement
#[derive(Debug, Clone, Serialize, Deserialize)]

pub struct SwitchCase {
    /// Value to match (e.g., "success", "error", "pending")
    /// Supports exact match or regex if prefixed with "regex:"
    pub match_value: String,

    /// Expression or action to execute if matched
    /// Can be a simple value or template expression
    pub action: String,
}

/// Configuration for parallel execution node
#[derive(Debug, Clone, Serialize, Deserialize)]

pub struct ParallelConfig {
    /// Parallel execution strategy
    pub strategy: ParallelStrategy,

    /// List of expressions/tasks to execute in parallel
    /// Each can reference context variables
    pub tasks: Vec<ParallelTask>,

    /// Maximum number of concurrent tasks (default: no limit)
    #[serde(default)]
    pub max_concurrency: Option<usize>,

    /// Timeout for all tasks in milliseconds (default: no timeout)
    #[serde(default)]
    pub timeout_ms: Option<u64>,
}

/// Strategy for parallel execution
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]

pub enum ParallelStrategy {
    /// Wait for all tasks to complete (fan-out/fan-in)
    /// Fails if any task fails
    WaitAll,

    /// Wait for first task to complete successfully
    /// Ignore other tasks once one succeeds
    Race,

    /// Wait for all tasks, but don't fail if some fail
    /// Collect both successes and failures
    AllSettled,
}

/// A task to execute in parallel
#[derive(Debug, Clone, Serialize, Deserialize)]

pub struct ParallelTask {
    /// Task identifier (used for result mapping)
    pub id: String,

    /// Expression or template to execute
    pub expression: String,

    /// Optional description of what this task does
    #[serde(default)]
    pub description: Option<String>,
}

/// Configuration for HTTP service call nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    /// Request URL (can contain {{variable}} placeholders)
    pub url: String,

    /// HTTP method (GET, POST, PUT, PATCH, DELETE)
    #[serde(default = "default_service_method")]
    pub method: String,

    /// Request headers
    #[serde(default)]
    pub headers: std::collections::HashMap<String, String>,

    /// Request body (JSON, with {{variable}} support)
    #[serde(default)]
    pub body: Option<serde_json::Value>,

    /// Query parameters
    #[serde(default)]
    pub query_params: std::collections::HashMap<String, String>,

    /// Authentication configuration
    #[serde(default)]
    pub auth: ServiceAuth,

    /// Request timeout in seconds (default: 30)
    #[serde(default = "default_service_timeout")]
    pub timeout_secs: u64,
}

fn default_service_method() -> String {
    "GET".to_string()
}

fn default_service_timeout() -> u64 {
    30
}

/// Authentication configuration for Service nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServiceAuth {
    /// No authentication
    #[serde(rename = "none")]
    None,
    /// Bearer token authentication
    Bearer { token: String },
    /// API key in header or query parameter
    ApiKey {
        key: String,
        value: String,
        #[serde(default)]
        in_header: bool,
    },
    /// Basic authentication
    Basic { username: String, password: String },
    /// OAuth2 client credentials
    OAuth2 {
        client_id: String,
        client_secret: String,
        token_url: String,
        #[serde(default)]
        scopes: Vec<String>,
    },
}

impl Default for ServiceAuth {
    fn default() -> Self {
        Self::None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_node() {
        let config = ServiceConfig {
            url: "https://api.example.com/data".to_string(),
            method: "POST".to_string(),
            headers: [("Content-Type".to_string(), "application/json".to_string())].into(),
            body: Some(serde_json::json!({"key": "value"})),
            query_params: [("page".to_string(), "1".to_string())].into(),
            auth: ServiceAuth::Bearer { token: "tok_xxx".to_string() },
            timeout_secs: 15,
        };

        let node = Node::new("API Call".to_string(), NodeKind::Service(config));

        assert_eq!(node.name, "API Call");
        if let NodeKind::Service(cfg) = &node.kind {
            assert_eq!(cfg.url, "https://api.example.com/data");
            assert_eq!(cfg.method, "POST");
            assert!(matches!(cfg.auth, ServiceAuth::Bearer { .. }));
        } else {
            panic!("Expected Service node");
        }

        // Round-trip JSON
        let json = serde_json::to_string(&node).unwrap();
        let restored: Node = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.name, "API Call");
    }

    #[test]
    fn test_switch_node() {
        let switch_config = SwitchConfig {
            switch_on: "{{status}}".to_string(),
            cases: vec![
                SwitchCase {
                    match_value: "success".to_string(),
                    action: "process_success".to_string(),
                },
                SwitchCase {
                    match_value: "error".to_string(),
                    action: "handle_error".to_string(),
                },
            ],
            default_case: Some("unknown_status".to_string()),
        };

        let node = Node::new(
            "Status Router".to_string(),
            NodeKind::Switch(switch_config.clone()),
        );

        assert_eq!(node.name, "Status Router");
        if let NodeKind::Switch(config) = &node.kind {
            assert_eq!(config.switch_on, "{{status}}");
            assert_eq!(config.cases.len(), 2);
            assert_eq!(config.default_case, Some("unknown_status".to_string()));
        } else {
            panic!("Expected Switch node");
        }
    }

    #[test]
    fn test_parallel_node() {
        let parallel_config = ParallelConfig {
            strategy: ParallelStrategy::WaitAll,
            tasks: vec![
                ParallelTask {
                    id: "task1".to_string(),
                    expression: "{{query1}}".to_string(),
                    description: Some("First query".to_string()),
                },
                ParallelTask {
                    id: "task2".to_string(),
                    expression: "{{query2}}".to_string(),
                    description: Some("Second query".to_string()),
                },
            ],
            max_concurrency: Some(2),
            timeout_ms: Some(30000),
        };

        let node = Node::new(
            "Parallel Execution".to_string(),
            NodeKind::Parallel(parallel_config.clone()),
        );

        assert_eq!(node.name, "Parallel Execution");
        if let NodeKind::Parallel(config) = &node.kind {
            assert_eq!(config.strategy, ParallelStrategy::WaitAll);
            assert_eq!(config.tasks.len(), 2);
            assert_eq!(config.max_concurrency, Some(2));
            assert_eq!(config.timeout_ms, Some(30000));
        } else {
            panic!("Expected Parallel node");
        }
    }

    #[test]
    fn test_parallel_strategy_race() {
        let parallel_config = ParallelConfig {
            strategy: ParallelStrategy::Race,
            tasks: vec![
                ParallelTask {
                    id: "fast".to_string(),
                    expression: "{{fast_api}}".to_string(),
                    description: None,
                },
                ParallelTask {
                    id: "slow".to_string(),
                    expression: "{{slow_api}}".to_string(),
                    description: None,
                },
            ],
            max_concurrency: None,
            timeout_ms: None,
        };

        let node = Node::new(
            "Race Condition".to_string(),
            NodeKind::Parallel(parallel_config),
        );

        if let NodeKind::Parallel(config) = &node.kind {
            assert_eq!(config.strategy, ParallelStrategy::Race);
        } else {
            panic!("Expected Parallel node");
        }
    }

    #[test]
    fn test_node_with_retry_and_timeout() {
        let node = Node::new("Resilient Node".to_string(), NodeKind::Start)
            .with_retry(RetryConfig {
                max_retries: 5,
                initial_delay_ms: 500,
                backoff_multiplier: 3.0,
                max_delay_ms: 60000,
            })
            .with_timeout(TimeoutConfig {
                execution_timeout_ms: 10000,
                idle_timeout_ms: Some(5000),
                timeout_action: TimeoutAction::Skip,
            });

        assert!(node.retry_config.is_some());
        assert!(node.timeout_config.is_some());

        if let Some(retry) = &node.retry_config {
            assert_eq!(retry.max_retries, 5);
            assert_eq!(retry.backoff_multiplier, 3.0);
        }

        if let Some(timeout) = &node.timeout_config {
            assert_eq!(timeout.execution_timeout_ms, 10000);
            assert_eq!(timeout.timeout_action, TimeoutAction::Skip);
        }
    }

    #[test]
    fn test_foreach_parallel_execution() {
        let loop_config = LoopConfig {
            loop_type: LoopType::ForEach {
                collection_path: "items".to_string(),
                item_variable: "item".to_string(),
                index_variable: Some("idx".to_string()),
                body_expression: "process({{item}})".to_string(),
                parallel: true,
                max_concurrency: Some(10),
            },
            max_iterations: 1000,
        };

        let node = Node::new("Parallel Loop".to_string(), NodeKind::Loop(loop_config));

        if let NodeKind::Loop(config) = &node.kind {
            if let LoopType::ForEach {
                parallel,
                max_concurrency,
                collection_path,
                item_variable,
                ..
            } = &config.loop_type
            {
                assert!(parallel);
                assert_eq!(*max_concurrency, Some(10));
                assert_eq!(collection_path, "items");
                assert_eq!(item_variable, "item");
            } else {
                panic!("Expected ForEach loop");
            }
        } else {
            panic!("Expected Loop node");
        }
    }

    #[test]
    fn test_foreach_sequential_execution() {
        let loop_config = LoopConfig {
            loop_type: LoopType::ForEach {
                collection_path: "items".to_string(),
                item_variable: "item".to_string(),
                index_variable: None,
                body_expression: "process({{item}})".to_string(),
                parallel: false,
                max_concurrency: None,
            },
            max_iterations: 1000,
        };

        let node = Node::new("Sequential Loop".to_string(), NodeKind::Loop(loop_config));

        if let NodeKind::Loop(config) = &node.kind {
            if let LoopType::ForEach {
                parallel,
                max_concurrency,
                ..
            } = &config.loop_type
            {
                assert!(!parallel);
                assert_eq!(*max_concurrency, None);
            } else {
                panic!("Expected ForEach loop");
            }
        } else {
            panic!("Expected Loop node");
        }
    }

    #[test]
    fn test_foreach_serialization_with_parallel() {
        let loop_config = LoopConfig {
            loop_type: LoopType::ForEach {
                collection_path: "data".to_string(),
                item_variable: "x".to_string(),
                index_variable: Some("i".to_string()),
                body_expression: "{{x}} * 2".to_string(),
                parallel: true,
                max_concurrency: Some(5),
            },
            max_iterations: 100,
        };

        // Test JSON serialization
        let json = serde_json::to_string(&loop_config).unwrap();
        let deserialized: LoopConfig = serde_json::from_str(&json).unwrap();

        if let LoopType::ForEach {
            parallel,
            max_concurrency,
            ..
        } = deserialized.loop_type
        {
            assert!(parallel);
            assert_eq!(max_concurrency, Some(5));
        } else {
            panic!("Expected ForEach loop");
        }
    }
}
