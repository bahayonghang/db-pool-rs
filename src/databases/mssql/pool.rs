use crate::core::error::{DbPoolError, Result, ConnectionError};
use crate::core::types::{QueryParams, DatabaseValue, BatchOperation, BatchResult, PoolStatus, DatabaseType};
use crate::databases::traits::{DatabasePool, DatabaseConnection, DatabaseRow, TypeConverter};
use crate::databases::mssql::connection::MSSQLConnection;
use crate::databases::mssql::types::MSSQLRow;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, Semaphore};
use tiberius::{Config, Client};
use tokio::net::TcpStream;
use tokio_util::compat::{TokioAsyncWriteCompatExt, Compat};
use std::collections::VecDeque;

/// MSSQL连接池
pub struct MSSQLPool {
    config: crate::core::types::DatabaseConfig,
    connections: Arc<RwLock<VecDeque<MSSQLConnection>>>,
    semaphore: Arc<Semaphore>,
    total_connections: Arc<RwLock<u32>>,
    created_at: Instant,
}

impl MSSQLPool {
    /// 创建新的MSSQL连接池
    pub async fn new(config: &crate::core::types::DatabaseConfig) -> Result<Self> {
        Self::validate_config(config)?;

        let pool = Self {
            config: config.clone(),
            connections: Arc::new(RwLock::new(VecDeque::new())),
            semaphore: Arc::new(Semaphore::new(config.pool_config.max_connections as usize)),
            total_connections: Arc::new(RwLock::new(0)),
            created_at: Instant::now(),
        };

        // 创建最小连接数
        pool.ensure_min_connections().await?;

        Ok(pool)
    }

    /// 验证配置
    pub fn validate_config(config: &crate::core::types::DatabaseConfig) -> Result<()> {
        if config.db_type != DatabaseType::MSSQL {
            return Err(DbPoolError::Runtime("配置类型不是MSSQL".to_string()));
        }

        if config.host.is_empty() {
            return Err(DbPoolError::Runtime("MSSQL主机地址不能为空".to_string()));
        }

        if config.username.is_empty() {
            return Err(DbPoolError::Runtime("MSSQL用户名不能为空".to_string()));
        }

        Ok(())
    }

    /// 确保最小连接数
    async fn ensure_min_connections(&self) -> Result<()> {
        let current_count = {
            let connections = self.connections.read().await;
            connections.len() as u32
        };

        let min_connections = self.config.pool_config.min_connections;
        if current_count < min_connections {
            let needed = min_connections - current_count;
            for _ in 0..needed {
                let connection = self.create_connection().await?;
                {
                    let mut connections = self.connections.write().await;
                    connections.push_back(connection);
                }
                {
                    let mut total = self.total_connections.write().await;
                    *total += 1;
                }
            }
        }

        Ok(())
    }

    /// 创建新连接
    async fn create_connection(&self) -> Result<MSSQLConnection> {
        let mut tiberius_config = Config::new();
        
        tiberius_config.host(&self.config.host);
        tiberius_config.port(self.config.port);
        tiberius_config.database(&self.config.database);
        tiberius_config.authentication(tiberius::AuthMethod::sql_server(
            &self.config.username,
            &self.config.password,
        ));

        // 设置超时
        // 移除不存在的command_timeout调用
        // tiberius_config.command_timeout(self.config.timeout_config.command_timeout);

        // SSL配置
        if let Some(ssl_config) = &self.config.ssl_config {
            match ssl_config.ssl_mode {
                crate::core::types::SslMode::Disable => {
                    tiberius_config.encryption(tiberius::EncryptionLevel::NotSupported);
                }
                crate::core::types::SslMode::Require => {
                    tiberius_config.encryption(tiberius::EncryptionLevel::Required);
                }
                crate::core::types::SslMode::Prefer => {
                    tiberius_config.encryption(tiberius::EncryptionLevel::On);
                }
            }
            
            if ssl_config.trust_server_certificate {
                tiberius_config.trust_cert();
            }
        }

        // 应用程序名称
        if let Some(app_name) = &self.config.application_name {
            tiberius_config.application_name(app_name);
        }

        // 建立连接
        let tcp = tokio::time::timeout(
            self.config.timeout_config.connection_timeout,
            TcpStream::connect((self.config.host.as_str(), self.config.port))
        ).await
        .map_err(|_| ConnectionError::AcquireTimeout)?
        .map_err(|e| ConnectionError::ConnectionFailed(e.to_string()))?;

        let client = Client::connect(tiberius_config, tcp.compat_write()).await
            .map_err(|e| ConnectionError::ConnectionFailed(e.to_string()))?;

        Ok(MSSQLConnection::new(client))
    }

