#!/bin/bash

# DB-Pool-RS Simple模式设置脚本
# 仅使用Python实现，快速上手，零依赖

set -e

echo "🚀 设置 DB-Pool-RS Simple 模式"
echo "================================"

# 检查Python版本
python_version=$(python3 --version 2>&1 | cut -d' ' -f2 | cut -d'.' -f1,2)
required_version="3.8"

if [ "$(printf '%s\n' "$required_version" "$python_version" | sort -V | head -n1)" != "$required_version" ]; then
    echo "❌ 错误: 需要Python $required_version 或更高版本，当前版本: $python_version"
    exit 1
fi

echo "✅ Python版本检查通过: $python_version"

# 检查UV是否安装
if ! command -v uv &> /dev/null; then
    echo "📦 安装UV包管理器..."
    curl -LsSf https://astral.sh/uv/install.sh | sh
    source $HOME/.cargo/env
    echo "✅ UV安装完成"
else
    echo "✅ UV已安装: $(uv --version)"
fi

# 同步依赖（仅Python依赖）
echo "📦 安装Python依赖..."
uv sync --extra simple

# 设置环境变量
export DB_POOL_MODE=simple
echo "export DB_POOL_MODE=simple" >> ~/.bashrc

# 创建配置文件
cat > .env << EOF
# DB-Pool-RS Simple模式配置
DB_POOL_MODE=simple
LOG_LEVEL=INFO
EOF

echo "✅ Simple模式设置完成！"
echo ""
echo "🎯 特性说明："
echo "  - 纯Python实现，无需编译"
echo "  - 兼容性最好，启动最快"
echo "  - 适合开发和测试环境"
echo "  - 性能: ~5,000 QPS"
echo ""
echo "📝 使用方法："
echo "  python -c \"import asyncio; from db_pool_rs import DatabasePool; print('Simple模式就绪')\""
echo ""
echo "🔄 升级到其他模式："
echo "  ./scripts/setup_balanced.sh  # 升级到Balanced模式"
echo "  ./scripts/setup_full.sh      # 升级到Full模式"