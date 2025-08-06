use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use serde::{Serialize, Deserialize};

/// 监控工具集合
pub struct MonitoringTools {
    metrics_collector: Arc<MetricsCollector>,
    health_checker: Arc<HealthChecker>,
    alert_manager: Arc<AlertManager>,
}

impl MonitoringTools {
    pub fn new() -> Self {
        Self {
            metrics_collector: Arc::new(MetricsCollector::new()),
            health_checker: Arc::new(HealthChecker::new()),
            alert_manager: Arc::new(AlertManager::new()),
        }
    }

    pub fn metrics(&self) -> Arc<MetricsCollector> {
        Arc::clone(&self.metrics_collector)
    }

    pub fn health(&self) -> Arc<HealthChecker> {
        Arc::clone(&self.health_checker)
    }

    pub fn alerts(&self) -> Arc<AlertManager> {
        Arc::clone(&self.alert_manager)
    }
}

/// 指标收集器
pub struct MetricsCollector {
    pool_metrics: RwLock<HashMap<String, PoolMetricsData>>,
    system_metrics: RwLock<SystemMetrics>,
}

#[derive(Debug, Clone)]
pub struct PoolMetricsData {
    pub pool_id: String,
    pub total_queries: u64,
    pub successful_queries: u64,
    pub failed_queries: u64,
    pub total_connections: u32,
    pub active_connections: u32,
    pub query_latencies: Vec<Duration>,
    pub created_at: Instant,
    pub last_updated: Instant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub memory_usage_bytes: u64,
    pub cpu_usage_percent: f64,
    pub open_file_descriptors: u32,
    pub network_connections: u32,
    pub uptime_seconds: u64,
    pub rust_version: String,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            pool_metrics: RwLock::new(HashMap::new()),
            system_metrics: RwLock::new(SystemMetrics::default()),
        }
    }

    /// 记录查询指标
    pub async fn record_query_metrics(
        &self,
        pool_id: &str,
        success: bool,
        latency: Duration,
    ) {
        let mut metrics = self.pool_metrics.write().await;
        let pool_metrics = metrics.entry(pool_id.to_string()).or_insert_with(|| {
            PoolMetricsData {
                pool_id: pool_id.to_string(),
                total_queries: 0,
                successful_queries: 0,
                failed_queries: 0,
                total_connections: 0,
                active_connections: 0,
                query_latencies: Vec::new(),
                created_at: Instant::now(),
                last_updated: Instant::now(),
            }
        });

        pool_metrics.total_queries += 1;
        pool_metrics.last_updated = Instant::now();
        
        if success {
            pool_metrics.successful_queries += 1;
        } else {
            pool_metrics.failed_queries += 1;
        }

        // 保留最近1000个延迟记录
        pool_metrics.query_latencies.push(latency);
        if pool_metrics.query_latencies.len() > 1000 {
            pool_metrics.query_latencies.remove(0);
        }
    }

    /// 更新连接池连接数
    pub async fn update_connection_count(&self, pool_id: &str, total: u32, active: u32) {
        let mut metrics = self.pool_metrics.write().await;
        if let Some(pool_metrics) = metrics.get_mut(pool_id) {
            pool_metrics.total_connections = total;
            pool_metrics.active_connections = active;
            pool_metrics.last_updated = Instant::now();
        }
    }

    /// 获取连接池指标
    pub async fn get_pool_metrics(&self, pool_id: &str) -> Option<PoolSummary> {
        let metrics = self.pool_metrics.read().await;
        let pool_data = metrics.get(pool_id)?;

        let uptime = pool_data.created_at.elapsed();
        let qps = if uptime.as_secs() > 0 {
            pool_data.total_queries as f64 / uptime.as_secs() as f64
        } else {
            0.0
        };

        let error_rate = if pool_data.total_queries > 0 {
            pool_data.failed_queries as f64 / pool_data.total_queries as f64
        } else {
            0.0
        };

        let (avg_latency, p99_latency) = if !pool_data.query_latencies.is_empty() {
            let total_latency: Duration = pool_data.query_latencies.iter().sum();
            let avg = total_latency / pool_data.query_latencies.len() as u32;
            
            let mut sorted_latencies = pool_data.query_latencies.clone();
            sorted_latencies.sort();
            let p99_index = (sorted_latencies.len() as f64 * 0.99) as usize;
            let p99 = sorted_latencies.get(p99_index).cloned().unwrap_or(Duration::ZERO);
            
            (avg, p99)
        } else {
            (Duration::ZERO, Duration::ZERO)
        };

        Some(PoolSummary {
            pool_id: pool_id.to_string(),
            queries_per_second: qps,
            error_rate,
            avg_latency_ms: avg_latency.as_millis() as f64,
            p99_latency_ms: p99_latency.as_millis() as f64,
            total_connections: pool_data.total_connections,
            active_connections: pool_data.active_connections,
            connection_utilization: if pool_data.total_connections > 0 {
                pool_data.active_connections as f64 / pool_data.total_connections as f64
            } else {
                0.0
            },
            uptime_seconds: uptime.as_secs(),
        })
    }

    /// 获取所有连接池指标
    pub async fn get_all_pool_metrics(&self) -> HashMap<String, PoolSummary> {
        let metrics = self.pool_metrics.read().await;
        let mut summaries = HashMap::new();

        for pool_id in metrics.keys() {
            if let Some(summary) = self.get_pool_metrics(pool_id).await {
                summaries.insert(pool_id.clone(), summary);
            }
        }

        summaries
    }

    /// 更新系统指标
    pub async fn update_system_metrics(&self) {
        let mut system_metrics = self.system_metrics.write().await;
        
        // 这里应该实现实际的系统指标收集
        // 为简化实现，使用模拟数据
        system_metrics.memory_usage_bytes = Self::get_memory_usage();
        system_metrics.cpu_usage_percent = Self::get_cpu_usage();
        system_metrics.open_file_descriptors = Self::get_open_fds();
        system_metrics.uptime_seconds = Self::get_uptime();
    }

    // 简化的系统指标收集方法
    fn get_memory_usage() -> u64 {
        // 实际实现应该读取 /proc/self/status 或使用系统调用
        50 * 1024 * 1024 // 50MB 模拟值
    }

    fn get_cpu_usage() -> f64 {
        // 实际实现应该计算CPU使用率
        15.5 // 模拟值
    }

    fn get_open_fds() -> u32 {
        // 实际实现应该读取 /proc/self/fd
        25 // 模拟值
    }

    fn get_uptime() -> u64 {
        // 实际实现应该记录启动时间
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolSummary {
    pub pool_id: String,
    pub queries_per_second: f64,
    pub error_rate: f64,
    pub avg_latency_ms: f64,
    pub p99_latency_ms: f64,
    pub total_connections: u32,
    pub active_connections: u32,
    pub connection_utilization: f64,
    pub uptime_seconds: u64,
}

impl Default for SystemMetrics {
    fn default() -> Self {
        Self {
            memory_usage_bytes: 0,
            cpu_usage_percent: 0.0,
            open_file_descriptors: 0,
            network_connections: 0,
            uptime_seconds: 0,
            rust_version: env!("CARGO_PKG_RUST_VERSION").to_string(),
        }
    }
}

/// 健康检查器
pub struct HealthChecker {
    pool_health: RwLock<HashMap<String, PoolHealth>>,
    system_health: RwLock<SystemHealth>,
}

#[derive(Debug, Clone)]
pub struct PoolHealth {
    pub pool_id: String,
    pub is_healthy: bool,
    pub last_check: Instant,
    pub consecutive_failures: u32,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SystemHealth {
    pub overall_healthy: bool,
    pub memory_healthy: bool,
    pub cpu_healthy: bool,
    pub disk_healthy: bool,
    pub network_healthy: bool,
    pub last_check: Instant,
}

impl HealthChecker {
    pub fn new() -> Self {
        Self {
            pool_health: RwLock::new(HashMap::new()),
            system_health: RwLock::new(SystemHealth::default()),
        }
    }

    /// 检查连接池健康状态
    pub async fn check_pool_health(&self, pool_id: &str) -> Result<bool, String> {
        // 这里应该实现实际的健康检查逻辑
        // 例如执行 SELECT 1 查询
        
        let is_healthy = true; // 模拟健康状态
        let error = None;

        let mut health_map = self.pool_health.write().await;
        let health = health_map.entry(pool_id.to_string()).or_insert_with(|| {
            PoolHealth {
                pool_id: pool_id.to_string(),
                is_healthy: true,
                last_check: Instant::now(),
                consecutive_failures: 0,
                last_error: None,
            }
        });

        health.is_healthy = is_healthy;
        health.last_check = Instant::now();
        health.last_error = error;

        if is_healthy {
            health.consecutive_failures = 0;
        } else {
            health.consecutive_failures += 1;
        }

        Ok(is_healthy)
    }

    /// 获取连接池健康状态
    pub async fn get_pool_health(&self, pool_id: &str) -> Option<PoolHealth> {
        let health_map = self.pool_health.read().await;
        health_map.get(pool_id).cloned()
    }

    /// 检查系统健康状态
    pub async fn check_system_health(&self) -> SystemHealth {
        let memory_healthy = true; // 实际应该检查内存使用率
        let cpu_healthy = true;    // 实际应该检查CPU使用率
        let disk_healthy = true;   // 实际应该检查磁盘空间
        let network_healthy = true; // 实际应该检查网络连通性

        let overall_healthy = memory_healthy && cpu_healthy && disk_healthy && network_healthy;

        let health = SystemHealth {
            overall_healthy,
            memory_healthy,
            cpu_healthy,
            disk_healthy,
            network_healthy,
            last_check: Instant::now(),
        };

        {
            let mut system_health = self.system_health.write().await;
            *system_health = health.clone();
        }

        health
    }
}

impl Default for SystemHealth {
    fn default() -> Self {
        Self {
            overall_healthy: true,
            memory_healthy: true,
            cpu_healthy: true,
            disk_healthy: true,
            network_healthy: true,
            last_check: Instant::now(),
        }
    }
}

/// 告警管理器
pub struct AlertManager {
    alert_rules: RwLock<Vec<AlertRule>>,
    active_alerts: RwLock<Vec<Alert>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRule {
    pub id: String,
    pub name: String,
    pub condition: AlertCondition,
    pub severity: AlertSeverity,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertCondition {
    HighErrorRate { threshold: f64 },
    HighLatency { threshold_ms: u64 },
    HighConnectionUtilization { threshold: f64 },
    PoolUnhealthy,
    SystemUnhealthy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

#[derive(Debug, Clone)]
pub struct Alert {
    pub id: String,
    pub rule_id: String,
    pub pool_id: Option<String>,
    pub message: String,
    pub severity: AlertSeverity,
    pub triggered_at: Instant,
    pub resolved_at: Option<Instant>,
}

impl AlertManager {
    pub fn new() -> Self {
        let alert_manager = Self {
            alert_rules: RwLock::new(Vec::new()),
            active_alerts: RwLock::new(Vec::new()),
        };

        alert_manager
    }

    pub async fn initialize_with_defaults(&self) {
        self.add_default_rules().await;
    }

    async fn add_default_rules(&self) {
        let default_rules = vec![
            AlertRule {
                id: "high_error_rate".to_string(),
                name: "高错误率告警".to_string(),
                condition: AlertCondition::HighErrorRate { threshold: 0.05 },
                severity: AlertSeverity::Warning,
                enabled: true,
            },
            AlertRule {
                id: "high_latency".to_string(),
                name: "高延迟告警".to_string(),
                condition: AlertCondition::HighLatency { threshold_ms: 1000 },
                severity: AlertSeverity::Warning,
                enabled: true,
            },
            AlertRule {
                id: "high_connection_utilization".to_string(),
                name: "高连接利用率告警".to_string(),
                condition: AlertCondition::HighConnectionUtilization { threshold: 0.9 },
                severity: AlertSeverity::Critical,
                enabled: true,
            },
        ];

        let mut rules = self.alert_rules.write().await;
        rules.extend(default_rules);
    }

    /// 评估告警条件
    pub async fn evaluate_alerts(&self, pool_summary: &PoolSummary) {
        let rules = self.alert_rules.read().await;
        
        for rule in rules.iter() {
            if !rule.enabled {
                continue;
            }

            let should_alert = match &rule.condition {
                AlertCondition::HighErrorRate { threshold } => {
                    pool_summary.error_rate > *threshold
                }
                AlertCondition::HighLatency { threshold_ms } => {
                    pool_summary.p99_latency_ms > *threshold_ms as f64
                }
                AlertCondition::HighConnectionUtilization { threshold } => {
                    pool_summary.connection_utilization > *threshold
                }
                _ => false,
            };

            if should_alert {
                self.trigger_alert(rule, Some(&pool_summary.pool_id)).await;
            } else {
                self.resolve_alert(&rule.id, Some(&pool_summary.pool_id)).await;
            }
        }
    }

    async fn trigger_alert(&self, rule: &AlertRule, pool_id: Option<&str>) {
        let alert_id = format!("{}_{}", rule.id, pool_id.unwrap_or("system"));
        
        // 检查是否已经存在相同的活跃告警
        {
            let alerts = self.active_alerts.read().await;
            if alerts.iter().any(|a| a.id == alert_id && a.resolved_at.is_none()) {
                return; // 告警已存在
            }
        }

        let message = match &rule.condition {
            AlertCondition::HighErrorRate { threshold } => {
                format!("连接池 {} 错误率超过 {:.1}%", pool_id.unwrap_or("unknown"), threshold * 100.0)
            }
            AlertCondition::HighLatency { threshold_ms } => {
                format!("连接池 {} P99延迟超过 {}ms", pool_id.unwrap_or("unknown"), threshold_ms)
            }
            AlertCondition::HighConnectionUtilization { threshold } => {
                format!("连接池 {} 连接利用率超过 {:.1}%", pool_id.unwrap_or("unknown"), threshold * 100.0)
            }
            _ => format!("告警规则 {} 被触发", rule.name),
        };

        let alert = Alert {
            id: alert_id,
            rule_id: rule.id.clone(),
            pool_id: pool_id.map(|s| s.to_string()),
            message,
            severity: rule.severity.clone(),
            triggered_at: Instant::now(),
            resolved_at: None,
        };

        let mut alerts = self.active_alerts.write().await;
        alerts.push(alert);

        tracing::warn!("告警触发: {}", rule.name);
    }

    async fn resolve_alert(&self, rule_id: &str, pool_id: Option<&str>) {
        let alert_id = format!("{}_{}", rule_id, pool_id.unwrap_or("system"));
        
        let mut alerts = self.active_alerts.write().await;
        if let Some(alert) = alerts.iter_mut().find(|a| a.id == alert_id && a.resolved_at.is_none()) {
            alert.resolved_at = Some(Instant::now());
            tracing::info!("告警已解决: {}", alert.message);
        }
    }

    /// 获取活跃告警
    pub async fn get_active_alerts(&self) -> Vec<Alert> {
        let alerts = self.active_alerts.read().await;
        alerts.iter().filter(|a| a.resolved_at.is_none()).cloned().collect()
    }
}