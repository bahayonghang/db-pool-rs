use thiserror::Error;

/// db-pool-rs 错误类型
#[derive(Error, Debug)]
pub enum DbPoolError {
    #[error("连接错误: {0}")]
    Connection(#[from] ConnectionError),

    #[error("查询错误: {0}")]
    Query(#[from] QueryError),

    #[error("配置错误: {0}")]
    Config(#[from] ConfigError),

    #[error("数据转换错误: {0}")]
    DataConversion(#[from] ConversionError),

    #[error("监控错误: {0}")]
    Monitoring(String),

    #[error("运行时错误: {0}")]
    Runtime(String),
}

/// 连接相关错误
#[derive(Error, Debug)]
pub enum ConnectionError {
    #[error("连接池已满")]
    PoolExhausted,

    #[error("获取连接超时")]
    AcquireTimeout,

    #[error("连接失败: {0}")]
    ConnectionFailed(String),

    #[error("连接已关闭")]
    ConnectionClosed,

    #[error("连接健康检查失败")]
    HealthCheckFailed,
}

/// 查询相关错误
#[derive(Error, Debug)]
pub enum QueryError {
    #[error("SQL语法错误: {0}")]
    SyntaxError(String),

    #[error("查询执行失败: {0}")]
    ExecutionFailed(String),

    #[error("查询超时")]
    Timeout,

    #[error("参数绑定错误: {0}")]
    ParameterBinding(String),

    #[error("结果集处理错误: {0}")]
    ResultProcessing(String),
}

/// 配置相关错误
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("配置解析错误: {0}")]
    ParseError(String),

    #[error("配置验证失败: {0}")]
    ValidationFailed(String),

    #[error("必需配置缺失: {0}")]
    MissingRequired(String),

    #[error("配置值无效: {0}")]
    InvalidValue(String),
}

/// 数据转换错误
#[derive(Error, Debug)]
pub enum ConversionError {
    #[error("类型转换失败: {0}")]
    TypeConversion(String),

    #[error("DataFrame转换失败: {0}")]
    DataFrameConversion(String),

    #[error("序列化失败: {0}")]
    Serialization(String),

    #[error("反序列化失败: {0}")]
    Deserialization(String),
}

/// 用于PyO3的错误转换
impl From<DbPoolError> for pyo3::PyErr {
    fn from(err: DbPoolError) -> Self {
        pyo3::exceptions::PyRuntimeError::new_err(err.to_string())
    }
}

/// 结果类型别名
pub type Result<T> = std::result::Result<T, DbPoolError>;
pub type ConnectionResult<T> = std::result::Result<T, ConnectionError>;
pub type QueryResult<T> = std::result::Result<T, QueryError>;
pub type ConfigResult<T> = std::result::Result<T, ConfigError>;
pub type ConversionResult<T> = std::result::Result<T, ConversionError>;