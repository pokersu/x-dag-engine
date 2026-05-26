//! DAG execution engine for API orchestration

mod conditional;
mod event_bus;
mod loop_executor;
mod retry;
mod rest_connector;
mod scheduler;
mod subworkflow_executor;
mod testing;
mod try_catch_executor;
mod variable_store;
mod webhook;

pub use conditional::ConditionalEvaluator;
pub use event_bus::{
    execution_events, EventBus, EventHandler, EventType, WorkflowEvent, WorkflowTrigger,
};
pub use loop_executor::LoopExecutor;
pub use rest_connector::{
    AuthConfig, CircuitBreaker, CircuitBreakerConfig, CircuitState, GraphQLQuery,
    HeaderInjectionInterceptor, LoggingInterceptor, RateLimitConfig, RequestInterceptor,
    RequestTemplate, ResponseInterceptor, RestConfig, RestConnector, RestConnectorError,
    RestResponse, RetryConfig,
};
pub use retry::retry_with_backoff;
pub use scheduler::{ScheduleId, ScheduledExecution, WorkflowScheduler};
pub use subworkflow_executor::SubWorkflowExecutor;
pub use testing::{
    AssertionResult, ExpectedStatus, ExpectedValue, TestReport, TestResult, TestSuite,
    WorkflowTestCase, WorkflowTestRunner,
};
pub use try_catch_executor::TryCatchExecutor;
pub use variable_store::{Variable, VariableStore, VariableStoreStats};
pub use webhook::{WebhookConfig, WebhookId, WebhookRegistry, WebhookTrigger};

use model::{
    ExecutionContext, ExecutionResult, ExecutionState, Node, NodeExecutionResult, NodeId, NodeKind,
    Workflow,
};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, EngineError>;

#[derive(Error, Debug)]
pub enum EngineError {
    #[error("Workflow validation failed: {0}")]
    ValidationError(String),
    #[error("Node not found: {0}")]
    NodeNotFound(NodeId),
    #[error("Execution error: {0}")]
    ExecutionError(String),
    #[error("Cycle detected in workflow")]
    CycleDetected,
    #[error("Variable not found: {0}")]
    VariableNotFound(String),
    #[error("Template error: {0}")]
    TemplateError(String),
    #[error("Timeout: {0}")]
    Timeout(String),
    #[error("Execution paused")]
    ExecutionPaused,
    #[error("Execution cancelled")]
    ExecutionCancelled,
}

#[derive(Debug, Clone)]
pub struct ExecutionConfig {
    pub emit_events: bool,
    pub node_timeout_ms: Option<u64>,
    pub max_concurrent_nodes: Option<usize>,
    pub continue_on_error: bool,
}

impl Default for ExecutionConfig {
    fn default() -> Self {
        Self {
            emit_events: false,
            node_timeout_ms: None,
            max_concurrent_nodes: None,
            continue_on_error: false,
        }
    }
}

impl ExecutionConfig {
    pub fn new() -> Self { Self::default() }
    pub fn with_events(mut self) -> Self { self.emit_events = true; self }
    pub fn with_node_timeout(mut self, t: u64) -> Self { self.node_timeout_ms = Some(t); self }
    pub fn with_max_concurrent(mut self, m: usize) -> Self { self.max_concurrent_nodes = Some(m); self }
    pub fn with_continue_on_error(mut self) -> Self { self.continue_on_error = true; self }
}

#[derive(Default)]
pub struct EngineBuilder {
    event_bus: Option<Arc<EventBus>>,
}

impl EngineBuilder {
    pub fn new() -> Self { Self::default() }
    pub fn with_event_bus(mut self, bus: Arc<EventBus>) -> Self { self.event_bus = Some(bus); self }
    pub fn build(self) -> Engine {
        Engine {
            event_bus: self.event_bus,
            pause_flag: Arc::new(std::sync::RwLock::new(HashMap::new())),
            cancel_flag: Arc::new(std::sync::RwLock::new(HashMap::new())),
        }
    }
}

