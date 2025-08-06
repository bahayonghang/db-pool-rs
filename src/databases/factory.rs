use crate::core::error::{DbPoolError, Result};
use crate::core::types::{DatabaseConfig, DatabaseType};
use crate::databases::traits::DatabasePool;
use std::sync::Arc;

#[cfg(feature = "mssql")]
use crate::databases::mssql::MSSQLPool;

/// 数据库工厂
pub struct DatabaseFactory;

impl DatabaseFactory {
    /// 创建数据库连接池
    pub async fn create_pool(config: &DatabaseConfig) -> Result<Arc<dyn DatabasePool>> {
        match config.db_type {
            #[cfg(feature = "mssql")]
            DatabaseType::MSSQL => {
                let pool = MSSQLPool::new(config).await?;
                Ok(Arc::new(pool))
            }

            #[cfg(feature = "postgresql")]
            DatabaseType::PostgreSQL => {
                // TODO: 实现PostgreSQL支持
                Err(DbPoolError::Runtime("PostgreSQL支持尚未实现".to_string()))
            }

            #[cfg(feature = "redis")]
            DatabaseType::Redis => {
                // TODO: 实现Redis支持
                Err(DbPoolError::Runtime("Redis支持尚未实现".to_string()))
            }

            #[cfg(feature = "sqlite")]
            DatabaseType::SQLite => {
                // TODO: 实现SQLite支持
                Err(DbPoolError::Runtime("SQLite支持尚未实现".to_string()))
            }

            DatabaseType::InfluxDB => {
                // TODO: 实现InfluxDB支持
                Err(DbPoolError::Runtime("InfluxDB支持尚未实现".to_string()))
            }

            #[allow(unreachable_patterns)]
            _ => Err(DbPoolError::Runtime(format!(
                "不支持的数据库类型: {:?}",
                config.db_type
            ))),
        }
    }

    /// 验证数据库配置
    pub fn validate_config(config: &DatabaseConfig) -> Result<()> {
        match config.db_type {
            #[cfg(feature = "mssql")]
            DatabaseType::MSSQL => {
                MSSQLPool::validate_config(config)
            }

            #[cfg(feature = "postgresql")]
            DatabaseType::PostgreSQL => {
                // TODO: 实现PostgreSQL配置验证
                Ok(())
            }

            #[cfg(feature = "redis")]
            DatabaseType::Redis => {
                // TODO: 实现Redis配置验证
                Ok(())
            }

            #[cfg(feature = "sqlite")]
            DatabaseType::SQLite => {
                // TODO: 实现SQLite配置验证
                Ok(())
            }

            DatabaseType::InfluxDB => {
                // TODO: 实现InfluxDB配置验证
                Ok(())
            }

            #[allow(unreachable_patterns)]
            _ => Err(DbPoolError::Runtime(format!(
                "不支持的数据库类型: {:?}",
                config.db_type
            ))),
        }
    }

    /// 获取支持的数据库类型列表
    pub fn supported_databases() -> Vec<DatabaseType> {
        let mut supported = Vec::new();

        #[cfg(feature = "mssql")]
        supported.push(DatabaseType::MSSQL);

        #[cfg(feature = "postgresql")]
        supported.push(DatabaseType::PostgreSQL);

        #[cfg(feature = "redis")]
        supported.push(DatabaseType::Redis);

        #[cfg(feature = "sqlite")]
        supported.push(DatabaseType::SQLite);

        supported
    }
}