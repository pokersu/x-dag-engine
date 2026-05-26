//! Testing Utilities - Helper functions for creating test workflows and mock data
//!
//! This module provides convenient functions to create test workflows, nodes,
//! and analytics data for testing purposes. This is especially useful when
//! writing tests for applications that use oxify-model.
//!
//! # Example
//!
//! ```
//! use oxify_model::test_utils::{create_test_workflow, create_test_analytics};
//!
//! # fn example() {
//! // Create a simple test workflow
//! let workflow = create_test_workflow("test", 5);
//! assert_eq!(workflow.nodes.len(), 5);
//!
//! // Create mock analytics data
//! let analytics = create_test_analytics("test", 100, 0.85);
//! assert_eq!(analytics.execution_stats.total_executions, 100);
//! assert_eq!(analytics.execution_stats.success_rate, 0.85);
//! # }
//! ```

use crate::{
    execution::{ExecutionContext, ExecutionResult, NodeExecutionResult, NodeMetrics, TokenUsage},
    node::{LoopConfig, Node, NodeKind, ScriptConfig},
    workflow::{Workflow, WorkflowMetadata},
    Edge, WorkflowBuilder,
};
use chrono::Utc;
use std::collections::HashMap;
use uuid::Uuid;

/// Create a simple test workflow with the specified number of nodes
///
/// This creates a linear workflow with:
/// - 1 Start node
/// - N-2 LLM nodes (where N is the node_count parameter)
/// - 1 End node
///
/// # Example
///
/// ```
/// use oxify_model::test_utils::create_test_workflow;
///
/// let workflow = create_test_workflow("my_test", 5);
/// assert_eq!(workflow.nodes.len(), 5);
/// ```
pub fn create_test_workflow(name: &str, node_count: usize) -> Workflow {
    let mut builder = WorkflowBuilder::new(name);

    // Start node
    builder = builder.start("Start");

    // Add intermediate LLM nodes
    for i in 1..node_count.saturating_sub(1) {
        let code_config = ScriptConfig {
            runtime: "rhai".to_string(),
            code: format!("// step {}", i),
            inputs: vec![],
            output: "result".to_string(),
        };
        builder = builder.code(format!("Code_{}", i), code_config);
    }

    // End node
    builder = builder.end("End");

    builder.build()
}

/// Create a branching test workflow with a switch node
///
/// This creates a workflow with:
/// - 1 Start node
/// - 1 LLM node
/// - 1 Switch node (multi-branch routing)
/// - 1 End node
///
/// # Example
///
/// ```
/// use oxify_model::test_utils::create_branching_workflow;
///
/// let workflow = create_branching_workflow("branching_test");
/// assert!(workflow.nodes.len() >= 4);
/// ```
pub fn create_branching_workflow(name: &str) -> Workflow {
    use crate::node::{SwitchCase, SwitchConfig};

    let mut builder = WorkflowBuilder::new(name);

    // Start node
    builder = builder.start("Start");

    // First code node
    let code_config = ScriptConfig {
        runtime: "rhai".to_string(),
        code: "// process".to_string(),
        inputs: vec!["input".to_string()],
        output: "result".to_string(),
    };
    builder = builder.code("Process", code_config);

    // Switch branching node
    let switch_config = SwitchConfig {
        switch_on: "{{status}}".to_string(),
        cases: vec![
            SwitchCase {
                match_value: "success".to_string(),
                action: "Process success".to_string(),
            },
            SwitchCase {
                match_value: "error".to_string(),
                action: "Handle error".to_string(),
            },
        ],
        default_case: Some("Default handling".to_string()),
    };
    builder = builder.switch("Router", switch_config);

    // End node
    builder = builder.end("End");

    builder.build()
}


