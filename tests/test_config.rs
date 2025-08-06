use db_pool_rs::core::config::ConfigManager;
use db_pool_rs::core::types::{DatabaseConfig, DatabaseType, PoolConfig, TimeoutConfig};
use std::collections::HashMap;
use std::time::Duration;

#[tokio::test]
async fn test_config_from_url() {
    let url = "mssql://testuser:testpass@localhost:1433/testdb?min_connections=10&max_connections=20";
    
    let config = ConfigManager::from_url("test_pool".to_string(), url).unwrap();
    
    assert_eq!(config.db_type, DatabaseType::MSSQL);
    assert_eq!(config.host, "localhost");
    assert_eq!(config.port, 1433);
    assert_eq!(config.database, "testdb");
    assert_eq!(config.username, "testuser");
    assert_eq!(config.password, "testpass");
}

#[tokio::test]
async fn test_config_from_dict() {
    let mut config_dict = HashMap::new();
    config_dict.insert("db_type".to_string(), "mssql".to_string());
    config_dict.insert("host".to_string(), "localhost".to_string());
    config_dict.insert("port".to_string(), "1433".to_string());
    config_dict.insert("database".to_string(), "testdb".to_string());
    config_dict.insert("username".to_string(), "testuser".to_string());
    config_dict.insert("password".to_string(), "testpass".to_string());
    
    let config = ConfigManager::from_dict(config_dict).unwrap();
    
    assert_eq!(config.db_type, DatabaseType::MSSQL);
    assert_eq!(config.host, "localhost");
    assert_eq!(config.port, 1433);
    assert_eq!(config.database, "testdb");
    assert_eq!(config.username, "testuser");
    assert_eq!(config.password, "testpass");
}

#[tokio::test]
async fn test_config_validation() {
    let config = DatabaseConfig {
        db_type: DatabaseType::MSSQL,
        host: "".to_string(), // 空主机名应该验证失败
        port: 1433,
        database: "testdb".to_string(),
        username: "testuser".to_string(),
        password: "testpass".to_string(),
        pool_config: PoolConfig::default(),
        ssl_config: None,
        timeout_config: TimeoutConfig::default(),
        application_name: None,
    };
    
    let result = ConfigManager::validate_config(&config);
    assert!(result.is_err());
}

#[tokio::test]
async fn test_config_manager() {
    let mut manager = ConfigManager::new();
    
    let config = DatabaseConfig {
        db_type: DatabaseType::MSSQL,
        host: "localhost".to_string(),
        port: 1433,
        database: "testdb".to_string(),
        username: "testuser".to_string(),
        password: "testpass".to_string(),
        pool_config: PoolConfig::default(),
        ssl_config: None,
        timeout_config: TimeoutConfig::default(),
        application_name: None,
    };
    
    // 添加配置
    manager.add_config("test_pool".to_string(), config.clone()).unwrap();
    
    // 获取配置
    let retrieved_config = manager.get_config("test_pool").unwrap();
    assert_eq!(retrieved_config.host, "localhost");
    
    // 列出配置
    let pool_ids = manager.list_configs();
    assert_eq!(pool_ids.len(), 1);
    assert_eq!(pool_ids[0], "test_pool");
    
    // 移除配置
    let removed_config = manager.remove_config("test_pool").unwrap();
    assert_eq!(removed_config.host, "localhost");
    
    // 验证已移除
    assert!(manager.get_config("test_pool").is_none());
}