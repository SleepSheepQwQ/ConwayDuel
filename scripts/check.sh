#!/bin/bash
# 检查脚本

set -e

echo "开始检查 ConwayDuel..."

# 检查 Rust 代码
echo "检查 Rust 代码..."
cargo check

# 检查 clippy
echo "运行 clippy..."
cargo clippy -- -D warnings

# 检查格式
echo "检查代码格式..."
cargo fmt -- --check

echo "检查完成！"
