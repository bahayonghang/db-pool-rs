use crate::core::error::Result;
use crate::core::types::{QueryParams, BatchOperation, BatchResult, PoolStatus, DatabaseValue};
use async_trait::async_trait;
use polars::frame::DataFrame;
use std::collections::HashMap;

/// 数据库连接池特征
#[async_trait]
pub trait DatabasePool: Send + Sync {
    /// 执行查询并返回DataFrame
    async fn execute_query(&self, sql: &str, params: Option<QueryParams>) -> Result<DataFrame>;

    /// 执行非查询操作（INSERT, UPDATE, DELETE等）
    async fn execute_non_query(&self, sql: &str, params: Option<QueryParams>) -> Result<u64>;

    /// 批量执行操作
    async fn execute_batch(&self, operations: Vec<BatchOperation>) -> Result<Vec<BatchResult>>;

    /// 执行事务
    async fn execute_transaction(&self, operations: Vec<BatchOperation>) -> Result<Vec<BatchResult>>;

    /// 获取连接池状态
    async fn get_status(&self) -> Result<PoolStatus>;

    /// 健康检查
    async fn health_check(&self) -> Result<bool>;

    /// 关闭连接池
    async fn close(&self) -> Result<()>;
}

/// 数据库行特征
pub trait DatabaseRow: Send + Sync {
    /// 获取列数
    fn column_count(&self) -> usize;

    /// 获取列名
    fn column_names(&self) -> Vec<String>;

    /// 获取指定列的值
    fn get_value(&self, index: usize) -> Option<DatabaseValue>;

    /// 根据列名获取值
    fn get_value_by_name(&self, name: &str) -> Option<DatabaseValue>;

    /// 转换为HashMap
    fn to_map(&self) -> HashMap<String, DatabaseValue>;
}

/// 数据库连接特征
#[async_trait]
pub trait DatabaseConnection: Send + Sync {
    type Row: DatabaseRow;

    /// 执行查询
    async fn query(&mut self, sql: &str, params: Option<QueryParams>) -> Result<Vec<Self::Row>>;

    /// 执行非查询操作
    async fn execute(&mut self, sql: &str, params: Option<QueryParams>) -> Result<u64>;

    /// 开始事务
    async fn begin_transaction(&mut self) -> Result<()>;

    /// 提交事务
    async fn commit_transaction(&mut self) -> Result<()>;

    /// 回滚事务
    async fn rollback_transaction(&mut self) -> Result<()>;

    /// 检查连接是否有效
    async fn is_valid(&mut self) -> bool;

    /// 关闭连接
    async fn close(&mut self) -> Result<()>;
}

/// 数据库类型转换特征
pub trait TypeConverter: Send + Sync {
    /// 将数据库行转换为DataFrame
    fn rows_to_dataframe<R: DatabaseRow>(rows: Vec<R>) -> Result<DataFrame>;

    /// 将DatabaseValue转换为Polars的AnyValue
    fn database_value_to_any_value(value: DatabaseValue) -> polars::prelude::AnyValue<'static>;

    /// 将参数映射转换为数据库特定的参数
    fn convert_params(params: &QueryParams) -> Result<Vec<(String, DatabaseValue)>>;
}

/// 连接池工厂特征
#[async_trait]
pub trait PoolFactory: Send + Sync {
    type Pool: DatabasePool;

    /// 创建连接池
    async fn create_pool(config: &crate::core::types::DatabaseConfig) -> Result<Self::Pool>;

    /// 验证配置
    fn validate_config(config: &crate::core::types::DatabaseConfig) -> Result<()>;
}