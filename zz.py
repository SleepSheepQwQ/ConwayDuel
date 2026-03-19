#!/usr/bin/env python3
"""
ConwayDuel 项目完整修复脚本 v3
直接重写有问题的文件，确保修复成功
"""

import os

# ============== src/ecs/components.rs ==============
COMPONENTS_RS = '''use glam::Vec2;
use std::time::Duration;
use crate::config::{Faction, GameConfig};

/// 变换组件：位置、旋转、缩放
#[derive(Debug, Clone, Copy)]
pub struct Transform {
    pub position: Vec2,
    pub rotation: f32,
    pub scale: Vec2,
}

/// 速度组件：线速度、角速度、最大速度
#[derive(Debug, Clone, Copy)]
pub struct Velocity {
    pub linear: Vec2,
    pub angular: f32,
    pub max_speed: f32,
}

impl Default for Velocity {
    fn default() -> Self {
        Self {
            linear: Vec2::ZERO,
            angular: 0.0,
            max_speed: 8.0,
        }
    }
}

/// 生命值组件
#[derive(Debug, Clone, Copy)]
pub struct Health {
    pub current: f32,
    pub max: f32,
    pub is_dead: bool,
}

impl Health {
    pub fn new(max: f32) -> Self {
        Self {
            current: max,
            max,
            is_dead: false,
        }
    }

    pub fn take_damage(&mut self, damage: f32) {
        self.current = (self.current - damage).max(0.0);
        if self.current <= 0.0 {
            self.is_dead = true;
        }
    }

    pub fn heal(&mut self, amount: f32) {
        self.current = (self.current + amount).min(self.max);
        if self.current > 0.0 {
            self.is_dead = false;
        }
    }
}

/// 阵营组件
#[derive(Debug, Clone, Copy)]
pub struct FactionComponent {
    pub faction: Faction,
}

/// 武器组件
#[derive(Debug, Clone)]
pub struct Weapon {
    pub fire_cooldown: Duration,
    pub cooldown_remaining: Duration,
    pub bullet_speed: f32,
    pub bullet_damage: f32,
    pub bullet_lifetime: Duration,
}

impl Weapon {
    pub fn from_config(config: &GameConfig) -> Self {
        Self {
            fire_cooldown: Duration::from_secs_f32(config.fire_cooldown),
            cooldown_remaining: Duration::ZERO,
            bullet_speed: config.ship_max_speed * config.bullet_speed_multiplier,
            bullet_damage: config.bullet_damage,
            bullet_lifetime: Duration::from_secs_f32(config.bullet_lifetime),
        }
    }

    pub fn can_fire(&self) -> bool {
        self.cooldown_remaining.is_zero()
    }

    pub fn fire(&mut self) {
        self.cooldown_remaining = self.fire_cooldown;
    }

    pub fn update(&mut self, dt: Duration) {
        self.cooldown_remaining = self.cooldown_remaining.saturating_sub(dt);
    }
}

/// 碰撞层
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollisionLayer {
    Ship,
    Bullet,
    Effect,
}

impl CollisionLayer {
    pub fn can_collide_with(&self, other: &Self) -> bool {
        match (self, other) {
            (CollisionLayer::Ship, CollisionLayer::Bullet) => true,
            (CollisionLayer::Bullet, CollisionLayer::Ship) => true,
            (CollisionLayer::Ship, CollisionLayer::Ship) => true,
            _ => false,
        }
    }
}

/// 碰撞体组件
#[derive(Debug, Clone, Copy)]
pub struct Collider {
    pub radius: f32,
    pub layer: CollisionLayer,
}

/// 子弹组件
#[derive(Debug, Clone, Copy)]
pub struct Bullet {
    pub shooter: hecs::Entity,
    pub lifetime: Duration,
    pub damage: f32,
}

/// 渲染层级
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RenderLayer {
    Background = 0,
    Boundary = 1,
    Nebula = 2,
    Bullet = 3,
    Ship = 4,
    Effect = 5,
}

/// 可渲染组件
#[derive(Debug, Clone, Copy)]
pub struct Renderable {
    pub color: [f32; 4],
    pub layer: RenderLayer,
    pub visible: bool,
}

/// AI行为状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiBehaviorState {
    Idle,
    Seeking,
    Chasing,
    Attacking,
    Retreating,
}

/// AI状态组件
#[derive(Debug, Clone)]
pub struct AiState {
    pub current_state: AiBehaviorState,
    pub target: Option<hecs::Entity>,
    pub target_lock_timer: Duration,
}

impl Default for AiState {
    fn default() -> Self {
        Self {
            current_state: AiBehaviorState::Idle,
            target: None,
            target_lock_timer: Duration::ZERO,
        }
    }
}

/// 特效组件
#[derive(Debug, Clone, Copy)]
pub struct Effect {
    pub lifetime: Duration,
    pub max_lifetime: Duration,
    pub start_scale: f32,
    pub end_scale: f32,
}

/// 重生计时器组件
#[derive(Debug, Clone, Copy)]
pub struct RespawnTimer {
    pub remaining: Duration,
}

impl RespawnTimer {
    pub fn new(delay: Duration) -> Self {
        Self { remaining: delay }
    }
}
'''

