use glam::Vec2;
use hecs::World;
use std::time::Duration;

use crate::config::GameConfig;
use crate::ecs::components::*;

pub fn ai_system(world: &mut World, dt: Duration, config: &GameConfig) {
    let dt_secs = dt.as_secs_f32();

    let mut ship_infos: Vec<(hecs::Entity, Transform, Ship, AiState)> = Vec::new();
    for (entity, (transform, ship, ai)) in world.query::<(&Transform, &Ship, &AiState)>().iter() {
        ship_infos.push((entity, *transform, *ship, *ai));
    }

    for (entity, _transform, ship, ai_state) in &ship_infos {
        let mut nearest_enemy: Option<(hecs::Entity, f32, Vec2)> = None;

        for (other_entity, other_transform, other_ship, _) in &ship_infos {
            if other_entity == entity {
                continue;
            }
            if !ship.faction.is_enemy(&other_ship.faction) {
                continue;
            }

            let dist = _transform.position.distance(other_transform.position);
            if dist < config.ai_detection_range {
                if nearest_enemy.is_none() || dist < nearest_enemy.as_ref().unwrap().1 {
                    nearest_enemy = Some((*other_entity, dist, other_transform.position));
                }
            }
        }

        let mut new_state = *ai_state;
        let mut target_velocity = Vec2::ZERO;
        let mut target_angular = 0.0;

        match nearest_enemy {
            Some((enemy_entity, dist, enemy_pos)) => {
                let direction = (enemy_pos - _transform.position).normalize_or_zero();

                if ship.health / ship.max_health < config.ai_flee_threshold {
                    new_state.current_state = AiBehaviorState::Retreating;
                    target_velocity = -direction * config.ship_max_speed * 0.8;
                } else if dist < 10.0 {
                    new_state.current_state = AiBehaviorState::Attacking;
                    new_state.target = Some(enemy_entity);

                    let target_angle = direction.y.atan2(direction.x);
                    let angle_diff = target_angle - _transform.rotation;
                    target_angular = angle_diff * config.ship_turn_speed;

                    if dist < 5.0 {
                        target_velocity = -direction * config.ship_max_speed * 0.5;
                    } else {
                        target_velocity = direction * config.ship_max_speed * 0.3;
                    }
                } else {
                    new_state.current_state = AiBehaviorState::Chasing;
                    new_state.target = Some(enemy_entity);

                    let target_angle = direction.y.atan2(direction.x);
                    let angle_diff = target_angle - _transform.rotation;
                    target_angular = angle_diff * config.ship_turn_speed;
                    target_velocity = direction * config.ship_max_speed;
                }
            }
            None => {
                new_state.current_state = AiBehaviorState::Idle;
                target_velocity = Vec2::new(
                    (entity.id() as f32 * 1.7).sin() * config.ship_max_speed * 0.3,
                    (entity.id() as f32 * 2.3).cos() * config.ship_max_speed * 0.3,
                );
            }
        }

        if let Ok((velocity, ai)) = world.query_one_mut::<(&mut Velocity, &mut AiState)>(entity) {
            let lerp_factor = 1.0 - (-config.ai_aggressiveness * dt_secs * 5.0).exp();
            velocity.linear = velocity.linear.lerp(target_velocity, lerp_factor);
            velocity.angular = velocity.angular.lerp(target_angular, lerp_factor);
            *ai = new_state;
        }
    }
}
