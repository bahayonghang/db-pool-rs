# ADR-002: 分布式连接池架构设计

## 状态
已接受

## 背景

原始设计采用全局 PoolManager 管理所有连接池，存在以下问题：
- **单点故障风险**：全局管理器故障影响所有连接池
- **性能瓶颈**：所有请求都要通过同一个管理器
- **扩展性限制**：难以支持多实例部署
- **资源竞争**：多个连接池竞争管理器资源

## 决策

采用分布式连接池架构：

```rust
pub struct DistributedPoolManager {
    // 本地连接池实例
    local_pools: HashMap<String, LocalPool>,
    // 可选的协调器（用于集群模式）
    coordinator: Option<Arc<PoolCoordinator>>,
    // 服务发现机制
    discovery: ServiceDiscovery,
    // 故障转移策略
    fallback_strategy: FailoverStrategy,
}

pub enum DeploymentMode {
    Standalone,      // 单机模式
    Coordinated,     // 协调模式
    FullDistributed, // 完全分布式
}
```

## 后果

### 正面影响
- ✅ **消除单点故障**：每个连接池独立运行
- ✅ **提升性能**：减少中心化协调开销
- ✅ **支持水平扩展**：可部署多个实例
- ✅ **提高可用性**：单个连接池故障不影响其他
- ✅ **灵活部署**：支持多种部署模式

### 负面影响
- ❌ **架构复杂性**：需要处理分布式协调问题
- ❌ **一致性挑战**：多实例间状态同步困难
- ❌ **监控复杂**：需要聚合多个实例的指标
- ❌ **配置管理**：分布式配置同步问题

## 实施

### 阶段一：本地池重构
```rust
pub struct LocalPool {
    id: String,
    connection_pool: Box<dyn DatabasePool>,
    metrics: LocalMetrics,
    health_checker: HealthChecker,
}

impl LocalPool {
    pub async fn execute_query(&self, sql: &str) -> Result<DataFrame> {
        // 本地执行，无需全局协调
    }
    
    pub fn get_status(&self) -> PoolStatus {
        // 本地状态，实时返回
    }
}
```

### 阶段二：协调器实现
```rust
pub struct PoolCoordinator {
    registry: Arc<ServiceRegistry>,
    load_balancer: LoadBalancer,
    health_monitor: HealthMonitor,
}

impl PoolCoordinator {
    pub async fn discover_pools(&self) -> Vec<PoolInstance> {
        // 服务发现逻辑
    }
    
    pub async fn route_request(&self, request: PoolRequest) -> Result<PoolResponse> {
        // 请求路由逻辑
    }
}
```

### 阶段三：故障转移机制
```rust
pub enum FailoverStrategy {
    LocalOnly {
        // 仅使用本地连接池
    },
    ActiveStandby {
        primary: String,
        backup: String,
        switch_threshold: Duration,
    },
    LoadBalanced {
        pools: Vec<String>,
        algorithm: LoadBalanceAlgorithm,
    },
}
```

## 风险缓解

### 风险 1：分布式复杂性
**缓解措施**：
- 提供简化的单机模式作为默认配置
- 渐进式启用分布式特性
- 完善的本地开发支持

### 风险 2：网络分区问题
**缓解措施**：
```rust
pub struct NetworkPartitionHandler {
    detection_interval: Duration,
    recovery_strategy: RecoveryStrategy,
    local_cache_ttl: Duration,
}

impl NetworkPartitionHandler {
    pub async fn handle_partition(&self) -> FallbackAction {
        // 网络分区处理逻辑
        FallbackAction::SwitchToLocalMode
    }
}
```

### 风险 3：配置一致性
**缓解措施**：
- 配置版本控制
- 配置验证机制
- 回滚策略

### 风险 4：监控聚合困难
**缓解措施**：
```rust
pub struct DistributedMetrics {
    local_collector: MetricsCollector,
    aggregator: Option<MetricsAggregator>,
    export_interval: Duration,
}
```

## 部署模式

### 单机模式 (Standalone)
```yaml
deployment:
  mode: standalone
  pools:
    - id: main_db
      type: mssql
      config: {...}
```

### 协调模式 (Coordinated)
```yaml
deployment:
  mode: coordinated
  coordinator:
    endpoint: "http://coordinator:8080"
  pools:
    - id: main_db
      type: mssql
      config: {...}
```

### 完全分布式 (FullDistributed)
```yaml
deployment:
  mode: distributed
  discovery:
    type: consul
    endpoint: "consul:8500"
  load_balancer:
    algorithm: round_robin
```

## 监控指标

- **连接池分布**：各节点连接池数量和状态
- **请求路由**：路由成功率和延迟
- **故障转移**：切换频率和恢复时间
- **网络分区**：检测和恢复时间
- **配置同步**：同步成功率和延迟

## 向后兼容

保持原有 API 兼容性：
```python
# 原有 API 继续支持
pool = DatabasePool()  # 自动使用单机模式

# 新的分布式 API
distributed_pool = DistributedDatabasePool(
    mode=DeploymentMode.COORDINATED,
    coordinator_url="http://coordinator:8080"
)
```