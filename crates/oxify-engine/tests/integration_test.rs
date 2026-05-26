//! Integration tests for the DAG engine
//!
//! Loads JSON flow definitions, executes them through the engine,
//! and verifies the execution results.

use oxify_engine::{Engine, ExecutionConfig};
use oxify_model::{Workflow, NodeKind, ExecutionResult, ExecutionState};

/// Helper: load a workflow JSON from the flows directory
fn load_flow(name: &str) -> Workflow {
    let path = format!("{}/flows/{}.json", env!("CARGO_MANIFEST_DIR"), name);
    // Go up from the crate dir to the workspace root
    let path = path.replace("/crates/oxify-engine", "");
    let content = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Failed to read flow file '{}': {}", path, e));
    serde_json::from_str(&content)
        .unwrap_or_else(|e| panic!("Failed to parse flow '{}': {}", name, e))
}

// ================================================================
//  Test 1: Simple linear workflow
// ================================================================
#[tokio::test]
async fn test_simple_linear() {
    let workflow = load_flow("simple-linear");
    let engine = Engine::new();
    let result = engine.execute(&workflow).await.unwrap();

    assert_eq!(result.state, ExecutionState::Completed,
        "Expected Completed, got {:?}", result.state);

    // Should have executed all 3 nodes
    assert_eq!(result.node_results.len(), 3,
        "Expected 3 node results, got {}", result.node_results.len());

    // The Start node should have succeeded
    let start_id = workflow.nodes[0].id;
    let start_result = result.node_results.get(&start_id)
        .expect("Start node result not found");
    assert!(matches!(start_result.result, oxify_model::ExecutionResult::Success(_)),
        "Start node should have succeeded");
}

// ================================================================
//  Test 2: Sequential execution (no parallelism)
// ================================================================
#[tokio::test]
async fn test_simple_linear_sequential() {
    let workflow = load_flow("simple-linear");
    let engine = Engine::new();
    let result = engine.execute_sequential(&workflow).await.unwrap();

    assert_eq!(result.state, ExecutionState::Completed);
    assert_eq!(result.node_results.len(), 3);
}

// ================================================================
//  Test 3: Conditional branching
// ================================================================
#[tokio::test]
async fn test_conditional_branch() {
    let workflow = load_flow("conditional-branch");
    let engine = Engine::new();
    let result = engine.execute(&workflow).await.unwrap();

    assert_eq!(result.state, ExecutionState::Completed,
        "Expected Completed, got {:?}", result.state);

    // All 7 nodes (Start, CheckStatus, Decide, BranchActive, BranchInactive, Merge, End)
    // should have results - but actually only one branch path executes
    // The engine executes all nodes in the DAG; condition evaluation just returns
    // a boolean result, it doesn't skip branches
    assert_eq!(result.node_results.len(), 7,
        "Expected 7 node results, got {}", result.node_results.len());

    // Log what each node produced
    for (id, node_result) in &result.node_results {
        let node = workflow.nodes.iter().find(|n| &n.id == id).unwrap();
        println!("  {} ({}): {:?}", node.name, id, node_result.result);
    }
}

// ================================================================
//  Test 4: Error handling with TryCatch
// ================================================================
#[tokio::test]
async fn test_error_handling() {
    let workflow = load_flow("error-handling");
    let engine = Engine::new();
    let result = engine.execute(&workflow).await.unwrap();

    assert_eq!(result.state, ExecutionState::Completed,
        "Expected Completed, got {:?}", result.state);
    assert_eq!(result.node_results.len(), 4,
        "Expected 4 node results, got {}", result.node_results.len());
}

// ================================================================
//  Test 5: Switch multi-branch routing
// ================================================================
#[tokio::test]
async fn test_switch_routing() {
    let workflow = load_flow("switch-routing");
    let engine = Engine::new();
    let result = engine.execute(&workflow).await.unwrap();

    assert_eq!(result.state, ExecutionState::Completed);
    // Start, Classifier, End = 3 nodes
    assert_eq!(result.node_results.len(), 3);
}

// ================================================================
//  Test 6: Parallel execution (fan-out)
// ================================================================
#[tokio::test]
async fn test_parallel_execution() {
    let workflow = load_flow("parallel-execution");
    let engine = Engine::new();
    let result = engine.execute(&workflow).await.unwrap();

    assert_eq!(result.state, ExecutionState::Completed,
        "Expected Completed, got {:?}", result.state);

    // Start, TaskA, TaskB, TaskC, Aggregate, End = 6 nodes
    assert_eq!(result.node_results.len(), 6,
        "Expected 6 node results, got {}", result.node_results.len());
}

// ================================================================
//  Test 7: Workflow with retry config
// ================================================================
#[tokio::test]
async fn test_with_retry() {
    let workflow = load_flow("with-retry");
    let engine = Engine::new();
    let result = engine.execute(&workflow).await.unwrap();

    assert_eq!(result.state, ExecutionState::Completed);
    assert_eq!(result.node_results.len(), 3);

    // The Code node should have retry_count = 0 (succeeded first try)
    let code_node = &workflow.nodes[1];
    let code_result = result.node_results.get(&code_node.id).unwrap();
    assert_eq!(code_result.retry_count, 0,
        "Expected no retries, got {}", code_result.retry_count);
}

