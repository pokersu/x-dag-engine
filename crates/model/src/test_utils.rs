//! Testing Utilities - Helper functions for creating test workflows and mock data
//!
//! This module provides convenient functions to create test workflows, nodes,
//! and analytics data for testing purposes. This is especially useful when
//! writing tests for applications that use model.
//!
//! # Example
//!
//! ```
//! use model::test_utils::create_test_workflow;
//!
//! let workflow = create_test_workflow("test", 5);
//! assert_eq!(workflow.nodes.len(), 5);
//! ```

use crate::{
    execution::{ExecutionContext, ExecutionResult, NodeExecutionResult, NodeMetrics, TokenUsage},
    node::{LoopConfig, LoopType, Node, NodeKind},
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
/// - N-2 Loop nodes (where N is the node_count parameter)
/// - 1 End node
///
/// # Example
///
/// ```
/// use model::test_utils::create_test_workflow;
///
/// let workflow = create_test_workflow("my_test", 5);
/// assert_eq!(workflow.nodes.len(), 5);
/// ```
pub fn create_test_workflow(name: &str, node_count: usize) -> Workflow {
    let mut workflow = Workflow::new(name.to_string());
    let mut prev_id = None;

    // Start node
    let start = Node::new("Start".to_string(), NodeKind::Start);
    prev_id = Some(start.id);
    workflow.add_node(start);

    // Add intermediate Loop nodes
    for i in 1..node_count.saturating_sub(1) {
        let loop_config = LoopConfig {
            loop_type: LoopType::Repeat {
                count: "1".to_string(),
                body_expression: format!("// step {}", i),
                index_variable: None,
            },
            max_iterations: 1,
        };
        let node = Node::new(format!("Step_{}", i), NodeKind::Loop(loop_config));
        let node_id = node.id;
        workflow.add_node(node);
        workflow.add_edge(Edge::new(prev_id.unwrap(), node_id));
        prev_id = Some(node_id);
    }

    // End node
    let end = Node::new("End".to_string(), NodeKind::End);
    workflow.add_node(end);
    workflow.add_edge(Edge::new(prev_id.unwrap(), workflow.nodes.last().unwrap().id));

    workflow
}

/// Create a branching test workflow with a switch node
///
/// This creates a workflow with:
/// - 1 Start node
/// - 1 Loop node
/// - 1 Switch node (multi-branch routing)
/// - 1 End node
///
/// # Example
///
/// ```
/// use model::test_utils::create_branching_workflow;
///
/// let workflow = create_branching_workflow("branching_test");
/// assert!(workflow.nodes.len() >= 4);
/// ```
pub fn create_branching_workflow(name: &str) -> Workflow {
    use crate::node::{SwitchCase, SwitchConfig};

    let mut builder = WorkflowBuilder::new(name);

    // Start node
    builder = builder.start("Start");

    // Intermediate loop node
    let loop_config = LoopConfig {
        loop_type: LoopType::Repeat {
            count: "1".to_string(),
            body_expression: "// process".to_string(),
            index_variable: None,
        },
        max_iterations: 1,
    };
    builder = builder.loop_node("Process", loop_config);

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
