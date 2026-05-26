//! Comprehensive workflow validation
//!
//! This module provides detailed validation for workflows to catch errors
//! before execution, including cycle detection, orphan nodes, and structural issues.

use crate::{NodeId, NodeKind, Workflow};
use std::collections::{HashMap, HashSet, VecDeque};
use thiserror::Error;

pub type Result<T> = std::result::Result<T, ValidationError>;

#[derive(Error, Debug, Clone)]
pub enum ValidationError {
    #[error("Workflow has no start node")]
    NoStartNode,

    #[error("Workflow has multiple start nodes: {0}")]
    MultipleStartNodes(String),

    #[error("Workflow has no end node")]
    NoEndNode,

    #[error("Workflow has multiple end nodes: {0}")]
    MultipleEndNodes(String),

    #[error("Workflow contains a cycle")]
    CycleDetected,

    #[error("Node {0} is unreachable from start")]
    UnreachableNode(NodeId),

    #[error("Node {0} cannot reach end")]
    DeadEndNode(NodeId),

    #[error("Edge references non-existent node: {0}")]
    InvalidNodeReference(NodeId),

    #[error("Conditional node {0} missing true branch")]
    MissingTrueBranch(NodeId),

    #[error("Conditional node {0} missing false branch")]
    MissingFalseBranch(NodeId),

    #[error("Conditional node {0} has invalid branch: {1}")]
    InvalidConditionalBranch(NodeId, NodeId),

    #[error("Duplicate edge from {0} to {1}")]
    DuplicateEdge(NodeId, NodeId),

    #[error("Workflow has no nodes")]
    EmptyWorkflow,

    #[error("Workflow has no edges")]
    NoEdges,

    #[error("Switch node {0} has no cases defined")]
    SwitchNodeNoCases(NodeId),

    #[error("Switch node {0} has empty switch expression")]
    SwitchNodeEmptyExpression(NodeId),

    #[error("Switch node {0} case has empty match value")]
    SwitchCaseEmptyMatch(NodeId),

    #[error("Parallel node {0} has no tasks defined")]
    ParallelNodeNoTasks(NodeId),

    #[error("Parallel node {0} task '{1}' has empty expression")]
    ParallelTaskEmptyExpression(NodeId, String),

    #[error("Parallel node {0} has duplicate task ID: {1}")]
    ParallelDuplicateTaskId(NodeId, String),

    #[error("Loop node {0} has empty collection path")]
    LoopEmptyCollectionPath(NodeId),

    #[error("Loop node {0} has empty body expression")]
    LoopEmptyBodyExpression(NodeId),

    #[error("TryCatch node {0} has empty try expression")]
    TryCatchEmptyTryExpression(NodeId),

    #[error("SubWorkflow node {0} has empty workflow path")]
    SubWorkflowEmptyPath(NodeId),
}

/// Comprehensive workflow validator
pub struct WorkflowValidator;

impl WorkflowValidator {
    /// Validate a workflow and return all errors
    pub fn validate(workflow: &Workflow) -> Result<ValidationReport> {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Basic structure validation
        if workflow.nodes.is_empty() {
            return Err(ValidationError::EmptyWorkflow);
        }

        // Validate start/end nodes
        if let Err(e) = Self::validate_start_end_nodes(workflow) {
            errors.push(e);
        }

        // Validate node references in edges (must be valid before cycle detection)
        let edges_valid = if let Err(errs) = Self::validate_edge_references(workflow) {
            errors.extend(errs);
            false
        } else {
            true
        };

        // Validate conditional nodes
        if let Err(errs) = Self::validate_conditional_nodes(workflow) {
            errors.extend(errs);
        }

        // Validate new node types (Switch, Parallel, Approval, Form, etc.)
        if let Err(errs) = Self::validate_advanced_nodes(workflow) {
            errors.extend(errs);
        }

        // Only run cycle detection if all edges are valid
        if edges_valid {
            // Detect cycles
            if let Err(e) = Self::detect_cycles(workflow) {
                errors.push(e);
            }

            // Check for unreachable nodes
            if let Err(errs) = Self::find_unreachable_nodes(workflow) {
                warnings.extend(errs);
            }

            // Check for dead-end nodes
            if let Err(errs) = Self::find_dead_end_nodes(workflow) {
                warnings.extend(errs);
            }

            // Check for duplicate edges
            if let Err(errs) = Self::find_duplicate_edges(workflow) {
                warnings.extend(errs);
            }
        }

        if !errors.is_empty() {
            return Err(errors.into_iter().next().unwrap());
        }

        // Calculate stats
        let stats = Self::calculate_stats(workflow);

        Ok(ValidationReport {
            valid: true,
            warnings,
            stats,
        })
    }

