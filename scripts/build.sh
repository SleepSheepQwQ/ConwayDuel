#!/bin/bash
# 构建脚本

set -e

echo "开始构建 ConwayDuel..."

# 检查 trunk 是否安装
if ! command -v trunk &> /dev/null; then
    echo "错误: trunk 未安装"
    echo "请运行: cargo install trunk"
    exit 1
fi

# 构建
trunk build --release

echo "构建完成！"
echo "产物位于 dist/ 目录"
