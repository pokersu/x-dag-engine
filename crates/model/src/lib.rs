//! Domain models for DAG-based API orchestration engine

pub mod builder;
pub mod edge;
pub mod execution;
pub mod json_schema;
pub mod linter;
pub mod node;
pub mod schedule;
pub mod template;
pub mod test_utils;
pub mod validation;
pub mod visualization;
pub mod workflow;
pub mod yaml;

pub use builder::{NodeBuilder, WorkflowBuilder};
pub use edge::{Edge, EdgeId};
pub use execution::{
    ExecutionContext, ExecutionResult, ExecutionState, NodeExecutionResult,
    NodeMetrics, TokenUsage,
};
pub use json_schema::{
    generate_workflow_schema, schema_to_json, schema_to_value, JsonSchema, WorkflowSchemaGenerator,
};
pub use linter::{
    LintCategory, LintFinding, LintResult, LintSeverity, LintStats, LinterConfig, WorkflowLinter,
};
pub use node::{
    Condition, LoopConfig,
    LoopType, Node, NodeId, NodeKind, ParallelConfig, ParallelStrategy, ParallelTask,
    RetryConfig, SubWorkflowConfig, SwitchCase, SwitchConfig, TimeoutAction,
    TimeoutConfig, TryCatchConfig,
};
pub use schedule::{Schedule, ScheduleExecution, ScheduleId};
pub use template::{
    InstantiateTemplateRequest, ParameterType, ParameterValidation, TemplateId, TemplateListItem,
    TemplateParameter, WorkflowTemplate,
};
pub use validation::{ValidationError, ValidationReport, ValidationStats, WorkflowValidator};
pub use visualization::{
    workflow_to_graphviz, workflow_to_mermaid, workflow_to_plantuml, DiagramOrientation,
    VisualizationFormat, VisualizationStyle, WorkflowVisualizer,
};
pub use workflow::{VersionBump, Workflow, WorkflowId, WorkflowMetadata, WorkflowSchedule};
pub use yaml::{
    json_to_yaml, load_template_yaml, load_workflow_yaml, save_template_yaml, save_workflow_yaml,
    template_from_yaml, template_to_yaml, workflow_from_yaml, workflow_to_yaml, yaml_to_json,
    YamlError,
};
