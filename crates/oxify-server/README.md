# oxify-server

HTTP server for DAG-based API orchestration engine. Built on Axum and Tower.

## Overview

`oxify-server` provides an HTTP server runtime for exposing orchestration workflows as REST APIs. Handles graceful shutdown, tracing, CORS, and compression.

## Usage

```rust
use oxify_server::{ServerRuntime, ServerConfig};
use std::net::SocketAddr;

let config = ServerConfig::new(SocketAddr::from(([127, 0, 0, 1], 3000)));
let server = ServerRuntime::new(config);
server.run().await?;
```

## Endpoints

| Path | Method | Description |
|------|--------|-------------|
| `/health` | GET | Health check |
| `/ready` | GET | Readiness check |

Additional routes can be added via `with_router()`:

```rust
use axum::{Router, routing::get};

let app = Router::new()
    .route("/workflows", get(list_workflows));

let server = ServerRuntime::new(config)
    .with_router(app);
server.run().await?;
```

## Configuration

```rust
use std::net::SocketAddr;

let config = ServerConfig::new(SocketAddr::from(([0, 0, 0, 0], 8080)))
    .with_shutdown_timeout(60);
```

## Features

| Feature | Description |
|---------|-------------|
| `compression` | Gzip/Brotli response compression |
| `cors` | Cross-Origin Resource Sharing |

## Modules

| Module | Description |
|--------|-------------|
| `server` | `ServerRuntime` — HTTP server lifecycle |
| `shutdown` | Graceful signal handling (SIGINT, SIGTERM) |
| `tracing_config` | Structured logging setup |
| `sse` | Server-Sent Events support |
| `rate_limit` | Token bucket rate limiting |
| `tls` | HTTPS/TLS certificate management |
| `error` | `AppError` / `ProblemDetails` types |
| `types` | `ServerConfig`, `ServerError` |
