"""
Simple模式实现 - 纯Python数据库连接池

提供基础的数据库连接池功能，使用纯Python实现。
性能较低但兼容性最好，适合快速原型开发。
"""

import asyncio
import time
import json
from typing import Dict, List, Optional, Any, Union
from collections import deque
from dataclasses import dataclass, asdict
import logging

try:
    import polars as pl
except ImportError:
    pl = None

try:
    import pandas as pd
except ImportError:
    pd = None

logger = logging.getLogger(__name__)

@dataclass
class PoolStatus:
    """连接池状态"""
    pool_id: str
    db_type: str
    total_connections: int
    active_connections: int
    idle_connections: int
    waiting_connections: int
    is_healthy: bool
    last_error: Optional[str]
    uptime_seconds: float

@dataclass
class PoolMetrics:
    """连接池指标"""
    pool_id: str
    queries_per_second: float
    connection_utilization: float
    avg_query_time_ms: float
    p99_query_time_ms: float
    error_rate: float
    total_queries: int
    total_errors: int
    cache_hit_rate: float

class SimpleConnection:
    """简单连接包装器"""
    
    def __init__(self, connection_id: str, db_type: str):
        self.connection_id = connection_id
        self.db_type = db_type
        self.created_at = time.time()
        self.last_used = time.time()
        self.is_active = False
        self.native_connection = None
    
    async def connect(self, config: Dict[str, Any]) -> None:
        """建立数据库连接"""
        # 模拟连接建立
        await asyncio.sleep(0.01)  # 模拟连接延迟
        self.native_connection = f"mock_connection_{self.connection_id}"
    
    async def execute_query(self, sql: str, params: Optional[Dict] = None) -> List[Dict]:
        """执行查询"""
        await asyncio.sleep(0.001)  # 模拟查询延迟
        self.last_used = time.time()
        
        # 模拟查询结果
        if sql.strip().upper().startswith("SELECT"):
            return [
                {"id": 1, "name": "测试数据1", "value": 100.0},
                {"id": 2, "name": "测试数据2", "value": 200.0},
            ]
        return []
    
    async def execute_non_query(self, sql: str, params: Optional[Dict] = None) -> int:
        """执行非查询操作"""
        await asyncio.sleep(0.001)  # 模拟执行延迟
        self.last_used = time.time()
        return 1  # 模拟影响行数
    
    async def is_valid(self) -> bool:
        """检查连接是否有效"""
        return self.native_connection is not None
    
    async def close(self) -> None:
        """关闭连接"""
        self.native_connection = None

