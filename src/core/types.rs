use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use uuid::Uuid;

/// 数据库连接配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub db_type: DatabaseType,
    pub host: String,
    pub port: u16,
    pub database: String,
    pub username: String,
    pub password: String,
    pub pool_config: PoolConfig,
    pub ssl_config: Option<SslConfig>,
    pub timeout_config: TimeoutConfig,
    pub application_name: Option<String>,
}

/// 支持的数据库类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DatabaseType {
    MSSQL,
    PostgreSQL,
    Redis,
    SQLite,
    InfluxDB,
}

/// 连接池配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolConfig {
    pub min_connections: u32,
    pub max_connections: u32,
    pub acquire_timeout: Duration,
    pub idle_timeout: Duration,
    pub max_lifetime: Duration,
    pub auto_scaling: bool,
    pub scale_up_threshold: f32,
    pub scale_down_threshold: f32,
    pub health_check_interval: Duration,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            min_connections: 5,
            max_connections: 50,
            acquire_timeout: Duration::from_secs(30),
            idle_timeout: Duration::from_secs(600),
            max_lifetime: Duration::from_secs(3600),
            auto_scaling: true,
            scale_up_threshold: 0.8,
            scale_down_threshold: 0.3,
            health_check_interval: Duration::from_secs(60),
        }
    }
}

/// SSL配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SslConfig {
    pub ssl_mode: SslMode,
    pub trust_server_certificate: bool,
    pub certificate_path: Option<String>,
    pub key_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SslMode {
    Disable,
    Require,
    Prefer,
}

/// 超时配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeoutConfig {
    pub query_timeout: Duration,
    pub connection_timeout: Duration,
    pub command_timeout: Duration,
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            query_timeout: Duration::from_secs(30),
            connection_timeout: Duration::from_secs(10),
            command_timeout: Duration::from_secs(30),
        }
    }
}

/// 连接池状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolStatus {
    pub pool_id: String,
    pub db_type: DatabaseType,
    pub total_connections: u32,
    pub active_connections: u32,
    pub idle_connections: u32,
    pub waiting_connections: u32,
    pub is_healthy: bool,
    pub last_error: Option<String>,
    pub uptime: Duration,
}

/// 连接池指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolMetrics {
    pub pool_id: String,
    pub queries_per_second: f64,
    pub connection_utilization: f64,
    pub avg_query_time: Duration,
    pub p99_query_time: Duration,
    pub error_rate: f64,
    pub total_queries: u64,
    pub total_errors: u64,
    pub cache_hit_rate: f64,
}

/// 数据库值类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DatabaseValue {
    Null,
    Bool(bool),
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
    String(String),
    Bytes(Vec<u8>),
    DateTime(chrono::DateTime<chrono::Utc>),
    Uuid(Uuid),
}

/// 查询参数
pub type QueryParams = HashMap<String, DatabaseValue>;

/// 批处理操作
#[derive(Debug, Clone)]
pub struct BatchOperation {
    pub sql: String,
    pub params: Option<QueryParams>,
}

/// 批处理结果
#[derive(Debug, Clone)]
pub struct BatchResult {
    pub affected_rows: u64,
    pub execution_time: Duration,
    pub error: Option<String>,
}

/// 部署模式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeploymentMode {
    Simple,   // 纯Python模式
    Balanced, // Python + C扩展
    Full,     // 完整Rust模式
}

/// 故障转移策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FailoverStrategy {
    LocalOnly,
    ActiveStandby {
        primary: String,
        backup: String,
        switch_threshold: Duration,
    },
    LoadBalanced {
        pools: Vec<String>,
        algorithm: LoadBalanceAlgorithm,
    },
}

/// 负载均衡算法
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LoadBalanceAlgorithm {
    RoundRobin,
    LeastConnections,
    WeightedRoundRobin,
    Random,
}