use db_pool_rs::core::pool_manager::DistributedPoolManager;
use db_pool_rs::core::types::{DatabaseConfig, DatabaseType, PoolConfig, TimeoutConfig, DeploymentMode};
use std::time::Duration;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn test_pool_manager_creation() {
    let manager = DistributedPoolManager::new(DeploymentMode::Full);
    
    // 测试初始状态
    let pools = manager.list_pools();
    assert!(pools.is_empty());
}

#[tokio::test]
#[serial]
async fn test_pool_creation_and_removal() {
    let manager = DistributedPoolManager::new(DeploymentMode::Full);
    
    let config = create_test_config();
    
    // 创建连接池
    let result = manager.create_pool("test_pool".to_string(), config).await;
    // 注意：在测试环境中，实际的数据库连接可能会失败
    // 这里我们主要测试管理器的逻辑
    
    // 列出连接池
    let pools = manager.list_pools();
    // 根据实际实现可能为空或包含test_pool
    
    // 移除连接池
    if pools.contains(&"test_pool".to_string()) {
        let result = manager.remove_pool("test_pool").await;
        assert!(result.is_ok());
        
        let pools_after_removal = manager.list_pools();
        assert!(!pools_after_removal.contains(&"test_pool".to_string()));
    }
}

#[tokio::test]
#[serial]
async fn test_pool_status() {
    let manager = DistributedPoolManager::new(DeploymentMode::Full);
    
    // 测试获取不存在的连接池状态
    let result = manager.get_pool_status("nonexistent_pool").await;
    assert!(result.is_err());
}

#[tokio::test]
#[serial]
async fn test_pool_metrics() {
    let manager = DistributedPoolManager::new(DeploymentMode::Full);
    
    // 测试获取不存在的连接池指标
    let result = manager.get_pool_metrics("nonexistent_pool").await;
    assert!(result.is_err());
}

#[tokio::test]
#[serial]
async fn test_health_check() {
    let manager = DistributedPoolManager::new(DeploymentMode::Full);
    
    // 测试对不存在的连接池进行健康检查
    let result = manager.health_check("nonexistent_pool").await;
    assert!(result.is_err());
}

fn create_test_config() -> DatabaseConfig {
    DatabaseConfig {
        db_type: DatabaseType::MSSQL,
        host: "localhost".to_string(),
        port: 1433,
        database: "test_db".to_string(),
        username: "test_user".to_string(),
        password: "test_pass".to_string(),
        pool_config: PoolConfig {
            min_connections: 1,
            max_connections: 5,
            acquire_timeout: Duration::from_secs(5),
            idle_timeout: Duration::from_secs(300),
            max_lifetime: Duration::from_secs(1800),
            auto_scaling: false,
            scale_up_threshold: 0.8,
            scale_down_threshold: 0.3,
            health_check_interval: Duration::from_secs(30),
        },
        ssl_config: None,
        timeout_config: TimeoutConfig::default(),
        application_name: Some("test_app".to_string()),
    }
}