    /// 获取连接
    async fn get_connection(&self) -> Result<MSSQLConnection> {
        // 等待信号量
        let _permit = tokio::time::timeout(
            self.config.pool_config.acquire_timeout,
            self.semaphore.acquire()
        ).await
        .map_err(|_| ConnectionError::AcquireTimeout)?
        .map_err(|_| ConnectionError::PoolExhausted)?;

        // 尝试从池中获取连接
        {
            let mut connections = self.connections.write().await;
            if let Some(mut connection) = connections.pop_front() {
                // 检查连接是否有效
                if connection.is_valid().await {
                    return Ok(connection);
                }
            }
        }

        // 没有可用连接，创建新连接
        let total_connections = {
            let guard = self.total_connections.read().await;
            *guard
        };

        if total_connections < self.config.pool_config.max_connections {
            let connection = self.create_connection().await?;
            {
                let mut total = self.total_connections.write().await;
                *total += 1;
            }
            Ok(connection)
        } else {
            Err(ConnectionError::PoolExhausted.into())
        }
    }

    /// 归还连接
    async fn return_connection(&self, mut connection: MSSQLConnection) {
        if connection.is_valid().await {
            let mut connections = self.connections.write().await;
            connections.push_back(connection);
        } else {
            // 连接无效，减少总连接数
            let mut total = self.total_connections.write().await;
            if *total > 0 {
                *total -= 1;
            }
        }
    }
}

#[async_trait::async_trait]
impl DatabasePool for MSSQLPool {
    async fn execute_query(&self, sql: &str, params: Option<QueryParams>) -> Result<polars::frame::DataFrame> {
        let mut connection = self.get_connection().await?;
        let result = connection.query(sql, params).await;
        self.return_connection(connection).await;

        match result {
            Ok(rows) => {
                // 转换为DataFrame
                crate::databases::mssql::types::MSSQLTypeConverter::rows_to_dataframe(rows)
            }
            Err(e) => Err(e),
        }
    }

    async fn execute_non_query(&self, sql: &str, params: Option<QueryParams>) -> Result<u64> {
        let mut connection = self.get_connection().await?;
        let result = connection.execute(sql, params).await;
        self.return_connection(connection).await;
        result
    }

    async fn execute_batch(&self, operations: Vec<BatchOperation>) -> Result<Vec<BatchResult>> {
        let mut connection = self.get_connection().await?;
        let mut results = Vec::new();

        for operation in operations {
            let start_time = Instant::now();
            let result = connection.execute(&operation.sql, operation.params).await;
            let execution_time = start_time.elapsed();

            match result {
                Ok(affected_rows) => {
                    results.push(BatchResult {
                        affected_rows,
                        execution_time,
                        error: None,
                    });
                }
                Err(e) => {
                    results.push(BatchResult {
                        affected_rows: 0,
                        execution_time,
                        error: Some(e.to_string()),
                    });
                }
            }
        }

        self.return_connection(connection).await;
        Ok(results)
    }

    async fn execute_transaction(&self, operations: Vec<BatchOperation>) -> Result<Vec<BatchResult>> {
        let mut connection = self.get_connection().await?;
        let mut results = Vec::new();

        // 开始事务
        connection.begin_transaction().await?;

        let mut transaction_failed = false;

        for operation in operations {
            let start_time = Instant::now();
            let result = connection.execute(&operation.sql, operation.params).await;
            let execution_time = start_time.elapsed();

            match result {
                Ok(affected_rows) => {
                    results.push(BatchResult {
                        affected_rows,
                        execution_time,
                        error: None,
                    });
                }
                Err(e) => {
                    results.push(BatchResult {
                        affected_rows: 0,
                        execution_time,
                        error: Some(e.to_string()),
                    });
                    transaction_failed = true;
                    break;
                }
            }
        }

        // 提交或回滚事务
        if transaction_failed {
            connection.rollback_transaction().await?;
        } else {
            connection.commit_transaction().await?;
        }

        self.return_connection(connection).await;
        Ok(results)
    }

    async fn get_status(&self) -> Result<PoolStatus> {
        let connections = self.connections.read().await;
        let total = self.total_connections.read().await;
        let active = *total - connections.len() as u32;

        Ok(PoolStatus {
            pool_id: "mssql_pool".to_string(),
            db_type: DatabaseType::MSSQL,
            total_connections: *total,
            active_connections: active,
            idle_connections: connections.len() as u32,
            waiting_connections: 0, // 简化实现
            is_healthy: true,
            last_error: None,
            uptime: self.created_at.elapsed(),
        })
    }

    async fn health_check(&self) -> Result<bool> {
        // 尝试执行简单查询
        match self.execute_query("SELECT 1", None).await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    async fn close(&self) -> Result<()> {
        let mut connections = self.connections.write().await;
        
        // 关闭所有连接
        while let Some(mut connection) = connections.pop_front() {
            let _ = connection.close().await;
        }

        // 重置连接计数
        {
            let mut total = self.total_connections.write().await;
            *total = 0;
        }

        Ok(())
    }
}