// ================================================================
//  Test 8: Cycle detection (invalid workflow)
// ================================================================
#[tokio::test]
async fn test_cycle_detection() {
    use oxify_model::{Node, NodeKind, Edge, Workflow};

    let mut workflow = Workflow::new("cycle".to_string());
    let a = Node::new("A".to_string(), NodeKind::Start);
    let b = Node::new("B".to_string(), NodeKind::End);
    let a_id = a.id;
    let b_id = b.id;

    workflow.add_node(a);
    workflow.add_node(b);
    // Create cycle: A -> B -> A
    workflow.add_edge(Edge::new(a_id, b_id));
    workflow.add_edge(Edge::new(b_id, a_id));

    let engine = Engine::new();
    let result = engine.execute(&workflow).await;
    assert!(result.is_err(), "Expected cycle detection error");
    let err = result.unwrap_err().to_string();
    assert!(err.contains("Cycle") || err.contains("cycle"),
        "Error should mention cycle: {}", err);
}

// ================================================================
//  Test 9: Empty workflow validation
// ================================================================
#[tokio::test]
async fn test_empty_workflow() {
    use oxify_model::Workflow;
    let workflow = Workflow::new("empty".to_string());
    let engine = Engine::new();
    let result = engine.execute(&workflow).await;
    assert!(result.is_err(), "Expected validation error for empty workflow");
}

// ================================================================
//  Test 10: Execute with node timeout
// ================================================================
#[tokio::test]
async fn test_execution_with_timeout() {
    let workflow = load_flow("simple-linear");
    let engine = Engine::new();
    let config = ExecutionConfig::default()
        .with_node_timeout(5000) // 5 second timeout per node
        .with_events();          // enable event emission

    let result = engine.execute_with_config(&workflow, config).await.unwrap();
    assert_eq!(result.state, ExecutionState::Completed);
}

// ================================================================
//  Test 11: Validate workflow structure
// ================================================================
#[test]
fn test_workflow_validation() {
    use oxify_model::validation::WorkflowValidator;
    let workflow = load_flow("simple-linear");
    let report = WorkflowValidator::validate(&workflow).unwrap();
    assert!(report.valid,
        "Validation should pass for simple-linear: {:?}", report.warnings);
}

// ================================================================
//  Test 12: JSON round-trip serialization
// ================================================================
#[test]
fn test_json_roundtrip() {
    let workflow = load_flow("simple-linear");
    let json = serde_json::to_string_pretty(&workflow).unwrap();
    let deserialized: Workflow = serde_json::from_str(&json).unwrap();

    assert_eq!(workflow.metadata.name, deserialized.metadata.name);
    assert_eq!(workflow.nodes.len(), deserialized.nodes.len());
    assert_eq!(workflow.edges.len(), deserialized.edges.len());
}

// ================================================================
//  Test 13: Maximum concurrency limit
// ================================================================
#[tokio::test]
async fn test_max_concurrent_limit() {
    let workflow = load_flow("parallel-execution");
    let engine = Engine::new();
    let config = ExecutionConfig::default()
        .with_max_concurrent(2); // Only 2 nodes at a time

    let result = engine.execute_with_config(&workflow, config).await.unwrap();
    assert_eq!(result.state, ExecutionState::Completed);
}

// ================================================================
//  Test 14: Pause and cancel
// ================================================================
#[tokio::test]
async fn test_cancel_execution() {
    let workflow = load_flow("parallel-execution");
    let engine = Engine::new();

    // Generate a UUID for this execution
    let execution_id = uuid::Uuid::new_v4();

    // Create a context with known execution_id
    let mut ctx = oxify_model::ExecutionContext::new(workflow.metadata.id);
    ctx.execution_id = execution_id;

    // Cancel the execution immediately
    engine.cancel_execution(execution_id);
    assert!(engine.is_cancelled(execution_id));

    // Clear cancellation
    engine.clear_cancellation(execution_id);
    assert!(!engine.is_cancelled(execution_id));
}

// ================================================================
//  Test 15: Workflow builder usage
// ================================================================
#[tokio::test]
async fn test_builder_workflow() {
    use oxify_model::{WorkflowBuilder, ScriptConfig};

    let workflow = WorkflowBuilder::new("builder-test")
        .description("Built with builder pattern")
        .start("Start")
        .code("Process", ScriptConfig {
            runtime: "rhai".to_string(),
            code: "let x = 1 + 1;".to_string(),
            inputs: vec![],
            output: "result".to_string(),
        })
        .end("End")
        .build();

    assert_eq!(workflow.nodes.len(), 3);
    let engine = Engine::new();
    let result = engine.execute(&workflow).await.unwrap();
    assert_eq!(result.state, ExecutionState::Completed);
}
