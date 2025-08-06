# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 项目概述

这是一个基于 Rust + PyO3 的高性能异步数据库连接池框架，支持多种数据库类型，提供 Python 友好的接口，数据自动转换为 Polars DataFrame。

## 核心架构

### 分层架构
- **应用层**: Python 应用程序接口
- **接口层**: PyO3 桥接层和异步接口包装器
- **业务层**: 连接池管理器、配置管理器、数据库工厂
- **抽象层**: 数据库特征、连接特征、行数据特征
- **实现层**: 各种数据库的具体实现 (MSSQL, PostgreSQL, Redis, SQLite, InfluxDB)
- **基础设施层**: 连接池、异步运行时、错误处理

### 项目结构
```
src/
├── lib.rs                  # PyO3 主入口
├── core/                   # 核心模块
│   ├── mod.rs             # 核心模块入口
│   ├── config.rs          # 统一配置管理
│   ├── pool_manager.rs    # 全局连接池管理器
│   ├── error.rs           # 错误类型定义
│   └── types.rs           # 通用类型
├── databases/             # 数据库模块
│   ├── mod.rs             # 数据库模块入口
│   ├── traits.rs          # 数据库抽象特征
│   ├── factory.rs         # 数据库工厂模式
│   ├── mssql/             # MSSQL 实现
│   ├── postgresql/        # PostgreSQL 实现
│   ├── redis/             # Redis 实现
│   ├── sqlite/            # SQLite 实现
│   └── influxdb/          # InfluxDB 实现
├── utils/                 # 工具模块
│   ├── mod.rs             # 工具模块入口
│   ├── dataframe.rs       # DataFrame 转换工具
│   ├── async_runtime.rs   # 异步运行时管理
│   └── validation.rs      # 数据验证工具
└── python/                # Python 接口模块
    ├── mod.rs             # Python 接口模块入口
    ├── pool.rs            # Python 连接池类
    ├── connection.rs      # Python 连接类
    └── exceptions.rs      # Python 异常类
```

## 常用命令

### 开发环境设置
```bash
# 安装 UV 包管理器
curl -LsSf https://astral.sh/uv/install.sh | sh

# 初始化开发环境
uv sync

# 开发模式构建和安装
uv run maturin develop

# 运行测试
uv run pytest tests/
```

### 开发工作流
```bash
# 开发模式 - 自动重新编译
uv run maturin develop --release

# 运行 Rust 测试
cargo test

# 运行 Python 测试
uv run pytest

# 性能基准测试
uv run python examples/python/benchmark.py
cargo bench

# 构建 wheel 包
uv run maturin build --release
```

### 代码质量和格式化
```bash
# 代码检查和格式化
uv run ruff check .
uv run black .
uv run mypy .

# 运行所有测试并生成覆盖率报告
uv run pytest --cov=db_pool_rs --cov-report=html
```

### 特定功能构建
```bash
# 仅构建特定数据库支持
uv run maturin develop --features mssql
uv run maturin develop --features postgresql
uv run maturin develop --features all-databases

# 完整功能构建
uv run maturin develop --features full
```

## 数据库支持状态

| 数据库 | 状态 | 特性 |
|--------|------|------|
| MSSQL | ✅ 完整支持 | 连接池、事务、批量操作 |
| PostgreSQL | 🚧 开发中 | 连接池、JSON 支持 |
| Redis | 🚧 开发中 | 连接池、发布订阅 |
| SQLite | 🚧 开发中 | 连接池、WAL 模式 |
| InfluxDB | 📋 计划中 | 时序数据、聚合查询 |

## 核心概念

### 连接池管理器 (PoolManager)
全局连接池管理器，负责管理多个数据库连接池实例：
- 创建、删除、查询连接池状态
- 执行查询和批量操作
- 监控和指标收集

### 数据库抽象特征 (DatabaseConnection)
所有数据库实现必须实现的核心特征：
- `create_pool()`: 创建连接池
- `execute_query()`: 执行查询
- `execute_non_query()`: 执行非查询操作
- `execute_transaction()`: 执行事务
- `rows_to_dataframe()`: 将结果转换为 DataFrame

### 配置系统 (DatabaseConfig)
支持多种配置方式的统一配置系统：
- 从 URL 字符串配置
- 从字典配置
- 从环境变量配置
- 配置验证

## 扩展开发

### 添加新数据库支持
1. 在 `src/databases/` 下创建新数据库模块
2. 实现 `DatabaseConnection` 特征
3. 在 `factory.rs` 中注册新数据库
4. 更新 `Cargo.toml` 和 `pyproject.toml`

### 中间件系统
支持查询中间件，可以：
- 在查询执行前修改 SQL
- 在查询执行后处理结果
- 处理错误情况

## 性能优化

### 连接池优化
- 自适应连接管理
- 智能负载均衡
- 连接复用和健康检查

### 数据转换优化
- 零拷贝数据转换
- 批量操作支持
- 缓存机制

## 测试策略

### 测试分层
- **单元测试**: 测试核心组件
- **集成测试**: 测试数据库连接和查询
- **Python 接口测试**: 测试 Python API
- **性能测试**: 测试性能和基准

### 测试命令
```bash
# 运行所有测试
uv run pytest

# 运行特定测试
uv run pytest tests/test_mssql.py -v

# 运行集成测试
uv run pytest tests/integration/ -v

# 运行性能测试
uv run pytest tests/benchmark/ -v
```

## 错误处理

### 错误类型
- `ConnectionError`: 连接相关错误
- `QueryError`: 查询相关错误
- `ConfigError`: 配置相关错误
- `DataConversionError`: 数据转换错误

### 错误恢复
- 自动重试机制
- 连接池重建
- 优雅降级

## 监控和指标

### 性能指标
- 连接池状态监控
- 查询性能统计
- 错误率统计
- 资源使用监控

### 健康检查
- 连接健康检查
- 数据库可达性检查
- 资源使用检查

## 部署和分发

### 构建配置
- 使用 Maturin 构建 Python wheel
- 支持多平台构建
- 优化的发布构建

### CI/CD
- 多 Python 版本测试
- 多平台构建测试
- 自动化发布流程

## 开发注意事项

### Rust 开发
- 遵循 Rust 官方代码风格
- 使用 `async/await` 进行异步编程
- 充分利用 Rust 的类型系统
- 注意内存安全和并发安全

### Python 接口开发
- 使用 PyO3 进行 Rust-Python 绑定
- 提供 Python 友好的 API
- 自动转换为 Polars DataFrame
- 异步操作支持

### 性能考虑
- 避免不必要的内存分配
- 使用零拷贝技术
- 优化热点路径
- 合理使用并发

## 常见问题

### 构建问题
- 确保 Rust 工具链正确安装
- 检查依赖版本兼容性
- 使用正确的特性标志

### 运行时问题
- 检查异步运行时配置
- 确保数据库连接配置正确
- 监控资源使用情况

### 性能问题
- 调整连接池配置
- 优化查询语句
- 使用批量操作