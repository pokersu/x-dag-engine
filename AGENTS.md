# x-dag-engine (AGENTS.md)

## 项目来源

- **源项目**: oxify
- **源项目本地路径**: `/Users/pokersu/Projects/open-source/oxify`
- **源项目 GitHub**: https://github.com/cool-japan/oxify
- **本项目**: [pokersu/x-dag-engine](https://github.com/pokersu/x-dag-engine)

## 项目结构

```
/Users/pokersu/Projects/x-dag-engine/
├── crates/
│   ├── model/         # DAG 数据模型（Workflow / Node / Edge）
│   ├── engine/        # 执行引擎（拓扑排序 + 节点执行器）
│   ├── server/        # Axum HTTP 服务
│   └── storage/       # SQLite 持久化层
├── flows/             # 示例工作流定义（JSON）
└── tests/             # 集成测试
```

## 当前状态

- ✅ 四个核心 crate 保留自 oxify，已清除 LLM / 向量 / MCP / 认证 / Code 等无关代码
- ✅ engine 执行核心已重写（拓扑排序 + 并行执行 + 节点分派）
- ✅ 6 个 JSON flow 定义文件（Code 节点已替换为 Loop 节点）
- ✅ 已推送到 GitHub

### 节点类型

支持 9 种节点类型：

| 类型 | 说明 | 状态 |
|---|---|---|
| `Start` / `End` | 入口/出口 | ✅ 已实现 |
| `IfElse` | 条件分支（`evalexpr` + JSONPath） | ✅ 已实现 |
| `Switch` | 多路路由 | ✅ 已实现 |
| `Loop` | ForEach / While / Repeat 循环 | ✅ 已实现 |
| `TryCatch` | try-catch-finally 异常处理 | ✅ 已实现 |
| `Parallel` | 扇出/扇入并行执行 | ✅ 已实现 |
| `SubWorkflow` | 嵌套子工作流 | ✅ 已实现 |
| `Service` | HTTP 服务调用（GET/POST/PUT/DELETE，Bearer/ApiKey/Basic 认证） | ✅ 已实现 |

### 已清理的类型（原 oxify 遗留，已删除）

- ❌ `LLM` — LLM 调用节点
- ❌ `Retriever` — 向量检索节点
- ❌ `Tool` — MCP 工具调用
- ❌ `Approval` — 人工审批
- ❌ `Form` — 表单输入
- ❌ `Vision` — 图片 OCR
- ❌ `Code` — Rhai/WASM 脚本执行

## 已知缺口

### 1. 远程 Worker 执行

Service 节点如果需要由外部 Worker 执行：

- `model` 层：新增 `RemoteServiceCall` 或 `ExecutionResult::Pending`
- `engine` 层：新增 `TaskRegistry`，管理 pending/complete 状态
- `server` 层：新增 `GET /tasks/next` / `POST /tasks/:id/result` 供 Worker 拉取任务和回报结果

### 2. 变量上下文

- `node_results` 全程累积，最后一个节点能看到前面所有节点的输出
- 条件表达式支持 JSONPath 点语法（`$.node_{uuid}.field`）引用前置节点输出

### 3. 故障恢复

- 当前纯内存执行，引擎重启后正在运行的流程丢失
- 无检查点/快照机制
- 原项目有 checkpoint.rs，已被删除

### 4. 分布式执行

- 单机模型，无法跨实例分散节点执行
- 需要任务队列 + 共享状态层才能实现

## 测试

```bash
cargo test -p model                           # model 层单元测试
cargo test -p engine --test integration_test  # 集成测试
cargo test --no-run                           # 所有 crate 编译验证
```
