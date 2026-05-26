//! Persistence layer for API orchestration engine

pub mod db_utils;
pub mod error;
pub mod execution_store;
pub mod migration_runner;
pub mod migrations;
pub mod models;
pub mod pagination;
pub mod pool;
pub mod query_builder;
pub mod retry;
pub mod validation;
pub mod workflow_store;

// Re-export commonly used types
pub use error::{Result, StorageError, ResourceType, ResourceId};
pub use execution_store::ExecutionStore;
pub use migration_runner::{Migration, MigrationRunner, MigrationStatus};
pub use models::{WorkflowRow, ExecutionRow, UserRow};
pub use pagination::{PaginationStrategy, CursorDirection, DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE, MIN_PAGE_SIZE};
pub use pool::{DatabaseConfig, DatabasePool, PoolStats, PoolMetrics, PoolHealth};
pub use query_builder::{QueryBuilder, SortOrder, Condition, Join, JoinType};
pub use retry::{RetryConfig, RetryStats};
pub use validation::{validate_positive, validate_non_empty_string, validate_collection_size};
pub use workflow_store::{WorkflowStore, BulkOperationResult, WorkflowExport, ImportOptions, ConflictStrategy};
