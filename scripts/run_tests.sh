#!/bin/bash

# DB-Pool-RS 测试运行脚本

set -e

echo "🧪 DB-Pool-RS 测试套件"
echo "========================="

# 设置环境变量
export RUST_LOG=info
export DB_POOL_MODE=simple

# 检查依赖
echo "📋 检查测试依赖..."

# 检查Rust
if ! command -v cargo &> /dev/null; then
    echo "❌ Cargo未找到，请安装Rust工具链"
    exit 1
fi

# 检查Python
if ! command -v python3 &> /dev/null; then
    echo "❌ Python3未找到"
    exit 1
fi

# 检查pytest
if ! python3 -c "import pytest" &> /dev/null; then
    echo "📦 安装pytest..."
    pip3 install pytest pytest-asyncio
fi

echo "✅ 依赖检查完成"

# 运行Rust测试
echo ""
echo "📦 运行Rust单元测试..."
echo "------------------------"

rust_tests=(
    "test_config"
    "test_pool_manager" 
    "test_types"
    "test_monitoring"
)

rust_passed=0
rust_total=${#rust_tests[@]}

for test in "${rust_tests[@]}"; do
    echo "🧪 运行 $test..."
    if cargo test --test "$test" --quiet; then
        echo "  ✅ $test 通过"
        ((rust_passed++))
    else
        echo "  ❌ $test 失败"
    fi
done

echo "Rust测试结果: $rust_passed/$rust_total 通过"

# 运行Python测试
echo ""
echo "🐍 运行Python测试..."
echo "--------------------"

python_passed=true
if python3 -m pytest tests/python/ -v; then
    echo "✅ Python测试通过"
else
    echo "❌ Python测试失败"
    python_passed=false
fi

# 运行集成测试
echo ""
echo "🔗 运行集成测试..."
echo "-------------------"

integration_passed=true
echo "🧪 运行基础使用示例..."
if python3 examples/python/basic_usage.py; then
    echo "✅ 基础使用示例通过"
else
    echo "❌ 基础使用示例失败"
    integration_passed=false
fi

# 运行性能基准测试（可选）
if [[ "${1:-}" == "--benchmark" ]]; then
    echo ""
    echo "📊 运行性能基准测试..."
    echo "----------------------"
    
    if python3 examples/python/benchmark.py; then
        echo "✅ 性能基准测试完成"
    else
        echo "❌ 性能基准测试失败"
    fi
fi

# 总结结果
echo ""
echo "📋 测试结果总结"
echo "================"

total_passed=true

echo "Rust单元测试: $rust_passed/$rust_total"
if [[ $rust_passed -ne $rust_total ]]; then
    total_passed=false
fi

echo "Python测试: $(if $python_passed; then echo "通过"; else echo "失败"; fi)"
if ! $python_passed; then
    total_passed=false
fi

echo "集成测试: $(if $integration_passed; then echo "通过"; else echo "失败"; fi)"
if ! $integration_passed; then
    total_passed=false
fi

if $total_passed; then
    echo ""
    echo "🎉 所有测试通过！"
    exit 0
else
    echo ""
    echo "❌ 部分测试失败，请检查错误信息"
    exit 1
fi