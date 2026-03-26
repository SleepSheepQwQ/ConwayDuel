#!/bin/bash
# ConwayDuel 一键部署脚本 (Termux)
set -e

echo "===== ConwayDuel 部署脚本 ====="

if ! command -v cargo &> /dev/null; then
    echo "[1/4] 安装 Rust 工具链..."
    pkg install -y rust
    source $HOME/.cargo/env
else
    echo "[1/4] Rust 已安装: $(rustc --version)"
fi

if ! command -v wasm-pack &> /dev/null; then
    echo "[2/4] 安装 wasm-pack..."
    cargo install wasm-pack
else
    echo "[2/4] wasm-pack 已安装"
fi

if ! command -v trunk &> /dev/null; then
    echo "[3/4] 安装 trunk..."
    cargo install trunk
else
    echo "[3/4] trunk 已安装"
fi

echo "[4/4] 检查 WASM 目标..."
rustup target add wasm32-unknown-unknown 2>/dev/null || true

echo ""
echo "===== 开始构建 ====="
echo "启动开发服务器..."
echo "访问地址: http://localhost:8080"
echo ""
trunk serve
