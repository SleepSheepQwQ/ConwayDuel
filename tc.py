#!/usr/bin/env python3
"""
ConwayDuel 编译错误修复脚本
修复以下问题：
1. config.rs 中未使用的 glam::Vec2 导入
2. app.rs 中 Instant 在 WASM 环境的兼容性问题
"""

import os

# 项目根目录
ROOT_DIR = os.path.dirname(os.path.abspath(__file__))

def fix_config_rs():
    """修复 config.rs 中未使用的导入"""
    filepath = os.path.join(ROOT_DIR, "src/config.rs")
    
    with open(filepath, 'r', encoding='utf-8') as f:
        content = f.read()
    
    # 移除未使用的 glam::Vec2 导入
    content = content.replace("use glam::Vec2;\nuse serde::{Deserialize, Serialize};", 
                              "use serde::{Deserialize, Serialize};")
    
    with open(filepath, 'w', encoding='utf-8') as f:
        f.write(content)
    
    print("✓ 修复 src/config.rs - 移除未使用的 glam::Vec2 导入")


def fix_app_rs():
    """修复 app.rs 中 Instant 在 WASM 环境的问题"""
    filepath = os.path.join(ROOT_DIR, "src/app.rs")
    
    with open(filepath, 'r', encoding='utf-8') as f:
        content = f.read()
    
    # 替换 Instant 为 web_sys::Performance
    old_imports = """use std::time::{Duration, Instant};"""
    new_imports = """use std::time::Duration;"""
    
    content = content.replace(old_imports, new_imports)
    
    # 替换 Instant::now() 为性能计时
    content = content.replace(
        "self.last_frame_time = Some(Instant::now());",
        """let now_ms = web_sys::window()
            .and_then(|w| w.performance())
            .map(|p| p.now() as f64)
            .unwrap_or(0.0);
        self.last_frame_time = Some(now_ms);"""
    )
    
    # 替换 frame_time 计算
    old_frame_time = """let now = Instant::now();
            let frame_time = if let Some(last) = app.last_frame_time {
                now.duration_since(last)
            } else {
                Duration::from_secs_f64(1.0 / 60.0)
            };
            app.last_frame_time = Some(now);"""
    
    new_frame_time = """let now_ms = web_sys::window()
                .and_then(|w| w.performance())
                .map(|p| p.now() as f64)
                .unwrap_or(0.0);
            let frame_time = if let Some(last_ms) = app.last_frame_time {
                let delta_ms = now_ms - last_ms;
                Duration::from_secs_f64(delta_ms / 1000.0)
            } else {
                Duration::from_secs_f64(1.0 / 60.0)
            };
            app.last_frame_time = Some(now_ms);"""
    
    content = content.replace(old_frame_time, new_frame_time)
    
    # 修改结构体中的类型
    content = content.replace(
        "last_frame_time: Option<Instant>,",
        "last_frame_time: Option<f64>,  // milliseconds"
    )
    
    # 修改初始化
    content = content.replace(
        "last_frame_time: None,",
        "last_frame_time: None,  // Performance.now() milliseconds"
    )
    
    with open(filepath, 'w', encoding='utf-8') as f:
        f.write(content)
    
    print("✓ 修复 src/app.rs - 使用 web_sys::Performance 替代 Instant")


def main():
    print("=" * 60)
    print("ConwayDuel 编译错误修复脚本")
    print("=" * 60)
    print()
    
    # 修复 config.rs
    config_path = os.path.join(ROOT_DIR, "src/config.rs")
    if os.path.exists(config_path):
        fix_config_rs()
    else:
        print("✗ src/config.rs 不存在")
    
    # 修复 app.rs
    app_path = os.path.join(ROOT_DIR, "src/app.rs")
    if os.path.exists(app_path):
        fix_app_rs()
    else:
        print("✗ src/app.rs 不存在")
    
    print()
    print("=" * 60)
    print("修复完成！请运行 cargo check 验证")
    print("=" * 60)


if __name__ == "__main__":
    main()
