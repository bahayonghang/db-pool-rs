use crate::core::error::{ConfigError, ConfigResult};
use crate::core::types::{DatabaseConfig, DatabaseType, PoolConfig, SslConfig, TimeoutConfig, SslMode};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use url::Url;

/// 配置管理器
pub struct ConfigManager {
    configs: HashMap<String, DatabaseConfig>,
}

impl ConfigManager {
    pub fn new() -> Self {
        Self {
            configs: HashMap::new(),
        }
    }

    /// 从URL创建配置
    pub fn from_url(pool_id: String, url: &str) -> ConfigResult<DatabaseConfig> {
        let parsed_url = Url::parse(url)
            .map_err(|e| ConfigError::ParseError(format!("无效的URL: {}", e)))?;

        let db_type = match parsed_url.scheme() {
            "mssql" | "sqlserver" => DatabaseType::MSSQL,
            "postgresql" | "postgres" => DatabaseType::PostgreSQL,
            "redis" => DatabaseType::Redis,
            "sqlite" => DatabaseType::SQLite,
            scheme => return Err(ConfigError::ParseError(format!("不支持的数据库类型: {}", scheme))),
        };

        let host = parsed_url.host_str()
            .unwrap_or("localhost")
            .to_string();

        let port = parsed_url.port()
            .unwrap_or(match db_type {
                DatabaseType::MSSQL => 1433,
                DatabaseType::PostgreSQL => 5432,
                DatabaseType::Redis => 6379,
                DatabaseType::SQLite => 0,
                DatabaseType::InfluxDB => 8086,
            });

        let database = parsed_url.path()
            .trim_start_matches('/')
            .to_string();

        let username = parsed_url.username().to_string();
        let password = parsed_url.password().unwrap_or("").to_string();

        // 解析查询参数
        let query_params: HashMap<String, String> = parsed_url
            .query_pairs()
            .into_owned()
            .collect();

        let ssl_config = Self::parse_ssl_config(&query_params)?;
        let pool_config = Self::parse_pool_config(&query_params)?;
        let timeout_config = Self::parse_timeout_config(&query_params)?;

        Ok(DatabaseConfig {
            db_type,
            host,
            port,
            database,
            username,
            password,
            pool_config,
            ssl_config,
            timeout_config,
            application_name: query_params.get("application_name").cloned(),
        })
    }

    /// 从字典创建配置
    pub fn from_dict(config_dict: HashMap<String, String>) -> ConfigResult<DatabaseConfig> {
        let db_type_str = config_dict.get("db_type")
            .ok_or_else(|| ConfigError::MissingRequired("db_type".to_string()))?;

        let db_type = match db_type_str.as_str() {
            "mssql" => DatabaseType::MSSQL,
            "postgresql" => DatabaseType::PostgreSQL,
            "redis" => DatabaseType::Redis,
            "sqlite" => DatabaseType::SQLite,
            "influxdb" => DatabaseType::InfluxDB,
            _ => return Err(ConfigError::InvalidValue(format!("无效的数据库类型: {}", db_type_str))),
        };

        let host = config_dict.get("host")
            .cloned()
            .unwrap_or_else(|| "localhost".to_string());

        let port = config_dict.get("port")
            .and_then(|p| p.parse().ok())
            .unwrap_or(match db_type {
                DatabaseType::MSSQL => 1433,
                DatabaseType::PostgreSQL => 5432,
                DatabaseType::Redis => 6379,
                DatabaseType::SQLite => 0,
                DatabaseType::InfluxDB => 8086,
            });

        let database = config_dict.get("database")
            .cloned()
            .unwrap_or_else(|| "default".to_string());

        let username = config_dict.get("username")
            .cloned()
            .unwrap_or_default();

        let password = config_dict.get("password")
            .cloned()
            .unwrap_or_default();

        Ok(DatabaseConfig {
            db_type,
            host,
            port,
            database,
            username,
            password,
            pool_config: PoolConfig::default(),
            ssl_config: None,
            timeout_config: TimeoutConfig::default(),
            application_name: config_dict.get("application_name").cloned(),
        })
    }

    /// 从环境变量创建配置
    pub fn from_env(prefix: &str) -> ConfigResult<DatabaseConfig> {
        let mut config_dict = HashMap::new();

        // 读取环境变量
        for (key, value) in std::env::vars() {
            if key.starts_with(prefix) {
                let config_key = key.strip_prefix(prefix)
                    .unwrap()
                    .to_lowercase();
                config_dict.insert(config_key, value);
            }
        }

        Self::from_dict(config_dict)
    }

