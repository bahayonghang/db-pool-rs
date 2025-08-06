# 运维和监控指南

本指南提供 db-pool-rs 生产环境的运维和监控最佳实践。

## 📊 监控体系

### 核心指标监控

#### 1. 连接池指标
```python
# 获取连接池详细指标
from db_pool_rs import DatabasePool

pool = DatabasePool()
metrics = await pool.get_detailed_metrics("pool_id")

# 关键指标
print(f"连接池利用率: {metrics.connection_utilization:.2%}")
print(f"活跃连接数: {metrics.active_connections}")
print(f"等待队列长度: {metrics.waiting_queue_length}")
print(f"连接获取延迟 P99: {metrics.acquire_latency_p99}ms")
```

#### 2. 查询性能指标
```python
# 查询性能统计
query_metrics = await pool.get_query_metrics("pool_id")

print(f"QPS: {query_metrics.queries_per_second}")
print(f"查询延迟 P50: {query_metrics.latency_p50}ms")
print(f"查询延迟 P99: {query_metrics.latency_p99}ms")
print(f"错误率: {query_metrics.error_rate:.2%}")
print(f"慢查询数: {query_metrics.slow_queries_count}")
```

### 监控集成

#### Prometheus 集成
```python
# 启用 Prometheus 指标导出
from db_pool_rs.monitoring import PrometheusExporter

exporter = PrometheusExporter(
    port=9090,
    metrics_prefix="db_pool_rs",
    export_interval=30  # 秒
)

pool = DatabasePool(
    monitoring_exporters=[exporter]
)
```

#### Grafana 仪表板配置
```json
{
  "dashboard": {
    "title": "DB-Pool-RS 监控",
    "panels": [
      {
        "title": "连接池状态",
        "type": "stat",
        "targets": [
          {
            "expr": "db_pool_rs_connection_pool_active_connections",
            "legendFormat": "活跃连接"
          }
        ]
      },
      {
        "title": "查询性能",
        "type": "graph",
        "targets": [
          {
            "expr": "rate(db_pool_rs_queries_total[5m])",
            "legendFormat": "QPS"
          }
        ]
      }
    ]
  }
}
```

## 🚨 告警配置

### 告警规则

#### 1. 连接池告警
```yaml
# prometheus_alerts.yml
groups:
  - name: db_pool_rs
    rules:
      - alert: HighConnectionUtilization
        expr: db_pool_rs_connection_utilization > 0.8
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "连接池利用率过高"
          description: "连接池 {{ $labels.pool_id }} 利用率达到 {{ $value }}%"

      - alert: HighQueryLatency
        expr: db_pool_rs_query_latency_p99 > 1000
        for: 2m
        labels:
          severity: critical
        annotations:
          summary: "查询延迟过高"
          description: "连接池 {{ $labels.pool_id }} P99延迟达到 {{ $value }}ms"

      - alert: HighErrorRate
        expr: rate(db_pool_rs_query_errors_total[5m]) / rate(db_pool_rs_queries_total[5m]) > 0.05
        for: 3m
        labels:
          severity: critical
        annotations:
          summary: "查询错误率过高"
          description: "连接池 {{ $labels.pool_id }} 错误率达到 {{ $value }}%"
```

#### 2. 系统级告警
```python
# 自定义告警处理
from db_pool_rs.monitoring import AlertHandler

class CustomAlertHandler(AlertHandler):
    async def handle_high_latency(self, pool_id: str, latency: float):
        """处理高延迟告警"""
        if latency > 2000:  # 2秒
            # 发送紧急通知
            await self.send_alert(
                level="critical",
                message=f"连接池 {pool_id} 延迟达到 {latency}ms",
                channels=["slack", "email"]
            )
            # 自动扩容
            await self.auto_scale_pool(pool_id)

    async def handle_connection_exhaustion(self, pool_id: str):
        """处理连接耗尽"""
        # 临时增加连接数
        await self.emergency_scale_up(pool_id)
        # 发送告警
        await self.send_alert(
            level="critical", 
            message=f"连接池 {pool_id} 连接耗尽",
            channels=["pagerduty"]
        )

# 注册告警处理器
pool = DatabasePool(
    alert_handlers=[CustomAlertHandler()]
)
```

## 🔧 运维操作

### 健康检查

#### 1. 基础健康检查
```python
async def health_check():
    """基础健康检查"""
    pool = DatabasePool()
    
    # 检查所有连接池
    pools = await pool.list_pools()
    healthy_pools = []
    unhealthy_pools = []
    
    for pool_id in pools:
        try:
            # 执行简单查询验证连通性
            await pool.query(pool_id, "SELECT 1")
            healthy_pools.append(pool_id)
        except Exception as e:
            unhealthy_pools.append((pool_id, str(e)))
    
    return {
        "status": "healthy" if not unhealthy_pools else "degraded",
        "healthy_pools": healthy_pools,
        "unhealthy_pools": unhealthy_pools,
        "timestamp": datetime.utcnow().isoformat()
    }
```