# ============== src/ecs/mod.rs ==============
ECS_MOD_RS = '''pub mod components;
pub mod events;
'''

# ============== src/core/mod.rs ==============
CORE_MOD_RS = '''// 核心业务模块总入口，声明所有子模块，统一导出供全局使用
pub mod ai;
pub mod combat;
pub mod physics;
pub mod render;
pub mod modes;
pub mod skills;
'''

# ============== src/core/combat/mod.rs ==============
COMBAT_MOD_RS = '''use glam::Vec2;
use hecs::World;
use std::time::Duration;

use crate::config::{Faction, GameConfig};
use crate::ecs::components::*;
use crate::ecs::events::{EventBus, GameEvent};

/// 武器系统：处理飞船射击逻辑
pub fn weapon_system(world: &mut World, dt: Duration, _event_bus: &mut EventBus, config: &GameConfig) {
    // 收集射击信息
    let mut bullets_to_spawn = Vec::new();

    for (entity, (transform, _velocity, weapon, ai_state, faction)) in world.query::<(
        &Transform,
        &Velocity,
        &Weapon,
        &AiState,
        &FactionComponent,
    )>()
    .into_iter()
    {
        // 更新武器冷却
        let weapon = weapon;
        let can_fire = weapon.cooldown_remaining.is_zero();
        
        // 只有攻击状态且有目标才射击
        if ai_state.current_state != AiBehaviorState::Attacking {
            continue;
        }

        if let Some(target) = ai_state.target {
            if let Some(target_transform) = world.query_one::<&Transform>(target).ok().and_then(|mut q| q.get()) {
                let direction = (target_transform.position - transform.position).normalize_or_zero();
                
                // 检查是否可以射击
                if can_fire {
                    bullets_to_spawn.push((
                        transform.position,
                        direction,
                        entity,
                        faction.faction,
                        weapon.bullet_speed,
                        weapon.bullet_damage,
                        weapon.bullet_lifetime,
                    ));
                }
            }
        }
    }

    // 更新武器冷却
    for (_, weapon) in world.query::<&mut Weapon>().into_iter() {
        weapon.update(dt);
    }

    // 生成子弹
    for (position, direction, shooter, faction, speed, damage, lifetime) in bullets_to_spawn {
        spawn_bullet(world, config, position, direction, shooter, faction, speed, damage, lifetime);
    }
}

/// 生成子弹
fn spawn_bullet(
    world: &mut World,
    config: &GameConfig,
    position: Vec2,
    direction: Vec2,
    shooter: hecs::Entity,
    faction: Faction,
    speed: f32,
    damage: f32,
    lifetime: Duration,
) {
    let transform = Transform {
        position,
        rotation: direction.y.atan2(direction.x),
        scale: Vec2::splat(config.bullet_size),
    };

    let velocity = Velocity {
        linear: direction * speed,
        angular: 0.0,
        max_speed: speed,
    };

    let bullet = Bullet {
        shooter,
        lifetime,
        damage,
    };

    let collider = Collider {
        radius: config.bullet_size,
        layer: CollisionLayer::Bullet,
    };

    let renderable = Renderable {
        color: faction.to_color(),
        layer: RenderLayer::Bullet,
        visible: true,
    };

    world.spawn((transform, velocity, bullet, collider, renderable));
}

/// 伤害系统：处理命中伤害
pub fn damage_system(world: &mut World, event_bus: &mut EventBus) {
    // 收集伤害事件
    let mut damage_events = Vec::new();
    for event in event_bus.events() {
        if let GameEvent::Hit { target, damage, .. } = event {
            damage_events.push((*target, *damage));
        }
    }

    // 应用伤害
    for (target, damage) in damage_events {
        if let Ok(health) = world.query_one_mut::<&mut Health>(target) {
            health.take_damage(damage);
        }
    }

    // 处理死亡
    let mut death_events = Vec::new();
    for (entity, (health, transform, faction)) in world.query::<(&Health, &Transform, &FactionComponent)>().into_iter() {
        if health.is_dead {
            death_events.push((entity, transform.position, faction.faction));
        }
    }

    // 发布死亡事件
    for (entity, position, faction) in death_events {
        event_bus.publish(GameEvent::Death { position, faction });
        let _ = world.despawn(entity);
    }
}

/// 生成爆炸特效
pub fn spawn_explosion(world: &mut World, position: &Vec2, faction: Faction, config: &GameConfig) {
    let transform = Transform {
        position: *position,
        rotation: 0.0,
        scale: Vec2::splat(config.ship_size),
    };

    let effect = Effect {
        lifetime: Duration::from_secs_f32(0.5),
        max_lifetime: Duration::from_secs_f32(0.5),
        start_scale: config.ship_size * 0.5,
        end_scale: config.ship_size * 2.0,
    };

    let renderable = Renderable {
        color: faction.to_color(),
        layer: RenderLayer::Effect,
        visible: true,
    };

    world.spawn((transform, effect, renderable));
}

/// 清理系统：移除死亡实体和过期特效
pub fn cleanup_system(world: &mut World, dt: Duration) {
    // 收集需要移除的实体
    let mut to_remove = Vec::new();

    // 移除过期特效
    for (entity, effect) in world.query::<&mut Effect>().into_iter() {
        let effect = effect;
        let new_lifetime = effect.lifetime.saturating_sub(dt);
        if new_lifetime.is_zero() {
            to_remove.push(entity);
        }
    }
    
    // 更新特效生命周期
    for (entity, effect) in world.query::<&mut Effect>().into_iter() {
        effect.lifetime = effect.lifetime.saturating_sub(dt);
    }

    // 移除过期子弹
    for (entity, bullet) in world.query::<&mut Bullet>().into_iter() {
        let bullet = bullet;
        let new_lifetime = bullet.lifetime.saturating_sub(dt);
        if new_lifetime.is_zero() {
            to_remove.push(entity);
        }
    }
    
    // 更新子弹生命周期
    for (entity, bullet) in world.query::<&mut Bullet>().into_iter() {
        bullet.lifetime = bullet.lifetime.saturating_sub(dt);
    }

    // 移除实体
    for entity in to_remove {
        let _ = world.despawn(entity);
    }
}
'''

