import pytest
import asyncio
import sys
import os
from pathlib import Path

# 添加项目路径
project_root = Path(__file__).parent.parent.parent
sys.path.insert(0, str(project_root / "python"))

@pytest.fixture
def event_loop():
    """创建事件循环"""
    loop = asyncio.new_event_loop()
    asyncio.set_event_loop(loop)
    yield loop
    loop.close()

@pytest.mark.asyncio
async def test_simple_pool_import():
    """测试Simple模式导入"""
    # 强制使用Simple模式
    os.environ["DB_POOL_MODE"] = "simple"
    
    from db_pool_rs.simple import DatabasePool
    
    pool = DatabasePool("simple")
    assert pool.deployment_mode == "simple"
    
    # 测试版本和支持的数据库
    version = pool.version()
    assert isinstance(version, str)
    assert "simple" in version
    
    databases = pool.supported_databases()
    assert isinstance(databases, list)
    assert "mssql" in databases

@pytest.mark.asyncio
async def test_simple_pool_creation():
    """测试Simple模式连接池创建"""
    os.environ["DB_POOL_MODE"] = "simple"
    
    from db_pool_rs.simple import DatabasePool
    
    pool = DatabasePool("simple")
    
    # 创建连接池
    await pool.create_pool(
        pool_id="test_pool",
        db_type="mssql",
        host="localhost",
        port=1433,
        database="test_db",
        username="test_user",
        password="test_pass",
        min_connections=2,
        max_connections=5
    )
    
    # 验证连接池存在
    pools = pool.list_pools()
    assert "test_pool" in pools
    
    # 获取连接池状态
    status = await pool.get_pool_status("test_pool")
    assert status["pool_id"] == "test_pool"
    assert status["db_type"] == "mssql"
    
    # 清理
    await pool.remove_pool("test_pool")

@pytest.mark.asyncio
async def test_simple_pool_query():
    """测试Simple模式查询"""
    os.environ["DB_POOL_MODE"] = "simple"
    
    from db_pool_rs.simple import DatabasePool
    
    pool = DatabasePool("simple")
    
    await pool.create_pool(
        pool_id="test_pool",
        db_type="mssql",
        host="localhost",
        port=1433,
        database="test_db",
        username="test_user",
        password="test_pass"
    )
    
    # 执行查询
    result = await pool.query("test_pool", "SELECT * FROM users")
    
    # 验证结果格式
    assert isinstance(result, dict)
    assert "columns" in result
    assert "data" in result
    assert "shape" in result
    
    # 执行非查询操作
    affected_rows = await pool.execute("test_pool", "UPDATE users SET name = 'test'")
    assert isinstance(affected_rows, int)
    
    # 清理
    await pool.remove_pool("test_pool")

@pytest.mark.asyncio
async def test_simple_pool_batch_operations():
    """测试Simple模式批量操作"""
    os.environ["DB_POOL_MODE"] = "simple"
    
    from db_pool_rs.simple import DatabasePool
    
    pool = DatabasePool("simple")
    
    await pool.create_pool(
        pool_id="test_pool",
        db_type="mssql",
        host="localhost",
        port=1433,
        database="test_db",
        username="test_user",
        password="test_pass"
    )
    
    # 批量操作
    operations = [
        "INSERT INTO logs VALUES ('info', 'Test 1')",
        "INSERT INTO logs VALUES ('info', 'Test 2')",
        {"sql": "INSERT INTO logs VALUES (?, ?)", "params": {"level": "info", "message": "Test 3"}}
    ]
    
    results = await pool.execute_batch("test_pool", operations)
    
    # 验证结果
    assert isinstance(results, list)
    assert len(results) == 3
    
    for result in results:
        assert "affected_rows" in result
        assert "execution_time_ms" in result
        assert "error" in result
    
    # 清理
    await pool.remove_pool("test_pool")

@pytest.mark.asyncio
async def test_simple_pool_metrics():
    """测试Simple模式指标"""
    os.environ["DB_POOL_MODE"] = "simple"
    
    from db_pool_rs.simple import DatabasePool
    
    pool = DatabasePool("simple")
    
    await pool.create_pool(
        pool_id="test_pool",
        db_type="mssql",
        host="localhost",
        port=1433,
        database="test_db",
        username="test_user",
        password="test_pass"
    )
    
    # 执行一些查询以生成指标
    await pool.query("test_pool", "SELECT 1")
    await pool.query("test_pool", "SELECT 2")
    
    # 获取指标
    metrics = await pool.get_pool_metrics("test_pool")
    
    # 验证指标
    assert isinstance(metrics, dict)
    assert "pool_id" in metrics
    assert "queries_per_second" in metrics
    assert "error_rate" in metrics
    assert "avg_query_time_ms" in metrics
    assert "connection_utilization" in metrics
    
    assert metrics["pool_id"] == "test_pool"
    
    # 清理
    await pool.remove_pool("test_pool")

@pytest.mark.asyncio
async def test_simple_pool_health_check():
    """测试Simple模式健康检查"""
    os.environ["DB_POOL_MODE"] = "simple"
    
    from db_pool_rs.simple import DatabasePool
    
    pool = DatabasePool("simple")
    
    await pool.create_pool(
        pool_id="test_pool",
        db_type="mssql",
        host="localhost",
        port=1433,
        database="test_db",
        username="test_user",
        password="test_pass"
    )
    
    # 健康检查
    is_healthy = await pool.health_check("test_pool")
    assert isinstance(is_healthy, bool)
    
    # 清理
    await pool.remove_pool("test_pool")

@pytest.mark.asyncio
async def test_main_module_import():
    """测试主模块导入和自动模式检测"""
    # 清除环境变量以测试自动检测
    if "DB_POOL_MODE" in os.environ:
        del os.environ["DB_POOL_MODE"]
    
    from db_pool_rs import DatabasePool, get_deployment_mode, supported_databases
    
    # 测试自动模式检测
    mode = get_deployment_mode()
    assert mode in ["simple", "balanced", "full"]
    
    # 测试支持的数据库
    databases = supported_databases()
    assert isinstance(databases, list)
    assert "mssql" in databases
    
    # 测试DatabasePool创建
    pool = DatabasePool()
    assert pool is not None

def test_error_handling():
    """测试错误处理"""
    os.environ["DB_POOL_MODE"] = "simple"
    
    from db_pool_rs import DbPoolError, ConnectionError, QueryError, ConfigError
    
    # 测试异常类层次结构
    assert issubclass(ConnectionError, DbPoolError)
    assert issubclass(QueryError, DbPoolError)
    assert issubclass(ConfigError, DbPoolError)
    
    # 测试异常创建
    error = DbPoolError("Test error")
    assert str(error) == "Test error"

if __name__ == "__main__":
    # 运行测试
    pytest.main([__file__, "-v"])