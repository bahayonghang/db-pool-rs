use db_pool_rs::utils::monitoring::{MetricsCollector, HealthChecker, AlertManager};
use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
async fn test_metrics_collector() {
    let collector = MetricsCollector::new();
    
    // 记录一些查询指标
    collector.record_query_metrics("test_pool", true, Duration::from_millis(50)).await;
    collector.record_query_metrics("test_pool", true, Duration::from_millis(75)).await;
    collector.record_query_metrics("test_pool", false, Duration::from_millis(200)).await;
    
    // 更新连接数
    collector.update_connection_count("test_pool", 10, 3).await;
    
    // 获取指标
    let metrics = collector.get_pool_metrics("test_pool").await;
    assert!(metrics.is_some());
    
    let metrics = metrics.unwrap();
    assert_eq!(metrics.pool_id, "test_pool");
    assert_eq!(metrics.total_connections, 10);
    assert_eq!(metrics.active_connections, 3);
    assert!(metrics.queries_per_second >= 0.0);
    assert!(metrics.error_rate > 0.0); // 应该有一个失败的查询
    
    println!("✅ Metrics collector test passed");
}

#[tokio::test]
async fn test_health_checker() {
    let health_checker = HealthChecker::new();
    
    // 检查连接池健康状态
    let health_result = health_checker.check_pool_health("test_pool").await;
    assert!(health_result.is_ok());
    
    let is_healthy = health_result.unwrap();
    assert!(is_healthy); // 模拟实现总是返回健康
    
    // 获取健康状态
    let health_status = health_checker.get_pool_health("test_pool").await;
    assert!(health_status.is_some());
    
    let health_status = health_status.unwrap();
    assert_eq!(health_status.pool_id, "test_pool");
    assert!(health_status.is_healthy);
    assert_eq!(health_status.consecutive_failures, 0);
    
    // 检查系统健康状态
    let system_health = health_checker.check_system_health().await;
    assert!(system_health.overall_healthy);
    assert!(system_health.memory_healthy);
    assert!(system_health.cpu_healthy);
    assert!(system_health.disk_healthy);
    assert!(system_health.network_healthy);
    
    println!("✅ Health checker test passed");
}

#[tokio::test]
async fn test_alert_manager() {
    let alert_manager = AlertManager::new();
    
    // 等待默认规则加载
    sleep(Duration::from_millis(100)).await;
    
    // 创建一个触发告警的指标
    let pool_summary = db_pool_rs::utils::monitoring::PoolSummary {
        pool_id: "test_pool".to_string(),
        queries_per_second: 100.0,
        error_rate: 0.1, // 10% 错误率，应该触发告警
        avg_latency_ms: 50.0,
        p99_latency_ms: 150.0,
        total_connections: 10,
        active_connections: 8,
        connection_utilization: 0.8,
        uptime_seconds: 3600,
    };
    
    // 评估告警
    alert_manager.evaluate_alerts(&pool_summary).await;
    
    // 获取活跃告警
    let active_alerts = alert_manager.get_active_alerts().await;
    assert!(!active_alerts.is_empty());
    
    // 验证告警内容
    let high_error_alert = active_alerts.iter()
        .find(|alert| alert.rule_id == "high_error_rate");
    assert!(high_error_alert.is_some());
    
    let alert = high_error_alert.unwrap();
    assert_eq!(alert.pool_id, Some("test_pool".to_string()));
    assert!(alert.message.contains("错误率"));
    
    println!("✅ Alert manager test passed");
}

#[tokio::test]
async fn test_alert_resolution() {
    let alert_manager = AlertManager::new();
    
    // 等待默认规则加载
    sleep(Duration::from_millis(100)).await;
    
    // 首先触发告警
    let high_error_summary = db_pool_rs::utils::monitoring::PoolSummary {
        pool_id: "test_pool".to_string(),
        queries_per_second: 100.0,
        error_rate: 0.1, // 高错误率
        avg_latency_ms: 50.0,
        p99_latency_ms: 150.0,
        total_connections: 10,
        active_connections: 8,
        connection_utilization: 0.8,
        uptime_seconds: 3600,
    };
    
    alert_manager.evaluate_alerts(&high_error_summary).await;
    let alerts_after_trigger = alert_manager.get_active_alerts().await;
    assert!(!alerts_after_trigger.is_empty());
    
    // 然后解决告警
    let normal_summary = db_pool_rs::utils::monitoring::PoolSummary {
        pool_id: "test_pool".to_string(),
        queries_per_second: 100.0,
        error_rate: 0.01, // 正常错误率
        avg_latency_ms: 50.0,
        p99_latency_ms: 150.0,
        total_connections: 10,
        active_connections: 8,
        connection_utilization: 0.8,
        uptime_seconds: 3600,
    };
    
    alert_manager.evaluate_alerts(&normal_summary).await;
    
    // 验证告警数量变化
    let alerts_after_resolution = alert_manager.get_active_alerts().await;
    assert!(alerts_after_resolution.len() < alerts_after_trigger.len() || 
            alerts_after_resolution.is_empty());
    
    println!("✅ Alert resolution test passed");
}