# ============== src/core/physics/mod.rs ==============
PHYSICS_MOD_RS = '''use glam::Vec2;
use hecs::World;
use std::time::Duration;

use crate::config::GameConfig;
use crate::ecs::components::*;
use crate::ecs::events::{EventBus, GameEvent};
use crate::config::Faction;

// 移动系统：更新所有实体的位置、旋转，限制最大速度
pub fn movement_system(world: &mut World, dt: Duration) {
    let dt_secs = dt.as_secs_f32();

    for (_, (transform, velocity)) in world.query::<(&mut Transform, &mut Velocity)>().into_iter() {
        // 更新位置
        transform.position += velocity.linear * dt_secs;
        // 更新旋转，归一化角度避免溢出
        transform.rotation += velocity.angular * dt_secs;
        transform.rotation = transform.rotation.rem_euclid(std::f32::consts::TAU);
        // 限制最大速度，防止速度失控
        if velocity.linear.length_squared() > velocity.max_speed.powi(2) {
            velocity.linear = velocity.linear.normalize_or_zero() * velocity.max_speed;
        }
    }
}

// 边界系统：处理实体与战场边界的碰撞反弹，销毁出界子弹
pub fn boundary_system(world: &mut World, event_bus: &mut EventBus, config: &GameConfig) {
    let bounds_min = Vec2::ZERO;
    let bounds_max = Vec2::new(config.world_width, config.world_height);

    // 处理飞船的边界碰撞与反弹
    // 先收集所有需要处理的数据
    let mut boundary_collisions: Vec<(hecs::Entity, Vec2, Vec2)> = Vec::new();
    
    for (entity, (transform, velocity, collider, _faction)) in world.query::<(
        &Transform,
        &Velocity,
        &Collider,
        &FactionComponent,
    )>().into_iter()
    {
        let mut collision_normal = Vec2::ZERO;
        let radius = collider.radius;
        let mut new_pos = transform.position;

        // 左右边界检测
        if transform.position.x - radius < bounds_min.x {
            collision_normal.x = 1.0;
            new_pos.x = bounds_min.x + radius;
        } else if transform.position.x + radius > bounds_max.x {
            collision_normal.x = -1.0;
            new_pos.x = bounds_max.x - radius;
        }

        // 上下边界检测
        if transform.position.y - radius < bounds_min.y {
            collision_normal.y = 1.0;
            new_pos.y = bounds_min.y + radius;
        } else if transform.position.y + radius > bounds_max.y {
            collision_normal.y = -1.0;
            new_pos.y = bounds_max.y - radius;
        }

        if collision_normal != Vec2::ZERO {
            boundary_collisions.push((entity, collision_normal, new_pos));
        }
    }

    // 应用边界碰撞结果
    for (entity, collision_normal, new_pos) in boundary_collisions {
        if let Ok((transform, velocity)) = world.query_one_mut::<(&mut Transform, &mut Velocity)>(entity) {
            transform.position = new_pos;
            // 速度沿法线反射，实现反弹
            velocity.linear = velocity.linear - 2.0 * velocity.linear.dot(collision_normal) * collision_normal;
            // 应用阻尼，减少能量损耗，避免无限反弹
            velocity.linear *= config.ship_bounce_damping;

            // 发布边界碰撞事件
            event_bus.publish(GameEvent::BoundaryCollision {
                entity,
                normal: collision_normal,
            });
        }
    }

    // 销毁出界的子弹，避免内存泄漏
    let mut out_of_bounds_bullets = Vec::new();
    for (entity, (transform, _bullet)) in world.query::<(&Transform, &Bullet)>().into_iter()
    {
        let pos = transform.position;
        if pos.x < bounds_min.x || pos.x > bounds_max.x
            || pos.y < bounds_min.y || pos.y > bounds_max.y
        {
            out_of_bounds_bullets.push(entity);
        }
    }

    for entity in out_of_bounds_bullets {
        let _ = world.despawn(entity);
    }
}

// 碰撞系统：检测实体间的碰撞，处理子弹命中、飞船间碰撞
pub fn collision_system(world: &mut World, event_bus: &mut EventBus, config: &GameConfig) {
    // 收集所有碰撞体信息，避免迭代中修改世界
    // 使用 into_iter() 避免生命周期问题
    let mut colliders: Vec<(hecs::Entity, Vec2, Collider, Option<Faction>)> = Vec::new();
    for (entity, (transform, collider, faction)) in world.query::<(&Transform, &Collider, Option<&FactionComponent>)>().into_iter() {
        let faction_val = faction.map(|f| f.faction);
        colliders.push((entity, transform.position, *collider, faction_val));
    }

    // 收集需要处理的事件，避免在迭代中修改世界
    let mut hits_to_process: Vec<(hecs::Entity, hecs::Entity, f32, Vec2)> = Vec::new();
    let mut bullets_to_despawn: Vec<hecs::Entity> = Vec::new();
    let mut ship_collisions: Vec<(hecs::Entity, hecs::Entity)> = Vec::new();

    // 两两碰撞检测，实体数量少，无性能压力
    for i in 0..colliders.len() {
        let (entity_a, pos_a, collider_a, faction_a) = colliders[i];
        for j in (i + 1)..colliders.len() {
            let (entity_b, pos_b, collider_b, faction_b) = colliders[j];

            // 先判断碰撞层级是否允许碰撞
            if !collider_a.layer.can_collide_with(&collider_b.layer) {
                continue;
            }

            // 飞船间碰撞，检查配置是否开启
            if collider_a.layer == CollisionLayer::Ship && collider_b.layer == CollisionLayer::Ship {
                if !config.ship_ship_collision_enabled {
                    continue;
                }
            }

            // 子弹与飞船碰撞，禁止友军伤害
            if (collider_a.layer == CollisionLayer::Bullet && collider_b.layer == CollisionLayer::Ship)
                || (collider_a.layer == CollisionLayer::Ship && collider_b.layer == CollisionLayer::Bullet)
            {
                if let (Some(fa), Some(fb)) = (faction_a, faction_b) {
                    if !fa.is_enemy(&fb) {
                        continue;
                    }
                }
            }

            // 圆形碰撞检测，计算距离
            let distance = pos_a.distance(pos_b);
            let min_collision_distance = collider_a.radius + collider_b.radius + config.collision_margin;

            // 发生碰撞
            if distance < min_collision_distance {
                // 子弹命中飞船
                if collider_a.layer == CollisionLayer::Bullet && collider_b.layer == CollisionLayer::Ship {
                    // 获取子弹信息
                    if let Some(bullet) = world.query_one::<&Bullet>(entity_a).ok().and_then(|mut q| q.get()) {
                        let damage = world.query_one::<&Weapon>(bullet.shooter).ok().and_then(|mut q| q.get())
                            .map(|w| w.bullet_damage)
                            .unwrap_or(config.bullet_damage);
                        hits_to_process.push((bullet.shooter, entity_b, damage, pos_a));
                        bullets_to_despawn.push(entity_a);
                    }
                }
                // 飞船被子弹命中
                else if collider_a.layer == CollisionLayer::Ship && collider_b.layer == CollisionLayer::Bullet {
                    if let Some(bullet) = world.query_one::<&Bullet>(entity_b).ok().and_then(|mut q| q.get()) {
                        let damage = world.query_one::<&Weapon>(bullet.shooter).ok().and_then(|mut q| q.get())
                            .map(|w| w.bullet_damage)
                            .unwrap_or(config.bullet_damage);
                        hits_to_process.push((bullet.shooter, entity_a, damage, pos_b));
                        bullets_to_despawn.push(entity_b);
                    }
                }
                // 飞船间碰撞
                else if collider_a.layer == CollisionLayer::Ship && collider_b.layer == CollisionLayer::Ship {
                    ship_collisions.push((entity_a, entity_b));
                }
            }
        }
    }

    // 处理命中事件
    for (attacker, target, damage, position) in hits_to_process {
        event_bus.publish(GameEvent::Hit {
            attacker,
            target,
            damage,
            position,
        });
    }

    // 销毁命中的子弹
    for entity in bullets_to_despawn {
        let _ = world.despawn(entity);
    }

    // 处理飞船间碰撞
    for (entity_a, entity_b) in ship_collisions {
        // 获取位置计算碰撞法线
        let pos_a = world.query_one::<&Transform>(entity_a).ok().and_then(|mut q| q.get())
            .map(|t| t.position)
            .unwrap_or(Vec2::ZERO);
        let pos_b = world.query_one::<&Transform>(entity_b).ok().and_then(|mut q| q.get())
            .map(|t| t.position)
            .unwrap_or(Vec2::ZERO);
        let collision_normal = (pos_a - pos_b).normalize_or_zero();
        
        // 给两个飞船施加反向的速度
        if let Ok(vel_a) = world.query_one_mut::<&mut Velocity>(entity_a) {
            vel_a.linear += collision_normal * 0.5;
        }
        if let Ok(vel_b) = world.query_one_mut::<&mut Velocity>(entity_b) {
            vel_b.linear -= collision_normal * 0.5;
        }
    }
}
'''