    /// 验证配置
    pub fn validate_config(config: &DatabaseConfig) -> ConfigResult<()> {
        if config.host.is_empty() {
            return Err(ConfigError::ValidationFailed("主机地址不能为空".to_string()));
        }

        if config.port == 0 && config.db_type != DatabaseType::SQLite {
            return Err(ConfigError::ValidationFailed("端口号无效".to_string()));
        }

        if config.database.is_empty() && config.db_type != DatabaseType::Redis {
            return Err(ConfigError::ValidationFailed("数据库名称不能为空".to_string()));
        }

        if config.pool_config.min_connections > config.pool_config.max_connections {
            return Err(ConfigError::ValidationFailed("最小连接数不能大于最大连接数".to_string()));
        }

        if config.pool_config.max_connections == 0 {
            return Err(ConfigError::ValidationFailed("最大连接数必须大于0".to_string()));
        }

        Ok(())
    }

    /// 添加配置
    pub fn add_config(&mut self, pool_id: String, config: DatabaseConfig) -> ConfigResult<()> {
        Self::validate_config(&config)?;
        self.configs.insert(pool_id, config);
        Ok(())
    }

    /// 获取配置
    pub fn get_config(&self, pool_id: &str) -> Option<&DatabaseConfig> {
        self.configs.get(pool_id)
    }

    /// 移除配置
    pub fn remove_config(&mut self, pool_id: &str) -> Option<DatabaseConfig> {
        self.configs.remove(pool_id)
    }

    /// 列出所有配置
    pub fn list_configs(&self) -> Vec<String> {
        self.configs.keys().cloned().collect()
    }

    // 私有辅助方法
    fn parse_ssl_config(params: &HashMap<String, String>) -> ConfigResult<Option<SslConfig>> {
        if let Some(ssl_mode_str) = params.get("ssl_mode") {
            let ssl_mode = match ssl_mode_str.as_str() {
                "disable" => SslMode::Disable,
                "require" => SslMode::Require,
                "prefer" => SslMode::Prefer,
                _ => return Err(ConfigError::InvalidValue(format!("无效的SSL模式: {}", ssl_mode_str))),
            };

            let trust_server_certificate = params
                .get("trust_server_certificate")
                .and_then(|v| v.parse().ok())
                .unwrap_or(false);

            Ok(Some(SslConfig {
                ssl_mode,
                trust_server_certificate,
                certificate_path: params.get("certificate_path").cloned(),
                key_path: params.get("key_path").cloned(),
            }))
        } else {
            Ok(None)
        }
    }

    fn parse_pool_config(params: &HashMap<String, String>) -> ConfigResult<PoolConfig> {
        let mut config = PoolConfig::default();

        if let Some(min_conn) = params.get("min_connections") {
            config.min_connections = min_conn.parse()
                .map_err(|_| ConfigError::InvalidValue("min_connections必须是数字".to_string()))?;
        }

        if let Some(max_conn) = params.get("max_connections") {
            config.max_connections = max_conn.parse()
                .map_err(|_| ConfigError::InvalidValue("max_connections必须是数字".to_string()))?;
        }

        if let Some(timeout) = params.get("acquire_timeout") {
            let secs: u64 = timeout.parse()
                .map_err(|_| ConfigError::InvalidValue("acquire_timeout必须是数字".to_string()))?;
            config.acquire_timeout = Duration::from_secs(secs);
        }

        Ok(config)
    }

    fn parse_timeout_config(params: &HashMap<String, String>) -> ConfigResult<TimeoutConfig> {
        let mut config = TimeoutConfig::default();

        if let Some(timeout) = params.get("query_timeout") {
            let secs: u64 = timeout.parse()
                .map_err(|_| ConfigError::InvalidValue("query_timeout必须是数字".to_string()))?;
            config.query_timeout = Duration::from_secs(secs);
        }

        if let Some(timeout) = params.get("connection_timeout") {
            let secs: u64 = timeout.parse()
                .map_err(|_| ConfigError::InvalidValue("connection_timeout必须是数字".to_string()))?;
            config.connection_timeout = Duration::from_secs(secs);
        }

        Ok(config)
    }
}

impl Default for ConfigManager {
    fn default() -> Self {
        Self::new()
    }
}