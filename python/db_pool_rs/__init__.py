"""
db-pool-rs: 高性能异步数据库连接池框架

基于 Rust 实现的高性能数据库连接池，支持多种数据库类型，
自动转换查询结果为 Polars DataFrame。
"""

__version__ = "0.1.0"
__author__ = "Your Name <bahayonghang@gmail.com>"

import os
import sys
from typing import Optional

# 检测部署模式
DEPLOYMENT_MODE = os.environ.get("DB_POOL_MODE", "auto")

def _detect_deployment_mode() -> str:
    """自动检测最佳部署模式"""
    try:
        # 尝试导入Rust扩展
        from . import _db_pool_rs
        return "full"
    except ImportError:
        try:
            # 尝试导入balanced模式
            from .balanced import DatabasePool
            return "balanced"
        except ImportError:
            # 回退到simple模式
            return "simple"

def _get_database_pool_class():
    """根据部署模式获取DatabasePool类"""
    mode = DEPLOYMENT_MODE
    
    if mode == "auto":
        mode = _detect_deployment_mode()
    
    if mode == "full":
        try:
            from ._db_pool_rs import DatabasePool
            return DatabasePool
        except ImportError:
            print("警告: 无法加载完整Rust模式，回退到balanced模式")
            mode = "balanced"
    
    if mode == "balanced":
        try:
            from .balanced import DatabasePool
            return DatabasePool
        except ImportError:
            print("警告: 无法加载balanced模式，回退到simple模式")
            mode = "simple"
    
    if mode == "simple":
        from .simple import DatabasePool
        return DatabasePool
    
    raise ImportError(f"无法加载任何部署模式: {mode}")

# 导出主要类
DatabasePool = _get_database_pool_class()

# 导出异常类
class DbPoolError(Exception):
    """数据库连接池错误基类"""
    pass

class ConnectionError(DbPoolError):
    """连接相关错误"""
    pass

class QueryError(DbPoolError):
    """查询相关错误"""
    pass

class ConfigError(DbPoolError):
    """配置相关错误"""
    pass

# 导出工具函数
def get_version() -> str:
    """获取版本信息"""
    return __version__

def get_deployment_mode() -> str:
    """获取当前部署模式"""
    return DEPLOYMENT_MODE if DEPLOYMENT_MODE != "auto" else _detect_deployment_mode()

def supported_databases() -> list:
    """获取支持的数据库类型"""
    return ["mssql", "postgresql", "redis", "sqlite", "influxdb"]

# 模块级别的便利函数
async def create_pool(
    pool_id: str,
    db_type: str,
    host: str = "localhost",
    port: Optional[int] = None,
    database: str = "default",
    username: str = "",
    password: str = "",
    **kwargs
) -> DatabasePool:
    """
    便利函数：创建数据库连接池
    
    Args:
        pool_id: 连接池唯一标识
        db_type: 数据库类型 (mssql, postgresql, redis, sqlite, influxdb)
        host: 数据库主机地址
        port: 数据库端口 (可选，使用默认端口)
        database: 数据库名称
        username: 用户名
        password: 密码
        **kwargs: 其他配置参数
    
    Returns:
        DatabasePool: 配置好的数据库连接池实例
    """
    # 设置默认端口
    if port is None:
        port_defaults = {
            "mssql": 1433,
            "postgresql": 5432,
            "redis": 6379,
            "sqlite": 0,
            "influxdb": 8086,
        }
        port = port_defaults.get(db_type, 0)
    
    pool = DatabasePool()
    await pool.create_pool(
        pool_id=pool_id,
        db_type=db_type,
        host=host,
        port=port,
        database=database,
        username=username,
        password=password,
        **kwargs
    )
    return pool

__all__ = [
    "DatabasePool",
    "DbPoolError",
    "ConnectionError",
    "QueryError", 
    "ConfigError",
    "get_version",
    "get_deployment_mode",
    "supported_databases",
    "create_pool",
]