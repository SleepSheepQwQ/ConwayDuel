#!/usr/bin/env python3
"""
ConwayDuel 最小化修复脚本 v3
严格遵循最小变更原则，只修复编译错误

修复内容：
1. src/ecs/components.rs - 添加 hecs::Component trait 实现
2. src/core/render/mod.rs - 修复 glow API 和 hecs query API
3. src/core/physics/mod.rs - 修复 hecs query API
4. src/core/combat/mod.rs - 修复 hecs query API
"""

import os
import re

ROOT_DIR = os.path.dirname(os.path.abspath(__file__))

def fix_components_rs():
    """修复 components.rs - 添加 Component trait 实现"""
    filepath = os.path.join(ROOT_DIR, "src/ecs/components.rs")
    
    with open(filepath, 'r', encoding='utf-8') as f:
        content = f.read()
    
    # 定义需要添加 Component trait 的结构体和枚举
    types_to_fix = [
        ("Transform", "struct"),
        ("Velocity", "struct"),
        ("Health", "struct"),
        ("FactionComponent", "struct"),
        ("Weapon", "struct"),
        ("CollisionLayer", "enum"),
        ("Collider", "struct"),
        ("Bullet", "struct"),
        ("RenderLayer", "enum"),
        ("Renderable", "struct"),
        ("AiBehaviorState", "enum"),
        ("AiState", "struct"),
        ("Effect", "struct"),
        ("RespawnTimer", "struct"),
    ]
    
    for type_name, type_kind in types_to_fix:
        # 查找类型定义的结束位置
        if type_kind == "struct":
            # 匹配 pub struct Name { ... }
            pattern = rf'(pub struct {type_name} \{{[^}}]*\}})'
        else:
            # 匹配 pub enum Name { ... }
            pattern = rf'(pub enum {type_name} \{{[^}}]*\}})'
        
        def add_impl(match):
            original = match.group(1)
            # 检查是否已经添加了 impl
            if f"unsafe impl hecs::Component for {type_name}" in content:
                return original
            return original + f"\n\nunsafe impl hecs::Component for {type_name} {{}}"
        
        content = re.sub(pattern, add_impl, content, count=1)
    
    with open(filepath, 'w', encoding='utf-8') as f:
        f.write(content)
    
    print("✓ 修复 src/ecs/components.rs - 添加 Component trait 实现")


def fix_render_mod_rs():
    """修复 render/mod.rs"""
    filepath = os.path.join(ROOT_DIR, "src/core/render/mod.rs")
    
    with open(filepath, 'r', encoding='utf-8') as f:
        content = f.read()
    
    # 1. 修复 glow::Context::from_webgl2
    content = content.replace(
        "let gl = glow::Context::from_webgl2(gl);",
        "let gl = unsafe { glow::Context::from_webgl2_context(gl) };"
    )
    
    # 2. 修复 world.get 调用
    replacements = [
        ("if world.get::<FactionComponent>(entity).is_ok() {",
         "if world.query_one::<&FactionComponent>(entity).get().is_ok() {"),
        ("else if world.get::<Bullet>(entity).is_ok() {",
         "else if world.query_one::<&Bullet>(entity).get().is_ok() {"),
        ("else if let Ok(effect) = world.get::<Effect>(entity) {",
         "else if let Ok(effect) = world.query_one::<&Effect>(entity).get() {"),
    ]
    
    for old, new in replacements:
        content = content.replace(old, new)
    
    with open(filepath, 'w', encoding='utf-8') as f:
        f.write(content)
    
    print("✓ 修复 src/core/render/mod.rs")