#### 2. 深度健康检查
```python
async def deep_health_check(pool_id: str):
    """深度健康检查"""
    pool = DatabasePool()
    
    checks = {
        "connectivity": False,
        "performance": False,
        "resource_usage": False,
        "error_rate": False
    }
    
    # 连通性检查
    try:
        await pool.query(pool_id, "SELECT 1")
        checks["connectivity"] = True
    except Exception:
        pass
    
    # 性能检查
    start_time = time.time()
    try:
        await pool.query(pool_id, "SELECT 1")
        latency = (time.time() - start_time) * 1000
        checks["performance"] = latency < 100  # 100ms 阈值
    except Exception:
        pass
    
    # 资源使用检查
    metrics = await pool.get_metrics(pool_id)
    checks["resource_usage"] = metrics.connection_utilization < 0.9
    
    # 错误率检查
    checks["error_rate"] = metrics.error_rate < 0.01
    
    return {
        "pool_id": pool_id,
        "overall_healthy": all(checks.values()),
        "checks": checks,
        "metrics": metrics.to_dict()
    }
```

### 故障恢复

#### 1. 自动故障恢复
```python
class AutoRecoveryHandler:
    """自动故障恢复处理器"""
    
    async def handle_connection_failure(self, pool_id: str, error: Exception):
        """处理连接故障"""
        print(f"检测到连接故障: {pool_id} - {error}")
        
        # 步骤1: 重试连接
        for attempt in range(3):
            try:
                await self.recreate_pool(pool_id)
                print(f"连接池 {pool_id} 重建成功")
                return
            except Exception as e:
                print(f"重试 {attempt + 1} 失败: {e}")
                await asyncio.sleep(2 ** attempt)  # 指数退避
        
        # 步骤2: 切换到备用连接
        backup_pool = await self.get_backup_pool(pool_id)
        if backup_pool:
            await self.switch_to_backup(pool_id, backup_pool)
            print(f"已切换到备用连接池: {backup_pool}")
        
        # 步骤3: 发送告警
        await self.send_failure_alert(pool_id, error)
    
    async def handle_performance_degradation(self, pool_id: str, metrics):
        """处理性能下降"""
        if metrics.latency_p99 > 1000:  # 1秒
            # 增加连接数
            await self.scale_up_connections(pool_id)
        
        if metrics.connection_utilization > 0.9:
            # 清理空闲连接
            await self.cleanup_idle_connections(pool_id)
            # 增加最大连接数
            await self.increase_max_connections(pool_id)
```

#### 2. 手动故障恢复操作
```bash
#!/bin/bash
# scripts/recovery_operations.sh

# 重启特定连接池
restart_pool() {
    local pool_id=$1
    echo "重启连接池: $pool_id"
    python -c "
import asyncio
from db_pool_rs import DatabasePool

async def restart():
    pool = DatabasePool()
    await pool.remove_pool('$pool_id')
    # 从配置重新创建
    await pool.create_pool_from_config('$pool_id')
    print('连接池重启完成')

asyncio.run(restart())
"
}

# 清理连接池状态
cleanup_pool() {
    local pool_id=$1
    echo "清理连接池状态: $pool_id"
    python -c "
import asyncio
from db_pool_rs import DatabasePool

async def cleanup():
    pool = DatabasePool()
    await pool.cleanup_pool('$pool_id')
    print('连接池清理完成')

asyncio.run(cleanup())
"
}

# 导出连接池指标
export_metrics() {
    local pool_id=$1
    local output_file=${2:-"metrics_$(date +%Y%m%d_%H%M%S).json"}
    
    echo "导出连接池指标: $pool_id -> $output_file"
    python -c "
import asyncio
import json
from db_pool_rs import DatabasePool

async def export():
    pool = DatabasePool()
    metrics = await pool.get_detailed_metrics('$pool_id')
    with open('$output_file', 'w') as f:
        json.dump(metrics.to_dict(), f, indent=2)
    print('指标导出完成')

asyncio.run(export())
"
}
```

## 📈 性能调优

### 连接池调优