    fn validate_start_end_nodes(workflow: &Workflow) -> Result<()> {
        let start_nodes: Vec<_> = workflow
            .nodes
            .iter()
            .filter(|n| matches!(n.kind, NodeKind::Start))
            .collect();

        let end_nodes: Vec<_> = workflow
            .nodes
            .iter()
            .filter(|n| matches!(n.kind, NodeKind::End))
            .collect();

        if start_nodes.is_empty() {
            return Err(ValidationError::NoStartNode);
        }

        if start_nodes.len() > 1 {
            let ids = start_nodes
                .iter()
                .map(|n| n.id.to_string())
                .collect::<Vec<_>>()
                .join(", ");
            return Err(ValidationError::MultipleStartNodes(ids));
        }

        if end_nodes.is_empty() {
            return Err(ValidationError::NoEndNode);
        }

        if end_nodes.len() > 1 {
            let ids = end_nodes
                .iter()
                .map(|n| n.id.to_string())
                .collect::<Vec<_>>()
                .join(", ");
            return Err(ValidationError::MultipleEndNodes(ids));
        }

        Ok(())
    }

    fn validate_edge_references(
        workflow: &Workflow,
    ) -> std::result::Result<(), Vec<ValidationError>> {
        let node_ids: HashSet<_> = workflow.nodes.iter().map(|n| n.id).collect();
        let mut errors = Vec::new();

        for edge in &workflow.edges {
            if !node_ids.contains(&edge.from) {
                errors.push(ValidationError::InvalidNodeReference(edge.from));
            }
            if !node_ids.contains(&edge.to) {
                errors.push(ValidationError::InvalidNodeReference(edge.to));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    fn validate_conditional_nodes(
        workflow: &Workflow,
    ) -> std::result::Result<(), Vec<ValidationError>> {
        let node_ids: HashSet<_> = workflow.nodes.iter().map(|n| n.id).collect();
        let mut errors = Vec::new();

        for node in &workflow.nodes {
            if let NodeKind::IfElse(condition) = &node.kind {
                if !node_ids.contains(&condition.true_branch) {
                    errors.push(ValidationError::InvalidConditionalBranch(
                        node.id,
                        condition.true_branch,
                    ));
                }
                if !node_ids.contains(&condition.false_branch) {
                    errors.push(ValidationError::InvalidConditionalBranch(
                        node.id,
                        condition.false_branch,
                    ));
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    fn validate_advanced_nodes(
        workflow: &Workflow,
    ) -> std::result::Result<(), Vec<ValidationError>> {
        let mut errors = Vec::new();

        for node in &workflow.nodes {
            match &node.kind {
                // Validate Switch nodes
                NodeKind::Switch(config) => {
                    if config.switch_on.trim().is_empty() {
                        errors.push(ValidationError::SwitchNodeEmptyExpression(node.id));
                    }
                    if config.cases.is_empty() && config.default_case.is_none() {
                        errors.push(ValidationError::SwitchNodeNoCases(node.id));
                    }
                    for case in &config.cases {
                        if case.match_value.trim().is_empty() {
                            errors.push(ValidationError::SwitchCaseEmptyMatch(node.id));
                        }
                    }
                }

                // Validate Parallel nodes
                NodeKind::Parallel(config) => {
                    if config.tasks.is_empty() {
                        errors.push(ValidationError::ParallelNodeNoTasks(node.id));
                    }
                    let mut seen_ids = HashSet::new();
                    for task in &config.tasks {
                        if task.expression.trim().is_empty() {
                            errors.push(ValidationError::ParallelTaskEmptyExpression(
                                node.id,
                                task.id.clone(),
                            ));
                        }
                        if !seen_ids.insert(&task.id) {
                            errors.push(ValidationError::ParallelDuplicateTaskId(
                                node.id,
                                task.id.clone(),
                            ));
                        }
                    }
                }

                // Validate Loop nodes
                NodeKind::Loop(config) => match &config.loop_type {
                    crate::LoopType::ForEach {
                        collection_path,
                        body_expression,
                        ..
                    } => {
                        if collection_path.trim().is_empty() {
                            errors.push(ValidationError::LoopEmptyCollectionPath(node.id));
                        }
                        if body_expression.trim().is_empty() {
                            errors.push(ValidationError::LoopEmptyBodyExpression(node.id));
                        }
                    }
                    crate::LoopType::While {
                        body_expression, ..
                    } => {
                        if body_expression.trim().is_empty() {
                            errors.push(ValidationError::LoopEmptyBodyExpression(node.id));
                        }
                    }
                    crate::LoopType::Repeat {
                        body_expression, ..
                    } => {
                        if body_expression.trim().is_empty() {
                            errors.push(ValidationError::LoopEmptyBodyExpression(node.id));
                        }
                    }
                },

                // Validate TryCatch nodes
                NodeKind::TryCatch(config) => {
                    if config.try_expression.trim().is_empty() {
                        errors.push(ValidationError::TryCatchEmptyTryExpression(node.id));
                    }
                }

                // Validate SubWorkflow nodes
                NodeKind::SubWorkflow(config) => {
                    if config.workflow_path.trim().is_empty() {
                        errors.push(ValidationError::SubWorkflowEmptyPath(node.id));
                    }
                }

                // Other node types don't need advanced validation here
                _ => {}
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    fn detect_cycles(workflow: &Workflow) -> Result<()> {
        let mut in_degree: HashMap<NodeId, usize> = HashMap::new();
        let mut adj_list: HashMap<NodeId, Vec<NodeId>> = HashMap::new();

        // Initialize
        for node in &workflow.nodes {
            in_degree.insert(node.id, 0);
            adj_list.insert(node.id, Vec::new());
        }

        // Build adjacency list
        for edge in &workflow.edges {
            adj_list.get_mut(&edge.from).unwrap().push(edge.to);
            *in_degree.get_mut(&edge.to).unwrap() += 1;
        }

        // Kahn's algorithm
        let mut queue: VecDeque<_> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(&id, _)| id)
            .collect();

        let mut processed = 0;

        while let Some(node_id) = queue.pop_front() {
            processed += 1;

            if let Some(neighbors) = adj_list.get(&node_id) {
                for &neighbor in neighbors {
                    let deg = in_degree.get_mut(&neighbor).unwrap();
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push_back(neighbor);
                    }
                }
            }
        }

        if processed != workflow.nodes.len() {
            return Err(ValidationError::CycleDetected);
        }

        Ok(())
    }

    fn find_unreachable_nodes(
        workflow: &Workflow,
    ) -> std::result::Result<(), Vec<ValidationError>> {
        let start_node = workflow
            .nodes
            .iter()
            .find(|n| matches!(n.kind, NodeKind::Start));

        if start_node.is_none() {
            return Ok(()); // Already caught by start/end validation
        }

        let start_id = start_node.unwrap().id;

        // Build adjacency list
        let mut adj_list: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
        for node in &workflow.nodes {
            adj_list.insert(node.id, Vec::new());
        }
        for edge in &workflow.edges {
            adj_list.get_mut(&edge.from).unwrap().push(edge.to);
        }

        // BFS from start
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(start_id);
        visited.insert(start_id);

        while let Some(node_id) = queue.pop_front() {
            if let Some(neighbors) = adj_list.get(&node_id) {
                for &neighbor in neighbors {
                    if visited.insert(neighbor) {
                        queue.push_back(neighbor);
                    }
                }
            }
        }

        // Find unreachable nodes
        let errors: Vec<_> = workflow
            .nodes
            .iter()
            .filter(|n| !visited.contains(&n.id))
            .map(|n| ValidationError::UnreachableNode(n.id))
            .collect();

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    fn find_dead_end_nodes(workflow: &Workflow) -> std::result::Result<(), Vec<ValidationError>> {
        let end_node = workflow
            .nodes
            .iter()
            .find(|n| matches!(n.kind, NodeKind::End));

        if end_node.is_none() {
            return Ok(()); // Already caught by start/end validation
        }

        let end_id = end_node.unwrap().id;

        // Build reverse adjacency list
        let mut reverse_adj: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
        for node in &workflow.nodes {
            reverse_adj.insert(node.id, Vec::new());
        }
        for edge in &workflow.edges {
            reverse_adj.get_mut(&edge.to).unwrap().push(edge.from);
        }

        // BFS from end (backwards)
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(end_id);
        visited.insert(end_id);

        while let Some(node_id) = queue.pop_front() {
            if let Some(predecessors) = reverse_adj.get(&node_id) {
                for &pred in predecessors {
                    if visited.insert(pred) {
                        queue.push_back(pred);
                    }
                }
            }
        }

        // Find dead-end nodes
        let errors: Vec<_> = workflow
            .nodes
            .iter()
            .filter(|n| !visited.contains(&n.id))
            .map(|n| ValidationError::DeadEndNode(n.id))
            .collect();

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    fn find_duplicate_edges(workflow: &Workflow) -> std::result::Result<(), Vec<ValidationError>> {
        let mut seen = HashSet::new();
        let mut errors = Vec::new();

        for edge in &workflow.edges {
            let pair = (edge.from, edge.to);
            if !seen.insert(pair) {
                errors.push(ValidationError::DuplicateEdge(edge.from, edge.to));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    fn calculate_stats(workflow: &Workflow) -> ValidationStats {
        let total_nodes = workflow.nodes.len();
        let total_edges = workflow.edges.len();

        let start_nodes = workflow
            .nodes
            .iter()
            .filter(|n| matches!(n.kind, NodeKind::Start))
            .count();

        let end_nodes = workflow
            .nodes
            .iter()
            .filter(|n| matches!(n.kind, NodeKind::End))
            .count();

        // Calculate max depth using BFS
        let max_depth = Self::calculate_max_depth_bfs(workflow);

        // Count node types
        let mut node_type_counts = HashMap::new();
        for node in &workflow.nodes {
            let type_name = match &node.kind {
                NodeKind::Start => "Start",
                NodeKind::End => "End",
                NodeKind::IfElse(_) => "IfElse",
                NodeKind::Loop(_) => "Loop",
                NodeKind::TryCatch(_) => "TryCatch",
                NodeKind::SubWorkflow(_) => "SubWorkflow",
                NodeKind::Switch(_) => "Switch",
                NodeKind::Parallel(_) => "Parallel",
                NodeKind::Service(_) => "Service",
            };
            *node_type_counts.entry(type_name.to_string()).or_insert(0) += 1;
        }

        ValidationStats {
            total_nodes,
            total_edges,
            start_nodes,
            end_nodes,
            max_depth,
            node_type_counts,
        }
    }

    fn calculate_max_depth_bfs(workflow: &Workflow) -> usize {
        let start_node = workflow
            .nodes
            .iter()
            .find(|n| matches!(n.kind, NodeKind::Start));

        if start_node.is_none() {
            return 0;
        }

        let start_id = start_node.unwrap().id;

        // Build adjacency list
        let mut adj_list: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
        for node in &workflow.nodes {
            adj_list.insert(node.id, Vec::new());
        }
        for edge in &workflow.edges {
            adj_list.get_mut(&edge.from).unwrap().push(edge.to);
        }

        // BFS to calculate depth
        let mut queue = VecDeque::new();
        let mut depths = HashMap::new();

        queue.push_back(start_id);
        depths.insert(start_id, 0);

        let mut max_depth = 0;

        while let Some(node_id) = queue.pop_front() {
            let depth = *depths.get(&node_id).unwrap();
            max_depth = max_depth.max(depth);

            if let Some(neighbors) = adj_list.get(&node_id) {
                for &neighbor in neighbors {
                    use std::collections::hash_map::Entry;
                    if let Entry::Vacant(e) = depths.entry(neighbor) {
                        e.insert(depth + 1);
                        queue.push_back(neighbor);
                    }
                }
            }
        }

        max_depth
    }
}

/// Validation report with detailed statistics
#[derive(Debug, Clone)]
pub struct ValidationReport {
    pub valid: bool,
    pub warnings: Vec<ValidationError>,
    pub stats: ValidationStats,
}

/// Validation statistics
#[derive(Debug, Clone)]
pub struct ValidationStats {
    pub total_nodes: usize,
    pub total_edges: usize,
    pub start_nodes: usize,
    pub end_nodes: usize,
    pub max_depth: usize,
    pub node_type_counts: std::collections::HashMap<String, usize>,
}


