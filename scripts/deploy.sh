#!/bin/bash
# 部署脚本

set -e

echo "开始部署 ConwayDuel..."

# 构建
./scripts/build.sh

# 部署到 GitHub Pages
echo "部署到 GitHub Pages..."
# 这里可以添加 gh-pages 部署逻辑

echo "部署完成！"