def fix_physics_mod_rs():
    """修复 physics/mod.rs"""
    filepath = os.path.join(ROOT_DIR, "src/core/physics/mod.rs")
    
    with open(filepath, 'r', encoding='utf-8') as f:
        content = f.read()
    
    # 1. 修复 .with::<FactionComponent>() 查询
    # 将 query::<(&mut Transform, &mut Velocity, &Collider)>().with::<FactionComponent>().iter()
    # 改为 query::<(&mut Transform, &mut Velocity, &Collider, &FactionComponent)>().iter()
    old1 = """for (entity, (transform, velocity, collider)) in world.query::<(&mut Transform, &mut Velocity, &Collider)>()
        .with::<FactionComponent>()
        .iter()"""
    new1 = """for (entity, (transform, velocity, collider, _faction)) in world.query::<(&mut Transform, &mut Velocity, &Collider, &FactionComponent)>().iter()"""
    content = content.replace(old1, new1)
    
    # 2. 修复 .with::<Bullet>() 查询
    old2 = """for (entity, transform) in world.query::<&Transform>()
        .with::<Bullet>()
        .iter()"""
    new2 = """for (entity, (transform, _bullet)) in world.query::<(&Transform, &Bullet)>().iter()"""
    content = content.replace(old2, new2)
    
    # 3. 修复 world.get 和 world.get_mut 调用
    replacements = [
        ("if let Ok(bullet) = world.get::<Bullet>(entity_a) {",
         "if let Ok(bullet) = world.query_one::<&Bullet>(entity_a).get() {"),
        ("let damage = world.get::<Weapon>(bullet.shooter)",
         "let damage = world.query_one::<&Weapon>(bullet.shooter).get()"),
        ("if let Ok(bullet) = world.get::<Bullet>(entity_b) {",
         "if let Ok(bullet) = world.query_one::<&Bullet>(entity_b).get() {"),
        ("if let Ok(mut vel_a) = world.get_mut::<Velocity>(entity_a) {",
         "if let Ok(mut vel_a) = world.query_one_mut::<&mut Velocity>(entity_a) {"),
        ("if let Ok(mut vel_b) = world.get_mut::<Velocity>(entity_b) {",
         "if let Ok(mut vel_b) = world.query_one_mut::<&mut Velocity>(entity_b) {"),
    ]
    
    for old, new in replacements:
        content = content.replace(old, new)
    
    with open(filepath, 'w', encoding='utf-8') as f:
        f.write(content)
    
    print("✓ 修复 src/core/physics/mod.rs")


def fix_combat_mod_rs():
    """修复 combat/mod.rs"""
    filepath = os.path.join(ROOT_DIR, "src/core/combat/mod.rs")
    
    with open(filepath, 'r', encoding='utf-8') as f:
        content = f.read()
    
    # 修复 world.get 和 world.get_mut 调用
    replacements = [
        ("if let Ok(target_transform) = world.get::<Transform>(target) {",
         "if let Ok(target_transform) = world.query_one::<&Transform>(target).get() {"),
        ("if let Ok(mut health) = world.get_mut::<Health>(target) {",
         "if let Ok(mut health) = world.query_one_mut::<&mut Health>(target) {"),
    ]
    
    for old, new in replacements:
        content = content.replace(old, new)
    
    with open(filepath, 'w', encoding='utf-8') as f:
        f.write(content)
    
    print("✓ 修复 src/core/combat/mod.rs")


def main():
    print("=" * 60)
    print("ConwayDuel 最小化修复脚本 v3")
    print("=" * 60)
    print()
    
    files_to_fix = [
        ("src/ecs/components.rs", fix_components_rs),
        ("src/core/render/mod.rs", fix_render_mod_rs),
        ("src/core/physics/mod.rs", fix_physics_mod_rs),
        ("src/core/combat/mod.rs", fix_combat_mod_rs),
    ]
    
    for rel_path, fix_func in files_to_fix:
        full_path = os.path.join(ROOT_DIR, rel_path)
        if os.path.exists(full_path):
            fix_func()
        else:
            print(f"✗ {rel_path} 不存在")
    
    print()
    print("=" * 60)
    print("修复完成！请运行以下命令验证：")
    print("  cargo check")
    print("=" * 60)


if __name__ == "__main__":
    main()
