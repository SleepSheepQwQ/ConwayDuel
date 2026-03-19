#!/usr/bin/env python3
"""
ConwayDuel 项目修复脚本
修复 hecs 0.10.x API 变更导致的编译错误

问题1: Component trait 不再需要手动实现
问题2: query_one API 变化
问题3: 借用冲突
"""

import os
import re

def fix_components_rs(filepath):
    """删除所有手动实现的 Component trait"""
    print(f"修复文件: {filepath}")
    
    with open(filepath, 'r', encoding='utf-8') as f:
        content = f.read()
    
    # 删除所有 unsafe impl hecs::Component for XXX {} 行
    # 匹配模式: unsafe impl hecs::Component for TypeName {}
    pattern = r'^unsafe impl hecs::Component for \w+ \{\}\s*\n'
    new_content = re.sub(pattern, '', content, flags=re.MULTILINE)
    
    with open(filepath, 'w', encoding='utf-8') as f:
        f.write(new_content)
    
    print(f"  - 已删除所有手动实现的 Component trait")

def fix_query_one_api(content):
    """修复 query_one API 调用
    
    旧API: world.query_one::<&T>(entity).get() 返回 Result
    新API: world.query_one::<&T>(entity) 返回 Result<QueryOne, NoSuchEntity>
           QueryOne.get() 返回 Option<Q::Item>
    
    修复策略:
    - .get().is_ok() -> .ok().map(|q| q.get()).flatten().is_some()
    - if let Ok(x) = ... .get() -> if let Some(x) = ... .ok().and_then(|q| q.get())
    - let x = ... .get().map().unwrap_or() -> ... .ok().and_then(|q| q.get()).map().unwrap_or()
    """
    
    # 修复模式1: world.query_one::<&TYPE>(entity).get().is_ok()
    # 改为: world.query_one::<&TYPE>(entity).ok().map(|q| q.get()).flatten().is_some()
    pattern1 = r'world\.query_one::<([^>]+)>\(([^)]+)\)\.get\(\)\.is_ok\(\)'
    replacement1 = r'world.query_one::<\1>(\2).ok().map(|q| q.get()).flatten().is_some()'
    content = re.sub(pattern1, replacement1, content)
    
    # 修复模式2: if let Ok(var) = world.query_one::<&TYPE>(entity).get() {
    # 改为: if let Some(var) = world.query_one::<&TYPE>(entity).ok().and_then(|q| q.get()) {
    pattern2 = r'if let Ok\(([^)]+)\) = world\.query_one::<([^>]+)>\(([^)]+)\)\.get\(\)'
    replacement2 = r'if let Some(\1) = world.query_one::<\2>(\3).ok().and_then(|q| q.get())'
    content = re.sub(pattern2, replacement2, content)
    
    # 修复模式3: world.query_one::<&TYPE>(entity).get().map(...).unwrap_or(...)
    # 改为: world.query_one::<&TYPE>(entity).ok().and_then(|q| q.get()).map(...).unwrap_or(...)
    # 这个需要多行匹配
    pattern3 = r'world\.query_one::<([^>]+)>\(([^)]+)\)\.get\(\)\s*\.map\('
    replacement3 = r'world.query_one::<\1>(\2).ok().and_then(|q| q.get()).map('
    content = re.sub(pattern3, replacement3, content)
    
    return content

