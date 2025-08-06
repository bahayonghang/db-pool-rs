use crate::core::error::Result;
use crate::core::types::DatabaseConfig;
use serde::{Deserialize, Serialize};

/// MSSQL数据库特定配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MSSQLConfig {
    pub server: String,
    pub port: u16,
    pub database: String,
    pub username: String,
    pub password: String,
    pub encrypt: bool,
    pub trust_server_certificate: bool,
    pub connection_timeout: u64,
}

impl MSSQLConfig {
    pub fn from_database_config(config: &DatabaseConfig) -> Result<Self> {
        Ok(MSSQLConfig {
            server: config.host.clone(),
            port: config.port,
            database: config.database.clone(),
            username: config.username.clone(),
            password: config.password.clone(),
            encrypt: true,
            trust_server_certificate: false,
            connection_timeout: 30, // 默认30秒
        })
    }
}