# ============== src/core/ai/mod.rs ==============
AI_MOD_RS = '''use glam::Vec2;
use hecs::World;
use std::time::Duration;

use crate::config::GameConfig;
use crate::ecs::components::*;

// AI系统主入口，每一固定帧更新所有飞船的AI行为
pub fn ai_system(world: &mut World, dt: Duration, config: &GameConfig) {
    // 先收集所有存活飞船的信息，用于索敌逻辑
    let mut all_ships = Vec::new();
    for (entity, (transform, health, faction)) in world.query::<(&Transform, &Health, &FactionComponent)>()
        .into_iter()
    {
        if !health.is_dead {
            all_ships.push((entity, transform.position, faction.faction, health.current));
        }
    }

    // 遍历每个AI飞船，更新状态机和行为
    for (entity, (transform, velocity, ai_state, health, faction)) in world.query::<(
        &Transform,
        &mut Velocity,
        &mut AiState,
        &Health,
        &FactionComponent,
    )>()
    .into_iter()
    {
        if health.is_dead {
            continue;
        }

        // 更新目标锁定计时器，防止目标频繁切换
        if ai_state.target_lock_timer > Duration::ZERO {
            ai_state.target_lock_timer = ai_state.target_lock_timer.saturating_sub(dt);
        }

        // ========== 索敌逻辑：找到最优攻击目标 ==========
        let mut best_target = None;
        let mut best_score = f32::INFINITY;
        for (target_entity, target_pos, target_faction, target_health) in &all_ships {
            // 跳过自己和友军
            if *target_entity == entity || !faction.faction.is_enemy(target_faction) {
                continue;
            }
            // 超出视野范围，跳过
            let distance = transform.position.distance(*target_pos);
            if distance > config.ai_view_range {
                continue;
            }
            // 计算目标评分：距离越近、血量越低，评分越高（数值越小）
            let score = distance + (target_health / health.max) * 10.0;
            if score < best_score {
                best_score = score;
                best_target = Some((*target_entity, *target_pos));
            }
        }

        // 更新锁定目标：只有锁定时间结束才会切换，防止目标频繁抖动
        if ai_state.target_lock_timer.is_zero() {
            ai_state.target = best_target.map(|(e, _)| e);
            if ai_state.target.is_some() {
                ai_state.target_lock_timer = Duration::from_secs_f32(config.ai_target_lock_time);
            }
        }

        // 获取当前目标的信息
        let target_info = ai_state.target.and_then(|e| {
            all_ships.iter().find(|(ent, _, _, _)| *ent == e)
        });

        // ========== 状态机切换逻辑 ==========
        let health_percent = health.current / health.max;
        // 血量低于阈值，进入撤退状态
        if health_percent <= config.ai_evade_threshold {
            ai_state.current_state = AiBehaviorState::Retreating;
        }
        // 有目标，进入攻击/追击状态
        else if let Some((_, target_pos, _, _)) = target_info {
            let distance = transform.position.distance(*target_pos);
            ai_state.current_state = if distance <= config.ai_attack_range {
                AiBehaviorState::Attacking
            } else {
                AiBehaviorState::Chasing
            };
        }
        // 无目标，进入索敌状态
        else {
            ai_state.current_state = AiBehaviorState::Seeking;
        }

        // ========== 行为执行逻辑 ==========
        match ai_state.current_state {
            AiBehaviorState::Idle => {
                // 空闲状态：缓慢随机移动
                velocity.linear *= 0.95;
            }
            AiBehaviorState::Seeking => {
                // 索敌状态：随机巡逻
                let wander_angle = rand_random() * std::f32::consts::TAU;
                velocity.linear = Vec2::new(wander_angle.cos(), wander_angle.sin()) * config.ship_max_speed * 0.3;
            }
            AiBehaviorState::Chasing => {
                // 追击状态：朝目标移动
                if let Some((_, target_pos, _, _)) = target_info {
                    let direction = (*target_pos - transform.position).normalize_or_zero();
                    velocity.linear = direction * config.ship_max_speed;
                }
            }
            AiBehaviorState::Attacking => {
                // 攻击状态：保持距离并瞄准
                if let Some((_, target_pos, _, _)) = target_info {
                    let direction = (*target_pos - transform.position).normalize_or_zero();
                    let distance = transform.position.distance(*target_pos);
                    
                    if distance < config.ai_attack_range * 0.5 {
                        // 太近，后退
                        velocity.linear = -direction * config.ship_max_speed * 0.3;
                    } else {
                        // 绕目标旋转
                        let perpendicular = Vec2::new(-direction.y, direction.x);
                        velocity.linear = perpendicular * config.ship_max_speed * 0.5;
                    }
                }
            }
            AiBehaviorState::Retreating => {
                // 撤退状态：远离最近的敌人
                let mut nearest_enemy = None;
                let mut nearest_distance = f32::INFINITY;

                for (target_entity, target_pos, target_faction, _) in &all_ships {
                    if *target_entity == entity || !faction.faction.is_enemy(target_faction) {
                        continue;
                    }
                    let distance = transform.position.distance(*target_pos);
                    if distance < nearest_distance {
                        nearest_distance = distance;
                        nearest_enemy = Some(*target_pos);
                    }
                }

                if let Some(enemy_pos) = nearest_enemy {
                    let away_direction = (transform.position - enemy_pos).normalize_or_zero();
                    velocity.linear = away_direction * config.ship_max_speed;
                }
            }
        }
    }
}

/// 简单的随机数生成
fn rand_random() -> f32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();
    nanos as f32 / u32::MAX as f32
}
'''

