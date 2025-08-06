"""
性能基准测试 - 比较不同部署模式的性能
"""

import asyncio
import time
import sys
import os
import statistics
from pathlib import Path
from concurrent.futures import ThreadPoolExecutor
import logging

# 添加项目路径
project_root = Path(__file__).parent.parent.parent
sys.path.insert(0, str(project_root / "python"))

# 配置日志
logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

class BenchmarkRunner:
    def __init__(self, mode="simple"):
        self.mode = mode
        self.pool = None
        
    async def setup(self):
        """设置测试环境"""
        os.environ["DB_POOL_MODE"] = self.mode
        
        from db_pool_rs import DatabasePool
        self.pool = DatabasePool(self.mode)
        
        await self.pool.create_pool(
            pool_id="benchmark_pool",
            db_type="mssql",
            host="localhost",
            port=1433,
            database="benchmark_db",
            username="bench_user",
            password="bench_pass",
            min_connections=10,
            max_connections=50
        )
        
    async def cleanup(self):
        """清理测试环境"""
        if self.pool:
            await self.pool.remove_pool("benchmark_pool")
    
    async def benchmark_simple_queries(self, num_queries=1000):
        """基准测试：简单查询"""
        logger.info(f"🔍 基准测试: {num_queries} 个简单查询 ({self.mode}模式)")
        
        start_time = time.time()
        latencies = []
        
        for i in range(num_queries):
            query_start = time.time()
            try:
                result = await self.pool.query("benchmark_pool", "SELECT 1 as test_value")
                latencies.append((time.time() - query_start) * 1000)  # 转换为毫秒
            except Exception as e:
                logger.error(f"查询失败: {e}")
        
        end_time = time.time()
        total_time = end_time - start_time
        
        return {
            "total_time": total_time,
            "qps": num_queries / total_time,
            "avg_latency_ms": statistics.mean(latencies) if latencies else 0,
            "p99_latency_ms": statistics.quantiles(latencies, n=100)[98] if latencies else 0,
            "min_latency_ms": min(latencies) if latencies else 0,
            "max_latency_ms": max(latencies) if latencies else 0,
        }
    
    async def benchmark_concurrent_queries(self, num_concurrent=10, queries_per_thread=100):
        """基准测试：并发查询"""
        logger.info(f"🔀 基准测试: {num_concurrent} 并发 x {queries_per_thread} 查询 ({self.mode}模式)")
        
        async def worker(worker_id):
            latencies = []
            for i in range(queries_per_thread):
                query_start = time.time()
                try:
                    result = await self.pool.query("benchmark_pool", f"SELECT {worker_id} as worker_id, {i} as query_id")
                    latencies.append((time.time() - query_start) * 1000)
                except Exception as e:
                    logger.error(f"Worker {worker_id} 查询 {i} 失败: {e}")
            return latencies
        
        start_time = time.time()
        
        # 创建并发任务
        tasks = [worker(i) for i in range(num_concurrent)]
        results = await asyncio.gather(*tasks)
        
        end_time = time.time()
        total_time = end_time - start_time
        
        # 汇总所有延迟
        all_latencies = []
        for worker_latencies in results:
            all_latencies.extend(worker_latencies)
        
        total_queries = num_concurrent * queries_per_thread
        
        return {
            "total_time": total_time,
            "qps": total_queries / total_time,
            "avg_latency_ms": statistics.mean(all_latencies) if all_latencies else 0,
            "p99_latency_ms": statistics.quantiles(all_latencies, n=100)[98] if all_latencies else 0,
            "min_latency_ms": min(all_latencies) if all_latencies else 0,
            "max_latency_ms": max(all_latencies) if all_latencies else 0,
            "concurrent_workers": num_concurrent,
        }
    
    async def benchmark_batch_operations(self, num_batches=100, batch_size=10):
        """基准测试：批量操作"""
        logger.info(f"📦 基准测试: {num_batches} 批次 x {batch_size} 操作 ({self.mode}模式)")
        
        start_time = time.time()
        
        for batch_id in range(num_batches):
            operations = []
            for op_id in range(batch_size):
                operations.append(f"INSERT INTO benchmark_log VALUES ({batch_id}, {op_id}, 'test')")
            
            try:
                results = await self.pool.execute_batch("benchmark_pool", operations)
            except Exception as e:
                logger.error(f"批次 {batch_id} 失败: {e}")
        
        end_time = time.time()
        total_time = end_time - start_time
        total_operations = num_batches * batch_size
        
        return {
            "total_time": total_time,
            "operations_per_second": total_operations / total_time,
            "batches_per_second": num_batches / total_time,
            "avg_batch_time_ms": (total_time / num_batches) * 1000,
        }

async def run_benchmarks():
    """运行所有基准测试"""
    print("🚀 DB-Pool-RS 性能基准测试")
    print("=" * 60)
    
    modes = ["simple"]  # 当前只有simple模式可用
    
    results = {}
    
    for mode in modes:
        print(f"\n📊 测试 {mode.upper()} 模式")
        print("-" * 40)
        
        runner = BenchmarkRunner(mode)
        
        try:
            await runner.setup()
            
            # 简单查询基准
            simple_result = await runner.benchmark_simple_queries(1000)
            print(f"  简单查询 (1000次):")
            print(f"    QPS: {simple_result['qps']:.2f}")
            print(f"    平均延迟: {simple_result['avg_latency_ms']:.2f}ms")
            print(f"    P99延迟: {simple_result['p99_latency_ms']:.2f}ms")
            
            # 并发查询基准
            concurrent_result = await runner.benchmark_concurrent_queries(10, 100)
            print(f"  并发查询 (10并发 x 100次):")
            print(f"    QPS: {concurrent_result['qps']:.2f}")
            print(f"    平均延迟: {concurrent_result['avg_latency_ms']:.2f}ms")
            print(f"    P99延迟: {concurrent_result['p99_latency_ms']:.2f}ms")
            
            # 批量操作基准
            batch_result = await runner.benchmark_batch_operations(50, 20)
            print(f"  批量操作 (50批次 x 20操作):")
            print(f"    操作/秒: {batch_result['operations_per_second']:.2f}")
            print(f"    批次/秒: {batch_result['batches_per_second']:.2f}")
            print(f"    平均批次时间: {batch_result['avg_batch_time_ms']:.2f}ms")
            
            results[mode] = {
                "simple_queries": simple_result,
                "concurrent_queries": concurrent_result,
                "batch_operations": batch_result,
            }
            
        except Exception as e:
            logger.error(f"{mode}模式测试失败: {e}")
            results[mode] = {"error": str(e)}
        
        finally:
            await runner.cleanup()
    
    # 输出对比结果
    print(f"\n📋 性能对比总结")
    print("=" * 60)
    
    print(f"{'模式':<10} {'简单查询QPS':<15} {'并发查询QPS':<15} {'批量操作/秒':<15}")
    print("-" * 60)
    
    for mode, result in results.items():
        if "error" not in result:
            simple_qps = result["simple_queries"]["qps"]
            concurrent_qps = result["concurrent_queries"]["qps"]
            batch_ops = result["batch_operations"]["operations_per_second"]
            print(f"{mode.upper():<10} {simple_qps:<15.2f} {concurrent_qps:<15.2f} {batch_ops:<15.2f}")
        else:
            print(f"{mode.upper():<10} {'ERROR':<15} {'ERROR':<15} {'ERROR':<15}")

if __name__ == "__main__":
    asyncio.run(run_benchmarks())