# model

Core data model for DAG-based API orchestration engine.

## Overview

`model` provides the data structures for defining and managing API orchestration workflows as directed acyclic graphs (DAGs). Workflows are composed of typed nodes connected by edges, with support for conditional branching, loops, error handling, and parallel execution.

## Key Types

### Workflow

```rust
pub struct Workflow {
    pub metadata: WorkflowMetadata,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}
```

### Node

A node in the workflow DAG. Supported node kinds:

| Kind | Purpose |
|------|---------|
| `Start` | Entry point |
| `End` | Exit point |
| `Code` | Execute Rhai/WASM scripts |
| `IfElse` | Conditional branching |
| `Switch` | Multi-branch routing |
| `Loop` | ForEach / While / Repeat |
| `TryCatch` | Error handling |
| `Parallel` | Fan-out/fan-in |
| `SubWorkflow` | Nested workflow execution |

### Edge

```rust
pub struct Edge {
    pub from: NodeId,
    pub to: NodeId,
}
```

## Validation

Workflows are validated before execution:
- Cycle detection via topological sort
- All node references exist
- Start/End nodes present
