# engine

DAG execution engine for API orchestration.

## Overview

`engine` executes DAG-based API orchestration workflows. It resolves execution order via topological sort, executes independent nodes in parallel, and supports retry logic with exponential backoff.

## Execution Model

```
Workflow → Validate → Topological Sort → Level-based Parallel Execution → Result
```

1. **Validation**: Cycle detection, node reference checks
2. **Topological Sort**: Kahn's algorithm — produces execution levels
3. **Level Execution**: Each level runs in parallel; levels are sequential
4. **Result Collection**: Each node's output flows into the execution context

## Node Execution

The engine dispatches to specialized executors based on node kind:

| Executor | Handles |
|----------|---------|
| `ConditionalEvaluator` | IfElse, Switch |
| `LoopExecutor` | ForEach, While, Repeat |
| `TryCatchExecutor` | Try-Catch-Finally |
| `CodeExecutor` | Rhai/WASM scripts |
| `SubWorkflowExecutor` | Nested workflows |
| `RestConnector` | HTTP requests (via Code nodes) |

## HTTP Client (`RestConnector`)

A full-featured REST API client used by workflow nodes:

```rust
use engine::rest_connector::{RestConfig, RestConnector, AuthConfig};

let config = RestConfig::new("https://api.example.com")
    .with_auth(AuthConfig::bearer("token"))
    .with_timeout_secs(30);
let connector = RestConnector::new(config);
let response = connector.get("/users/1").await?;
```

Features:
- GET/POST/PUT/PATCH/DELETE
- Bearer, API Key, Basic, OAuth2 auth
- Rate limiting with circuit breaker
- Retry with exponential backoff
- Request/response interceptor pipeline
- Response caching

## Configuration

```rust
use engine::{Engine, ExecutionConfig};

let engine = Engine::new();
let config = ExecutionConfig::default()
    .with_events()                          // emit execution events
    .with_node_timeout(30_000)              // 30s per node
    .with_max_concurrent(4)                 // limit parallelism
    .with_continue_on_error();              // don't fail on errors

let result = engine.execute_with_config(&workflow, config).await?;
```
