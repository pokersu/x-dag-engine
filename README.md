# x-dag-engine

轻量级 DAG 编排引擎，用于 API 编排和工作流逻辑控制。

## Overview

`x-dag-engine` 是一个基于有向无环图（DAG）的编排引擎，支持将多个 API 调用、条件分支、循环、错误处理和并行执行组合成可复用的工作流。专注于**轻量、内嵌、无外部依赖**。

## Features

- **DAG 工作流** — 拓扑排序 + 分层并行执行
- **控制流节点** — IfElse / Switch / Loop / TryCatch / Parallel
- **HTTP 客户端** — RestConnector（支持 Bearer / API Key / Basic / OAuth2）
- **重试机制** — 指数退避重试
- **事件驱动** — 执行事件总线（SSE 推送）
- **定时调度** — cron 表达式调度
- **SQLite 持久化** — 工作流定义和执行记录存储
- **CLI** — 命令行运行工作流

## Architecture

```
x-dag-engine/
├── crates/
│   ├── model/       # DAG 数据模型（Workflow / Node / Edge）
│   ├── engine/      # 执行引擎（拓扑排序 + 节点执行器）
│   ├── server/      # Axum HTTP 服务
│   └── storage/     # SQLite 持久化层
├── flows/                 # 示例工作流定义（JSON）
└── tests/                 # 集成测试
```

## Quick Start

```rust
use oxify_engine::{Engine, ExecutionConfig};
use oxify_model::{Workflow, Node, NodeKind, Edge};

// 构建工作流
let mut workflow = Workflow::new("my-flow".to_string());
let start = Node::new("Start".to_string(), NodeKind::Start);
let end = Node::new("End".to_string(), NodeKind::End);

let start_id = start.id;
let end_id = end.id;

workflow.add_node(start);
workflow.add_node(end);
workflow.add_edge(Edge::new(start_id, end_id));

// 执行
let engine = Engine::new();
let result = engine.execute(&workflow).await.unwrap();
println!("{:?}", result.state); // Completed
```

## Run Tests

```bash
cargo test -p engine --test integration_test
```

## Acknowledgements

项目思路来源于 [oxify](https://github.com/cool-japan/oxify)。

## License

Apache-2.0