#### 1. 连接数优化
```python
# 动态调整连接池大小
async def optimize_pool_size(pool_id: str):
    """根据负载动态调整连接池大小"""
    pool = DatabasePool()
    metrics = await pool.get_metrics(pool_id)
    
    # 获取最近的负载数据
    recent_qps = metrics.queries_per_second
    recent_utilization = metrics.connection_utilization
    
    current_config = await pool.get_pool_config(pool_id)
    
    # 调优规则
    if recent_utilization > 0.8 and recent_qps > 1000:
        # 高负载，增加连接数
        new_max = min(current_config.max_connections * 1.5, 200)
        await pool.update_pool_config(pool_id, {
            "max_connections": new_max
        })
        
    elif recent_utilization < 0.3 and recent_qps < 100:
        # 低负载，减少连接数
        new_max = max(current_config.max_connections * 0.8, 10)
        await pool.update_pool_config(pool_id, {
            "max_connections": new_max
        })
```

#### 2. 超时配置优化
```python
# 智能超时配置
async def optimize_timeouts(pool_id: str):
    """根据网络延迟调整超时配置"""
    pool = DatabasePool()
    
    # 测量网络延迟
    latencies = []
    for _ in range(10):
        start = time.time()
        try:
            await pool.query(pool_id, "SELECT 1")
            latencies.append((time.time() - start) * 1000)
        except Exception:
            continue
    
    if latencies:
        avg_latency = sum(latencies) / len(latencies)
        p95_latency = sorted(latencies)[int(len(latencies) * 0.95)]
        
        # 设置超时为 P95 延迟的 3 倍
        recommended_timeout = max(p95_latency * 3, 1000)  # 最少1秒
        
        await pool.update_pool_config(pool_id, {
            "query_timeout": recommended_timeout,
            "acquire_timeout": recommended_timeout * 0.5
        })
```

## 🔍 故障排除

### 常见问题诊断

#### 1. 连接泄漏诊断
```python
async def diagnose_connection_leak(pool_id: str):
    """诊断连接泄漏问题"""
    pool = DatabasePool()
    
    # 获取详细连接信息
    connection_info = await pool.get_connection_details(pool_id)
    
    suspicious_connections = []
    for conn in connection_info:
        # 检查长时间活跃的连接
        if conn.active_duration > 300:  # 5分钟
            suspicious_connections.append({
                "connection_id": conn.id,
                "active_duration": conn.active_duration,
                "last_query": conn.last_query,
                "stack_trace": conn.stack_trace
            })
    
    return {
        "total_connections": len(connection_info),
        "suspicious_connections": suspicious_connections,
        "recommendations": generate_leak_recommendations(suspicious_connections)
    }

def generate_leak_recommendations(suspicious_connections):
    """生成连接泄漏修复建议"""
    recommendations = []
    
    if suspicious_connections:
        recommendations.extend([
            "检查应用代码是否正确关闭数据库连接",
            "验证异常处理逻辑是否会导致连接未释放",
            "考虑减少连接最大生存时间",
            "启用连接泄漏检测和自动回收"
        ])
    
    return recommendations
```

#### 2. 性能问题诊断
```python
async def diagnose_performance_issues(pool_id: str):
    """诊断性能问题"""
    pool = DatabasePool()
    
    metrics = await pool.get_detailed_metrics(pool_id)
    slow_queries = await pool.get_slow_queries(pool_id, limit=10)
    
    issues = []
    
    # 检查高延迟
    if metrics.latency_p99 > 1000:
        issues.append({
            "type": "high_latency",
            "severity": "high",
            "description": f"P99延迟达到 {metrics.latency_p99}ms",
            "recommendations": [
                "检查数据库服务器性能",
                "优化慢查询",
                "增加连接池大小",
                "考虑读写分离"
            ]
        })
    
    # 检查高错误率
    if metrics.error_rate > 0.05:
        issues.append({
            "type": "high_error_rate", 
            "severity": "critical",
            "description": f"错误率达到 {metrics.error_rate:.2%}",
            "recommendations": [
                "检查数据库连接配置",
                "验证网络连通性",
                "检查数据库服务器状态",
                "增加错误重试机制"
            ]
        })
    
    return {
        "issues": issues,
        "slow_queries": slow_queries,
        "overall_health": "healthy" if not issues else "degraded"
    }
```

## 📋 运维检查清单

### 日常检查 (每日)
- [ ] 检查所有连接池健康状态
- [ ] 查看错误日志和告警
- [ ] 验证关键性能指标
- [ ] 确认备份和恢复机制正常

### 周期检查 (每周)
- [ ] 分析性能趋势
- [ ] 检查连接池配置是否需要调整
- [ ] 清理过期的监控数据
- [ ] 验证故障恢复流程

### 月度检查 (每月)
- [ ] 全面性能评估和调优
- [ ] 检查和更新告警规则
- [ ] 容量规划和扩展评估
- [ ] 安全性检查和更新

这个运维指南提供了完整的监控、告警、故障恢复和性能调优策略，确保 db-pool-rs 在生产环境中稳定可靠地运行。