def fix_render_mod_rs(filepath):
    """修复 render/mod.rs 中的借用冲突和 query_one API"""
    print(f"修复文件: {filepath}")
    
    with open(filepath, 'r', encoding='utf-8') as f:
        content = f.read()
    
    # 先修复 query_one API
    content = fix_query_one_api(content)
    
    # 修复借用冲突: render_nebula 方法
    # 问题: for pos in &self.nebula_positions { self.render_circle(...) }
    # 解决: 先收集位置，再渲染
    old_render_nebula = r'''// 渲染背景星云
    unsafe fn render_nebula\(&mut self\) \{
        for pos in &self\.nebula_positions \{
            let color = \[0\.1, 0\.1, 0\.2, 0\.3\];
            self\.render_circle\(\*pos, 3\.0, color\);
        \}
    \}'''
    
    new_render_nebula = '''// 渲染背景星云
    unsafe fn render_nebula(&mut self) {
        // 先收集位置避免借用冲突
        let positions: Vec<Vec2> = self.nebula_positions.clone();
        for pos in positions {
            let color = [0.1, 0.1, 0.2, 0.3];
            self.render_circle(pos, 3.0, color);
        }
    }'''
    
    content = re.sub(old_render_nebula, new_render_nebula, content)
    
    # 修复不必要的 unsafe 块 (可选，只是警告)
    # content = content.replace(
    #     'let gl = unsafe { glow::Context::from_webgl2_context(gl) };',
    #     'let gl = glow::Context::from_webgl2_context(gl);'
    # )
    
    with open(filepath, 'w', encoding='utf-8') as f:
        f.write(content)
    
    print(f"  - 已修复 query_one API 调用")
    print(f"  - 已修复借用冲突")

def fix_combat_mod_rs(filepath):
    """修复 combat/mod.rs 中的 query_one API"""
    print(f"修复文件: {filepath}")
    
    with open(filepath, 'r', encoding='utf-8') as f:
        content = f.read()
    
    content = fix_query_one_api(content)
    
    with open(filepath, 'w', encoding='utf-8') as f:
        f.write(content)
    
    print(f"  - 已修复 query_one API 调用")

def fix_physics_mod_rs(filepath):
    """修复 physics/mod.rs 中的 query_one API"""
    print(f"修复文件: {filepath}")
    
    with open(filepath, 'r', encoding='utf-8') as f:
        content = f.read()
    
    content = fix_query_one_api(content)
    
    with open(filepath, 'w', encoding='utf-8') as f:
        f.write(content)
    
    print(f"  - 已修复 query_one API 调用")

def fix_app_rs(filepath):
    """修复 app.rs 中的警告（可选）"""
    print(f"修复文件: {filepath}")
    
    with open(filepath, 'r', encoding='utf-8') as f:
        content = f.read()
    
    # 修复不必要的括号
    content = content.replace(
        '(nanos as f32 / u32::MAX as f32)',
        'nanos as f32 / u32::MAX as f32'
    )
    
    with open(filepath, 'w', encoding='utf-8') as f:
        f.write(content)
    
    print(f"  - 已修复不必要的括号")

def fix_ai_mod_rs(filepath):
    """修复 ai/mod.rs 中的警告（可选）"""
    print(f"修复文件: {filepath}")
    
    with open(filepath, 'r', encoding='utf-8') as f:
        content = f.read()
    
    # 修复不必要的括号
    content = content.replace(
        '(nanos as f32 / u32::MAX as f32)',
        'nanos as f32 / u32::MAX as f32'
    )
    
    with open(filepath, 'w', encoding='utf-8') as f:
        f.write(content)
    
    print(f"  - 已修复不必要的括号")

def main():
    print("=" * 60)
    print("ConwayDuel 项目修复脚本")
    print("修复 hecs 0.10.x API 变更导致的编译错误")
    print("=" * 60)
    print()
    
    # 获取项目根目录（脚本所在目录）
    script_dir = os.path.dirname(os.path.abspath(__file__))
    
    # 定义需要修复的文件
    files_to_fix = [
        ('src/ecs/components.rs', fix_components_rs),
        ('src/core/render/mod.rs', fix_render_mod_rs),
        ('src/core/combat/mod.rs', fix_combat_mod_rs),
        ('src/core/physics/mod.rs', fix_physics_mod_rs),
        ('src/app.rs', fix_app_rs),
        ('src/core/ai/mod.rs', fix_ai_mod_rs),
    ]
    
    for relative_path, fix_func in files_to_fix:
        filepath = os.path.join(script_dir, relative_path)
        if os.path.exists(filepath):
            fix_func(filepath)
        else:
            print(f"警告: 文件不存在 - {filepath}")
    
    print()
    print("=" * 60)
    print("修复完成！")
    print("请运行 'cargo check' 验证修复结果")
    print("=" * 60)

if __name__ == '__main__':
    main()
