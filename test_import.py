#!/usr/bin/env python3
"""
简单的导入测试，验证db-pool-rs包是否成功安装
"""

def test_import():
    """测试导入db-pool-rs包"""
    try:
        import db_pool_rs
        print("✅ 成功导入 db_pool_rs")
        
        # 测试通过Python接口创建连接池管理器
        pool_manager = db_pool_rs.DatabasePool()
        print("✅ 成功创建 DatabasePool 实例")
        
        # 显示版本信息
        version = pool_manager.version()
        print(f"✅ db-pool-rs 版本: {version}")
        
        # 列出连接池（应该为空）
        pools = pool_manager.list_pools()
        print(f"✅ 当前连接池列表: {pools}")
        
        return True
        
    except ImportError as e:
        print(f"❌ 导入失败: {e}")
        return False
    except Exception as e:
        print(f"❌ 测试失败: {e}")
        return False

if __name__ == "__main__":
    print("🚀 开始测试 db-pool-rs...")
    success = test_import()
    
    if success:
        print("\n🎉 所有测试通过！db-pool-rs 构建和安装成功！")
    else:
        print("\n💥 测试失败，请检查构建和安装。")
        exit(1)