# ============== src/core/render/mod.rs (关键部分) ==============
RENDER_MOD_RS_PART1 = '''use glow::HasContext;
use glam::{Mat4, Vec2};
use hecs::World;
use std::mem;
use wasm_bindgen::JsCast;

use crate::config::GameConfig;
use crate::ecs::components::*;

// 基础几何渲染顶点着色器
const BASIC_VERT: &str = r#"
    #version 300 es
    precision highp float;

    layout (location = 0) in vec2 a_position;
    uniform mat4 u_view_proj;
    uniform vec2 u_offset;
    uniform float u_scale;
    uniform vec4 u_color;

    out vec4 v_color;

    void main() {
        vec2 pos = a_position * u_scale + u_offset;
        gl_Position = u_view_proj * vec4(pos, 0.0, 1.0);
        v_color = u_color;
    }
"#;

// 基础纯色渲染片段着色器
const BASIC_FRAG: &str = r#"
    #version 300 es
    precision highp float;

    in vec4 v_color;
    out vec4 out_color;

    void main() {
        out_color = v_color;
    }
"#;

// 高斯模糊顶点着色器（背景特效用）
const GAUSSIAN_BLUR_VERT: &str = r#"
    #version 300 es
    precision highp float;

    layout (location = 0) in vec2 a_position;
    out vec2 v_uv;

    void main() {
        v_uv = a_position * 0.5 + 0.5;
        gl_Position = vec4(a_position, 0.0, 1.0);
    }
"#;

// 高斯模糊片段着色器
const GAUSSIAN_BLUR_FRAG: &str = r#"
    #version 300 es
    precision highp float;

    in vec2 v_uv;
    out vec4 out_color;

    uniform sampler2D u_texture;
    uniform vec2 u_dir;
    uniform int u_radius;
    uniform vec2 u_resolution;

    void main() {
        vec2 texel_size = 1.0 / u_resolution;
        vec4 color = vec4(0.0);
        float total = 0.0;

        for (int i = -u_radius; i <= u_radius; i++) {
            float weight = exp(-float(i * i) / (2.0 * float(u_radius) / 2.0));
            color += texture(u_texture, v_uv + vec2(i) * u_dir * texel_size) * weight;
            total += weight;
        }

        out_color = color / total;
    }
"#;

// 渲染器核心结构体
pub struct Renderer {
    gl: glow::Context,
    canvas: web_sys::HtmlCanvasElement,
    // 着色器程序
    basic_program: glow::Program,
    blur_program: glow::Program,
    // 顶点缓冲区
    quad_vao: glow::VertexArray,
    quad_vbo: glow::Buffer,
    // 飞船等腰三角形顶点数据
    ship_vertices: Vec<Vec2>,
    // 高斯模糊帧缓冲区
    blur_fbo: glow::Framebuffer,
    blur_texture: glow::Texture,
    // 渲染状态
    screen_width: i32,
    screen_height: i32,
    config: GameConfig,
    // 相机视图投影矩阵
    view_proj: Mat4,
    // 星云数据
    nebula_positions: Vec<Vec2>,
    // WebGL上下文丢失标记
    context_lost: bool,
}

impl Renderer {
    // 初始化渲染器，创建WebGL上下文、编译着色器
    pub fn new(canvas: web_sys::HtmlCanvasElement, config: &GameConfig) -> Result<Self, String> {
        // 获取WebGL2上下文，兼容安卓99%以上设备
        let gl = canvas
            .get_context("webgl2")
            .map_err(|_| "无法获取WebGL2上下文".to_string())?
            .ok_or("当前设备不支持WebGL2".to_string())?
            .dyn_into::<web_sys::WebGl2RenderingContext>()
            .map_err(|_| "WebGL2上下文类型转换失败".to_string())?;

        let gl = glow::Context::from_webgl2_context(gl);

        // 编译着色器程序
        let basic_program = compile_program(&gl, BASIC_VERT, BASIC_FRAG)?;
        let blur_program = compile_program(&gl, GAUSSIAN_BLUR_VERT, GAUSSIAN_BLUR_FRAG)?;

        // 创建全屏四边形VAO（用于模糊渲染）
        let quad_vertices: [f32; 12] = [
            -1.0, -1.0, 1.0, -1.0, 1.0, 1.0,
            -1.0, -1.0, 1.0, 1.0, -1.0, 1.0,
        ];

        unsafe {
            let quad_vao = gl.create_vertex_array()?;
            let quad_vbo = gl.create_buffer()?;

            gl.bind_vertex_array(Some(quad_vao));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(quad_vbo));
            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                &quad_vertices.align_to::<u8>().1,
                glow::STATIC_DRAW,
            );

            gl.vertex_attrib_pointer_f32(
                0,
                2,
                glow::FLOAT,
                false,
                2 * mem::size_of::<f32>() as i32,
                0,
            );
            gl.enable_vertex_attrib_array(0);

            gl.bind_vertex_array(None);
            gl.bind_buffer(glow::ARRAY_BUFFER, None);

            // 创建模糊帧缓冲区和纹理
            let blur_fbo = gl.create_framebuffer()?;
            let blur_texture = gl.create_texture()?;

            // 生成等腰三角形飞船顶点（纸飞机样式，机头朝前）
            let ship_size = config.ship_size;
            let ship_vertices = vec![
                Vec2::new(ship_size, 0.0),         // 机头
                Vec2::new(-ship_size / 2.0, ship_size / 2.0), // 机尾左上
                Vec2::new(-ship_size / 2.0, -ship_size / 2.0), // 机尾左下
            ];

            // 启用混合，处理透明度
            gl.enable(glow::BLEND);
            gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
            // 禁用深度测试，2D渲染不需要
            gl.disable(glow::DEPTH_TEST);

            // 初始化相机矩阵
            let view_proj = Mat4::orthographic_rh_gl(
                0.0,
                config.world_width,
                0.0,
                config.world_height,
                -1.0,
                1.0,
            );

            // 生成星云位置
            let mut nebula_positions = Vec::new();
            for i in 0..config.nebula_count {
                let x = ((i * 17 + 31) % 100) as f32 / 100.0 * config.world_width;
                let y = ((i * 23 + 47) % 100) as f32 / 100.0 * config.world_height;
                nebula_positions.push(Vec2::new(x, y));
            }

            Ok(Self {
                gl,
                canvas,
                basic_program,
                blur_program,
                quad_vao,
                quad_vbo,
                ship_vertices,
                blur_fbo,
                blur_texture,
                screen_width: 0,
                screen_height: 0,
                config: config.clone(),
                view_proj,
                nebula_positions,
                context_lost: false,
            })
        }
    }

    // 屏幕尺寸变化时更新，适配安卓旋转屏幕
    pub fn resize(&mut self, width: f32, height: f32, dpr: f32) {
        let physical_width = (width * dpr) as i32;
        let physical_height = (height * dpr) as i32;

        if self.screen_width == physical_width && self.screen_height == physical_height {
            return;
        }

        self.screen_width = physical_width;
        self.screen_height = physical_height;

        // 更新画布尺寸
        self.canvas.set_width(physical_width as u32);
        self.canvas.set_height(physical_height as u32);

        // 更新视口
        unsafe {
            self.gl.viewport(0, 0, physical_width, physical_height);

            // 更新模糊纹理尺寸
            self.gl.bind_texture(glow::TEXTURE_2D, Some(self.blur_texture));
            self.gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGBA as i32,
                physical_width,
                physical_height,
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                None,
            );
            self.gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MIN_FILTER,
                glow::LINEAR as i32,
            );
            self.gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MAG_FILTER,
                glow::LINEAR as i32,
            );
            self.gl.bind_texture(glow::TEXTURE_2D, None);
        }

        // 更新相机投影矩阵，适配屏幕宽高比
        let world_width = self.config.world_width;
        let world_height = self.config.world_height;
        let screen_aspect = width / height;
        let world_aspect = world_width / world_height;

        let (view_width, view_height) = if screen_aspect > world_aspect {
            (world_height * screen_aspect, world_height)
        } else {
            (world_width, world_width / screen_aspect)
        };

        self.view_proj = Mat4::orthographic_rh_gl(
            (world_width - view_width) / 2.0,
            (world_width + view_width) / 2.0,
            (world_height - view_height) / 2.0,
            (world_height + view_height) / 2.0,
            -1.0,
            1.0,
        );
    }
'''

