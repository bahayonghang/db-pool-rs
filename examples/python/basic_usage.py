"""
基础使用示例 - 演示db-pool-rs的核心功能
"""

import asyncio
import sys
import logging
from pathlib import Path

# 添加项目路径
sys.path.insert(0, str(Path(__file__).parent.parent.parent / "python"))

# 配置日志
logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

async def main():
    """主函数演示基础用法"""
    try:
        # 导入数据库连接池
        from db_pool_rs import DatabasePool, get_deployment_mode
        
        logger.info(f"当前部署模式: {get_deployment_mode()}")
        
        # 创建连接池管理器
        pool = DatabasePool()
        logger.info("✅ 数据库连接池管理器创建成功")
        
        # 创建MSSQL连接池（使用模拟配置）
        await pool.create_pool(
            pool_id="demo_mssql",
            db_type="mssql",
            host="localhost",
            port=1433,
            database="demo_db",
            username="demo_user",
            password="demo_pass",
            min_connections=2,
            max_connections=10,
            application_name="db-pool-rs-demo"
        )
        logger.info("✅ MSSQL连接池создан successful")
        
        # 执行查询
        logger.info("🔍 执行查询测试...")
        result = await pool.query("demo_mssql", "SELECT * FROM users LIMIT 10")
        logger.info(f"查询结果: {result}")
        
        # 执行更新操作
        logger.info("📝 执行更新操作...")
        affected_rows = await pool.execute(
            "demo_mssql", 
            "UPDATE users SET last_login = GETDATE() WHERE id = 1"
        )
        logger.info(f"更新影响行数: {affected_rows}")
        
        # 批量操作
        logger.info("📦 执行批量操作...")
        batch_results = await pool.execute_batch("demo_mssql", [
            "INSERT INTO logs VALUES ('info', 'Demo started')",
            "INSERT INTO logs VALUES ('info', 'Query executed')",
            {"sql": "INSERT INTO logs VALUES (?, ?)", "params": {"level": "info", "message": "Batch completed"}}
        ])
        logger.info(f"批量操作结果: {batch_results}")
        
        # 获取连接池状态
        logger.info("📊 获取连接池状态...")
        status = await pool.get_pool_status("demo_mssql")
        logger.info(f"连接池状态: {status}")
        
        # 获取性能指标
        logger.info("📈 获取性能指标...")
        metrics = await pool.get_pool_metrics("demo_mssql")
        logger.info(f"性能指标: {metrics}")
        
        # 健康检查
        logger.info("🔧 执行健康检查...")
        is_healthy = await pool.health_check("demo_mssql")
        logger.info(f"连接池健康状态: {'健康' if is_healthy else '异常'}")
        
        # 清理资源
        logger.info("🧹 清理资源...")
        await pool.remove_pool("demo_mssql")
        logger.info("✅ 连接池已清理")
        
        logger.info("🎉 演示完成！")
        
    except Exception as e:
        logger.error(f"❌ 演示过程中发生错误: {e}")
        import traceback
        traceback.print_exc()

if __name__ == "__main__":
    print("🚀 DB-Pool-RS 基础使用演示")
    print("=" * 40)
    asyncio.run(main())