class SimplePool:
    """简单连接池实现"""
    
    def __init__(self, pool_id: str, config: Dict[str, Any]):
        self.pool_id = pool_id
        self.config = config
        self.db_type = config["db_type"]
        self.min_connections = config.get("min_connections", 5)
        self.max_connections = config.get("max_connections", 50)
        
        self.connections: deque = deque()
        self.active_connections: Dict[str, SimpleConnection] = {}
        self.total_connections = 0
        self.created_at = time.time()
        
        # 指标统计
        self.total_queries = 0
        self.total_errors = 0
        self.query_times = []
        self.last_error = None
        
        self._lock = asyncio.Lock()
    
    async def initialize(self) -> None:
        """初始化连接池"""
        # 创建最小连接数
        for i in range(self.min_connections):
            connection = SimpleConnection(f"{self.pool_id}_{i}", self.db_type)
            await connection.connect(self.config)
            self.connections.append(connection)
            self.total_connections += 1
    
    async def get_connection(self) -> SimpleConnection:
        """获取连接"""
        async with self._lock:
            # 尝试从空闲连接中获取
            while self.connections:
                connection = self.connections.popleft()
                if await connection.is_valid():
                    connection.is_active = True
                    self.active_connections[connection.connection_id] = connection
                    return connection
            
            # 没有空闲连接，创建新连接
            if self.total_connections < self.max_connections:
                connection = SimpleConnection(f"{self.pool_id}_{self.total_connections}", self.db_type)
                await connection.connect(self.config)
                connection.is_active = True
                self.active_connections[connection.connection_id] = connection
                self.total_connections += 1
                return connection
            
            # 连接池已满
            raise Exception("连接池已满")
    
    async def return_connection(self, connection: SimpleConnection) -> None:
        """归还连接"""
        async with self._lock:
            if connection.connection_id in self.active_connections:
                del self.active_connections[connection.connection_id]
                connection.is_active = False
                
                if await connection.is_valid():
                    self.connections.append(connection)
                else:
                    await connection.close()
                    self.total_connections -= 1
    
    async def execute_query(self, sql: str, params: Optional[Dict] = None) -> List[Dict]:
        """执行查询"""
        start_time = time.time()
        connection = None
        
        try:
            connection = await self.get_connection()
            result = await connection.execute_query(sql, params)
            
            # 记录指标
            query_time = (time.time() - start_time) * 1000
            self.query_times.append(query_time)
            self.total_queries += 1
            
            return result
            
        except Exception as e:
            self.total_errors += 1
            self.last_error = str(e)
            raise
        finally:
            if connection:
                await self.return_connection(connection)
    
    async def execute_non_query(self, sql: str, params: Optional[Dict] = None) -> int:
        """执行非查询操作"""
        start_time = time.time()
        connection = None
        
        try:
            connection = await self.get_connection()
            result = await connection.execute_non_query(sql, params)
            
            # 记录指标
            query_time = (time.time() - start_time) * 1000
            self.query_times.append(query_time)
            self.total_queries += 1
            
            return result
            
        except Exception as e:
            self.total_errors += 1
            self.last_error = str(e)
            raise
        finally:
            if connection:
                await self.return_connection(connection)
    
    async def get_status(self) -> PoolStatus:
        """获取连接池状态"""
        return PoolStatus(
            pool_id=self.pool_id,
            db_type=self.db_type,
            total_connections=self.total_connections,
            active_connections=len(self.active_connections),
            idle_connections=len(self.connections),
            waiting_connections=0,  # 简化实现
            is_healthy=True,
            last_error=self.last_error,
            uptime_seconds=time.time() - self.created_at,
        )
    
    async def get_metrics(self) -> PoolMetrics:
        """获取连接池指标"""
        uptime = time.time() - self.created_at
        qps = self.total_queries / uptime if uptime > 0 else 0
        
        avg_query_time = sum(self.query_times) / len(self.query_times) if self.query_times else 0
        p99_query_time = sorted(self.query_times)[int(len(self.query_times) * 0.99)] if self.query_times else 0
        
        error_rate = self.total_errors / self.total_queries if self.total_queries > 0 else 0
        
        connection_utilization = len(self.active_connections) / self.max_connections
        
        return PoolMetrics(
            pool_id=self.pool_id,
            queries_per_second=qps,
            connection_utilization=connection_utilization,
            avg_query_time_ms=avg_query_time,
            p99_query_time_ms=p99_query_time,
            error_rate=error_rate,
            total_queries=self.total_queries,
            total_errors=self.total_errors,
            cache_hit_rate=0.0,  # 暂未实现缓存
        )
    
    async def close(self) -> None:
        """关闭连接池"""
        # 关闭所有连接
        while self.connections:
            connection = self.connections.popleft()
            await connection.close()
        
        for connection in self.active_connections.values():
            await connection.close()
        
        self.active_connections.clear()
        self.total_connections = 0