RENDER_MOD_RS_PART2 = '''
    // 主渲染入口，按层级渲染所有游戏内容
    pub fn render(&mut self, world: &World, config: &GameConfig) {
        // 检查WebGL上下文是否丢失
        if self.context_lost {
            return;
        }

        unsafe {
            // 清空画布为深蓝色背景
            self.gl.clear_color(0.02, 0.02, 0.08, 1.0);
            self.gl.clear(glow::COLOR_BUFFER_BIT);

            // 渲染背景星云
            self.render_nebula();

            // 渲染战场边界
            self.render_boundary(config);

            // 收集所有可渲染实体，按层级排序
            // 使用 into_iter() 避免生命周期问题
            let mut renderables: Vec<(hecs::Entity, Transform, Renderable)> = Vec::new();
            for (entity, (transform, renderable)) in world.query::<(&Transform, &Renderable)>().into_iter() {
                if !renderable.visible {
                    continue;
                }
                renderables.push((entity, *transform, *renderable));
            }

            // 按渲染层级从后往前渲染，保证层级正确
            renderables.sort_by_key(|(_, _, r)| r.layer);

            // 遍历渲染所有实体
            for (entity, transform, renderable) in renderables {
                // 渲染飞船
                if world.query_one::<&FactionComponent>(entity).ok().map(|mut q| q.get()).flatten().is_some() {
                    self.render_ship(&transform, &renderable);
                }
                // 渲染子弹
                else if world.query_one::<&Bullet>(entity).ok().map(|mut q| q.get()).flatten().is_some() {
                    self.render_bullet(&transform, &renderable);
                }
                // 渲染爆炸特效
                else if let Some(effect) = world.query_one::<&Effect>(entity).ok().and_then(|mut q| q.get()) {
                    let progress = effect.lifetime.as_secs_f32() / effect.max_lifetime.as_secs_f32();
                    let current_scale = effect.start_scale + (effect.end_scale - effect.start_scale) * progress;
                    let mut color = renderable.color;
                    color[3] = 1.0 - progress; // 淡出效果
                    self.render_circle(transform.position, current_scale, color);
                }
            }
        }
    }

    // 渲染背景星云
    unsafe fn render_nebula(&mut self) {
        // 先收集位置避免借用冲突
        let positions: Vec<Vec2> = self.nebula_positions.clone();
        for pos in positions {
            let color = [0.1, 0.1, 0.2, 0.3];
            self.render_circle(pos, 3.0, color);
        }
    }
'''

