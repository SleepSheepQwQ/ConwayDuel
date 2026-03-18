# ConwayDuel - 几何飞船观战战斗

一个基于 Rust/WebAssembly 的 2D 太空战斗游戏，三艘不同颜色的飞船在战场上自动战斗。

## 技术栈

- **Rust** - 核心游戏逻辑
- **WebAssembly** - 高性能浏览器运行
- **WebGL2** - GPU 加速渲染
- **hecs** - 实体组件系统
- **glam** - 数学库

## 快速开始

### 前置要求

- Rust (edition 2021)
- trunk (`cargo install trunk`)

### 本地运行

```bash
# 安装依赖
cargo build

# 启动开发服务器
trunk serve
```

访问 http://localhost:8080

### 构建

```bash
trunk build --release
```

## 游戏特性

- 三阵营飞船（红、绿、蓝）
- AI 自动战斗
- 边界反弹
- 碰撞检测
- 爆炸特效
- 飞船重生系统

## 项目结构

```
src/
├── lib.rs          # WASM 入口
├── app.rs          # 游戏主循环
├── config.rs       # 配置
├── ecs/            # 实体组件系统
│   ├── components.rs
│   └── events.rs
└── core/           # 核心系统
    ├── ai/         # AI 系统
    ├── combat/     # 战斗系统
    ├── physics/    # 物理系统
    └── render/     # 渲染系统
```

## 许可证

MIT
