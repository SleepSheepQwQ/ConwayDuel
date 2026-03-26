# ConwayDuel

基于 Rust/WASM + WebGL2 的三阵营太空对战游戏。

## 快速开始

### 环境要求
- Rust (wasm32-unknown-unknown target)
- trunk
- Node.js (TypeScript 编译)

### 一键部署 (Termux)
```bash
bash scripts/deploy.sh
```

### 手动构建
```bash
rustup target add wasm32-unknown-unknown
cargo install trunk
trunk serve
```

访问 http://localhost:8080

## 项目结构
```
src/
  lib.rs          # WASM 入口
  app.rs          # 游戏主循环
  config.rs       # 全局配置
  main.ts         # TypeScript 加载器
  style.css       # 全屏画布样式
  ecs/            # ECS 组件和事件
  core/           # AI/战斗/物理/渲染
```