/// DAG execution engine
pub struct Engine {
    event_bus: Option<Arc<EventBus>>,
    pause_flag: Arc<std::sync::RwLock<HashMap<uuid::Uuid, bool>>>,
    cancel_flag: Arc<std::sync::RwLock<HashMap<uuid::Uuid, bool>>>,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            event_bus: None,
            pause_flag: Arc::new(std::sync::RwLock::new(HashMap::new())),
            cancel_flag: Arc::new(std::sync::RwLock::new(HashMap::new())),
        }
    }

    pub fn builder() -> EngineBuilder { EngineBuilder::new() }

    pub fn with_event_bus(event_bus: Arc<EventBus>) -> Self {
        Self {
            event_bus: Some(event_bus),
            pause_flag: Arc::new(std::sync::RwLock::new(HashMap::new())),
            cancel_flag: Arc::new(std::sync::RwLock::new(HashMap::new())),
        }
    }

    pub fn set_event_bus(&mut self, bus: Arc<EventBus>) { self.event_bus = Some(bus); }
    pub fn event_bus(&self) -> Option<&Arc<EventBus>> { self.event_bus.as_ref() }

    pub fn pause_execution(&self, id: uuid::Uuid) {
        self.pause_flag.write().unwrap().insert(id, true);
    }

    pub fn resume_execution(&self, id: uuid::Uuid) {
        self.pause_flag.write().unwrap().insert(id, false);
    }

    pub fn is_paused(&self, id: uuid::Uuid) -> bool {
        self.pause_flag.read().unwrap().get(&id).copied().unwrap_or(false)
    }

    pub fn cancel_execution(&self, id: uuid::Uuid) {
        self.cancel_flag.write().unwrap().insert(id, true);
    }

    pub fn is_cancelled(&self, id: uuid::Uuid) -> bool {
        self.cancel_flag.read().unwrap().get(&id).copied().unwrap_or(false)
    }

    pub fn clear_cancellation(&self, id: uuid::Uuid) {
        self.cancel_flag.write().unwrap().remove(&id);
    }

    /// Emit an event
    async fn emit_event(&self, event: WorkflowEvent) {
        if let Some(bus) = &self.event_bus {
            let _ = bus.publish(event).await;
        }
    }

    /// Execute workflow sequentially (for sub-workflows)
    pub async fn execute_sequential(&self, workflow: &Workflow) -> Result<ExecutionContext> {
        workflow.validate().map_err(EngineError::ValidationError)?;
        let mut ctx = ExecutionContext::new(workflow.metadata.id);
        let levels = self.compute_execution_levels(workflow)?;
        for level_nodes in levels {
            for node_id in &level_nodes {
                let node = workflow.get_node(node_id).ok_or(EngineError::NodeNotFound(*node_id))?;
                let result = self.execute_node_with_retry(node, &ctx).await?;
                ctx.record_node_result(*node_id, result);
            }
        }
        ctx.state = ExecutionState::Completed;
        ctx.mark_completed();
        Ok(ctx)
    }

    /// Execute with default config
    pub async fn execute(&self, workflow: &Workflow) -> Result<ExecutionContext> {
        self.execute_with_config(workflow, ExecutionConfig::default()).await
    }

    /// Execute with custom config (parallel execution)
    pub async fn execute_with_config(
        &self,
        workflow: &Workflow,
        config: ExecutionConfig,
    ) -> Result<ExecutionContext> {
        workflow.validate().map_err(EngineError::ValidationError)?;
        let mut ctx = ExecutionContext::new(workflow.metadata.id);
        let execution_id = ctx.execution_id;
        let workflow_id = workflow.metadata.id;

        let levels = self.compute_execution_levels(workflow)?;

        if config.emit_events {
            self.emit_event(WorkflowEvent::workflow_started(workflow_id, execution_id)).await;
        }

        for level_nodes in &levels {
            if self.is_cancelled(execution_id) {
                self.clear_cancellation(execution_id);
                return Err(EngineError::ExecutionCancelled);
            }
            if self.is_paused(execution_id) {
                return Err(EngineError::ExecutionPaused);
            }

            if config.emit_events {
                self.emit_event(WorkflowEvent::level_started(
                    workflow_id, execution_id, 0, level_nodes.len(),
                )).await;
            }

            // Spawn all nodes at this level in parallel
            let mut handles = Vec::new();
            let max_concurrent = config.max_concurrent_nodes.unwrap_or(level_nodes.len()).min(level_nodes.len());

            for chunk in level_nodes.chunks(max_concurrent) {
                for node_id in chunk {
                    let node = workflow.get_node(node_id)
                        .ok_or(EngineError::NodeNotFound(*node_id))?
                        .clone();
                    let ctx_clone = ctx.clone();
                    let wf_clone = workflow.clone();
                    let timeout = config.node_timeout_ms;
                    let emit = config.emit_events;
                    let bus = self.event_bus.clone();

                    let handle = tokio::spawn(async move {
                        if emit {
                            if let Some(ref b) = bus {
                                let _ = b.publish(WorkflowEvent::node_started(
                                    workflow_id, execution_id, node.id, &node.name,
                                )).await;
                            }
                        }
                        let engine = Engine::new();
                        let result = if let Some(ms) = timeout {
                            match tokio::time::timeout(
                                tokio::time::Duration::from_millis(ms),
                                engine.execute_node_with_retry(&node, &ctx_clone),
                            ).await {
                                Ok(r) => r,
                                Err(_) => Err(EngineError::Timeout(format!("Node '{}' timed out", node.name))),
                            }
                        } else {
                            engine.execute_node_with_retry(&node, &ctx_clone).await
                        };
                        (node.id, result)
                    });
                    handles.push(handle);
                }

                for handle in handles.drain(..) {
                    let (node_id, result) = handle.await
                        .map_err(|e| EngineError::ExecutionError(format!("Task join: {}", e)))?;
                    match result {
                        Ok(r) => { ctx.record_node_result(node_id, r); }
                        Err(e) if config.continue_on_error => {
                            let mut f = NodeExecutionResult::new();
                            f = f.complete(ExecutionResult::Failure(e.to_string()));
                            ctx.record_node_result(node_id, f);
                        }
                        Err(e) => {
                            if config.emit_events {
                                self.emit_event(WorkflowEvent::workflow_failed(
                                    workflow_id, execution_id, &e.to_string(),
                                )).await;
                            }
                            return Err(e);
                        }
                    }
                }
            }
        }

        ctx.state = ExecutionState::Completed;
        ctx.mark_completed();
        if config.emit_events {
            self.emit_event(WorkflowEvent::workflow_completed(workflow_id, execution_id, 0, 0)).await;
        }
        Ok(ctx)
    }

    /// Execute a node with retry (exponential backoff)
    async fn execute_node_with_retry(
        &self,
        node: &Node,
        ctx: &ExecutionContext,
    ) -> Result<NodeExecutionResult> {
        let retry_config = node.retry_config.as_ref();
        let max_retries = retry_config.map(|c| c.max_retries).unwrap_or(0);
        let mut attempt = 0;
        loop {
            match self.execute_node(node, ctx).await {
                Ok(mut result) => {
                    result.retry_count = attempt;
                    return Ok(result);
                }
                Err(e) => {
                    attempt += 1;
                    if attempt > max_retries {
                        let mut result = NodeExecutionResult::new();
                        result.retry_count = attempt - 1;
                        result = result.complete(ExecutionResult::Failure(
                            format!("Failed after {} retries: {}", max_retries, e),
                        ));
                        return Ok(result);
                    }
                    if let Some(cfg) = retry_config {
                        let delay = (cfg.initial_delay_ms as f64
                            * cfg.backoff_multiplier.powi((attempt - 1) as i32)) as u64;
                        let delay = delay.min(cfg.max_delay_ms);
                        tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
                    }
                }
            }
        }
    }

    /// Execute a single node by dispatching to the correct handler
    async fn execute_node(
        &self,
        node: &Node,
        ctx: &ExecutionContext,
    ) -> Result<NodeExecutionResult> {
        let result = match &node.kind {
            NodeKind::Start => ExecutionResult::Success(serde_json::json!({})),
            NodeKind::End => ExecutionResult::Success(serde_json::json!({})),
            NodeKind::IfElse(condition) => {
                let evaluator = ConditionalEvaluator::new(ctx)
                    .map_err(|e| EngineError::ExecutionError(e.to_string()))?;
                let result = evaluator.evaluate(&condition.expression)
                    .map_err(|e| EngineError::ExecutionError(e.to_string()))?;
                ExecutionResult::Success(serde_json::json!({ "condition_result": result }))
            }
            NodeKind::Loop(config) => {
                let value = LoopExecutor::execute(config, ctx).await
                    .map_err(|e| EngineError::ExecutionError(e.to_string()))?;
                ExecutionResult::Success(serde_json::json!({ "loop_result": value }))
            }
            NodeKind::TryCatch(config) => {
                let value = TryCatchExecutor::execute(config, ctx).await
                    .map_err(|e| EngineError::ExecutionError(e.to_string()))?;
                ExecutionResult::Success(serde_json::to_value(value).unwrap_or_default())
            }
            NodeKind::Switch(config) => {
                let evaluator = ConditionalEvaluator::new(ctx)
                    .map_err(|e| EngineError::ExecutionError(e.to_string()))?;
                let switched = evaluator.evaluate(&config.switch_on)
                    .map_err(|e| EngineError::ExecutionError(e.to_string()))?;
                ExecutionResult::Success(serde_json::json!({ "switch_result": switched }))
            }
            NodeKind::Parallel(config) => {
                ExecutionResult::Success(serde_json::json!({ "parallel": true }))
            }
            NodeKind::SubWorkflow(config) => {
                let value = SubWorkflowExecutor::execute(config, ctx).await
                    .map_err(|e| EngineError::ExecutionError(e.to_string()))?;
                ExecutionResult::Success(serde_json::to_value(value).unwrap_or_default())
            }
            _ => ExecutionResult::Failure(format!("Unsupported node type: {:?}", node.kind)),
        };

        let mut node_result = NodeExecutionResult::new();
        node_result = node_result.complete(result);
        Ok(node_result)
    }

    /// Compute execution levels via topological sort (BFS)
    fn compute_execution_levels(&self, workflow: &Workflow) -> Result<Vec<Vec<NodeId>>> {
        let mut in_degree: HashMap<NodeId, usize> = HashMap::new();
        let mut adj_list: HashMap<NodeId, Vec<NodeId>> = HashMap::new();

        for node in &workflow.nodes {
            in_degree.insert(node.id, 0);
            adj_list.insert(node.id, Vec::new());
        }
        for edge in &workflow.edges {
            adj_list.get_mut(&edge.from).unwrap().push(edge.to);
            *in_degree.get_mut(&edge.to).unwrap() += 1;
        }

        let mut queue: Vec<NodeId> = in_degree.iter()
            .filter(|(_, &d)| d == 0).map(|(&id, _)| id).collect();
        let mut levels: Vec<Vec<NodeId>> = Vec::new();
        let mut processed = 0;

        while !queue.is_empty() {
            let current = queue.clone();
            levels.push(current.clone());
            processed += current.len();

            let mut next = Vec::new();
            for nid in current {
                if let Some(neighbors) = adj_list.get(&nid) {
                    for &n in neighbors {
                        let d = in_degree.get_mut(&n).unwrap();
                        *d -= 1;
                        if *d == 0 { next.push(n); }
                    }
                }
            }
            queue = next;
        }

        if processed != workflow.nodes.len() {
            return Err(EngineError::CycleDetected);
        }
        Ok(levels)
    }
}

impl Default for Engine {
    fn default() -> Self { Self::new() }
}