def write_file(filepath, content):
    """写入文件"""
    os.makedirs(os.path.dirname(filepath), exist_ok=True)
    with open(filepath, 'w', encoding='utf-8') as f:
        f.write(content)
    print(f"  已写入: {filepath}")

def main():
    print("=" * 60)
    print("ConwayDuel 项目完整修复脚本 v3")
    print("直接重写有问题的文件")
    print("=" * 60)
    print()
    
    # 获取项目根目录
    script_dir = os.path.dirname(os.path.abspath(__file__))
    
    # 写入文件
    print("写入修复后的文件...")
    
    write_file(os.path.join(script_dir, 'src/ecs/components.rs'), COMPONENTS_RS)
    write_file(os.path.join(script_dir, 'src/ecs/mod.rs'), ECS_MOD_RS)
    write_file(os.path.join(script_dir, 'src/core/mod.rs'), CORE_MOD_RS)
    write_file(os.path.join(script_dir, 'src/core/combat/mod.rs'), COMBAT_MOD_RS)
    write_file(os.path.join(script_dir, 'src/core/physics/mod.rs'), PHYSICS_MOD_RS)
    write_file(os.path.join(script_dir, 'src/core/ai/mod.rs'), AI_MOD_RS)
    
    # render/mod.rs 需要特殊处理，只替换关键部分
    render_path = os.path.join(script_dir, 'src/core/render/mod.rs')
    if os.path.exists(render_path):
        with open(render_path, 'r', encoding='utf-8') as f:
            content = f.read()
        
        # 修复 query_one API
        content = content.replace('.ok().map(|q| q.get()).flatten()', '.ok().map(|mut q| q.get()).flatten()')
        content = content.replace('.ok().and_then(|q| q.get())', '.ok().and_then(|mut q| q.get())')
        
        # 修复 render 方法中的借用冲突
        # 替换 renderables 收集逻辑
        old_pattern = '''// 收集所有可渲染实体，按层级排序
            let mut renderables = Vec::new();
            for (entity, (transform, renderable)) in world.query::<(&Transform, &Renderable)>().iter() {
                if !renderable.visible {
                    continue;
                }
                renderables.push((entity, transform, renderable));
            }'''
        
        new_pattern = '''// 收集所有可渲染实体，按层级排序
            // 使用 into_iter() 避免生命周期问题
            let mut renderables: Vec<(hecs::Entity, Transform, Renderable)> = Vec::new();
            for (entity, (transform, renderable)) in world.query::<(&Transform, &Renderable)>().into_iter() {
                if !renderable.visible {
                    continue;
                }
                renderables.push((entity, *transform, *renderable));
            }'''
        
        content = content.replace(old_pattern, new_pattern)
        
        # 修复 render_nebula
        old_nebula = '''// 渲染背景星云
    unsafe fn render_nebula(&mut self) {
        for pos in &self.nebula_positions {
            let color = [0.1, 0.1, 0.2, 0.3];
            self.render_circle(*pos, 3.0, color);
        }
    }'''
        
        new_nebula = '''// 渲染背景星云
    unsafe fn render_nebula(&mut self) {
        // 先收集位置避免借用冲突
        let positions: Vec<Vec2> = self.nebula_positions.clone();
        for pos in positions {
            let color = [0.1, 0.1, 0.2, 0.3];
            self.render_circle(pos, 3.0, color);
        }
    }'''
        
        content = content.replace(old_nebula, new_nebula)
        
        # 修复不必要的 unsafe
        content = content.replace(
            'let gl = unsafe { glow::Context::from_webgl2_context(gl) };',
            'let gl = glow::Context::from_webgl2_context(gl);'
        )
        
        with open(render_path, 'w', encoding='utf-8') as f:
            f.write(content)
        print(f"  已修复: {render_path}")
    
    # 修复 app.rs
    app_path = os.path.join(script_dir, 'src/app.rs')
    if os.path.exists(app_path):
        with open(app_path, 'r', encoding='utf-8') as f:
            content = f.read()
        content = content.replace('(nanos as f32 / u32::MAX as f32)', 'nanos as f32 / u32::MAX as f32')
        with open(app_path, 'w', encoding='utf-8') as f:
            f.write(content)
        print(f"  已修复: {app_path}")
    
    print()
    print("=" * 60)
    print("修复完成！")
    print("请运行 'cargo check' 验证修复结果")
    print("=" * 60)

if __name__ == '__main__':
    main()