class DatabasePool:
    """Simple模式数据库连接池"""
    
    def __init__(self, deployment_mode: str = "simple"):
        self.deployment_mode = deployment_mode
        self.pools: Dict[str, SimplePool] = {}
        self._initialized = False
        
        logger.info(f"初始化Simple模式数据库连接池: {deployment_mode}")
    
    async def create_pool(
        self,
        pool_id: str,
        db_type: str,
        host: str = "localhost",
        port: int = 1433,
        database: str = "default",
        username: str = "",
        password: str = "",
        min_connections: int = 5,
        max_connections: int = 50,
        **kwargs
    ) -> None:
        """创建连接池"""
        config = {
            "db_type": db_type,
            "host": host,
            "port": port,
            "database": database,
            "username": username,
            "password": password,
            "min_connections": min_connections,
            "max_connections": max_connections,
            **kwargs
        }
        
        pool = SimplePool(pool_id, config)
        await pool.initialize()
        self.pools[pool_id] = pool
        
        logger.info(f"创建连接池成功: {pool_id} ({db_type})")
    
    async def query(self, pool_id: str, sql: str, params: Optional[Dict] = None) -> Union[Any, Dict]:
        """执行查询"""
        if pool_id not in self.pools:
            raise KeyError(f"连接池不存在: {pool_id}")
        
        pool = self.pools[pool_id]
        rows = await pool.execute_query(sql, params)
        
        # 转换为DataFrame格式
        return self._rows_to_dataframe(rows)
    
    async def execute(self, pool_id: str, sql: str, params: Optional[Dict] = None) -> int:
        """执行非查询操作"""
        if pool_id not in self.pools:
            raise KeyError(f"连接池不存在: {pool_id}")
        
        pool = self.pools[pool_id]
        return await pool.execute_non_query(sql, params)
    
    async def execute_batch(self, pool_id: str, operations: List[Union[str, Dict]]) -> List[Dict]:
        """批量执行操作"""
        if pool_id not in self.pools:
            raise KeyError(f"连接池不存在: {pool_id}")
        
        pool = self.pools[pool_id]
        results = []
        
        for op in operations:
            start_time = time.time()
            try:
                if isinstance(op, str):
                    affected_rows = await pool.execute_non_query(op)
                    results.append({
                        "affected_rows": affected_rows,
                        "execution_time_ms": (time.time() - start_time) * 1000,
                        "error": None,
                    })
                elif isinstance(op, dict):
                    sql = op.get("sql", "")
                    params = op.get("params")
                    affected_rows = await pool.execute_non_query(sql, params)
                    results.append({
                        "affected_rows": affected_rows,
                        "execution_time_ms": (time.time() - start_time) * 1000,
                        "error": None,
                    })
            except Exception as e:
                results.append({
                    "affected_rows": 0,
                    "execution_time_ms": (time.time() - start_time) * 1000,
                    "error": str(e),
                })
        
        return results
    
    async def get_pool_status(self, pool_id: str) -> Dict[str, Any]:
        """获取连接池状态"""
        if pool_id not in self.pools:
            raise KeyError(f"连接池不存在: {pool_id}")
        
        pool = self.pools[pool_id]
        status = await pool.get_status()
        return asdict(status)
    
    async def get_pool_metrics(self, pool_id: str) -> Dict[str, Any]:
        """获取连接池指标"""
        if pool_id not in self.pools:
            raise KeyError(f"连接池不存在: {pool_id}")
        
        pool = self.pools[pool_id]
        metrics = await pool.get_metrics()
        return asdict(metrics)
    
    def list_pools(self) -> List[str]:
        """列出所有连接池"""
        return list(self.pools.keys())
    
    async def remove_pool(self, pool_id: str) -> None:
        """移除连接池"""
        if pool_id in self.pools:
            pool = self.pools.pop(pool_id)
            await pool.close()
            logger.info(f"移除连接池: {pool_id}")
    
    async def health_check(self, pool_id: str) -> bool:
        """健康检查"""
        if pool_id not in self.pools:
            return False
        
        try:
            # 执行简单查询测试
            await self.query(pool_id, "SELECT 1")
            return True
        except Exception:
            return False
    
    @staticmethod
    def version() -> str:
        """获取版本信息"""
        return "0.1.0-simple"
    
    @staticmethod
    def supported_databases() -> List[str]:
        """获取支持的数据库类型"""
        return ["mssql", "postgresql", "redis", "sqlite", "influxdb"]
    
    def _rows_to_dataframe(self, rows: List[Dict]) -> Union[Any, Dict]:
        """将行数据转换为DataFrame"""
        if not rows:
            return {"columns": [], "data": []}
        
        # 如果有Polars，优先使用
        if pl is not None:
            try:
                df = pl.DataFrame(rows)
                return {
                    "columns": df.columns,
                    "data": [list(row) for row in df.rows()],
                    "dtypes": {col: str(dtype) for col, dtype in zip(df.columns, df.dtypes)},
                    "shape": df.shape,
                }
            except Exception:
                pass
        
        # 回退到基础格式
        columns = list(rows[0].keys()) if rows else []
        data = [[row.get(col) for col in columns] for row in rows]
        
        return {
            "columns": columns,
            "data": data,
            "shape": (len(rows), len(columns)),
        }