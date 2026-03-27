use hecs::World;
use std::time::Duration;
use crate::config::GameConfig;
use crate::ecs::components::*;
use crate::ecs::events::EventBus;
pub fn movement_system(world: &mut World, dt: Duration) {
    let dt_secs = dt.as_secs_f32();
    for (_entity, (transform, velocity)) in world.query_mut::<(&mut Transform, &Velocity)>() {
        transform.position += velocity.linear * dt_secs;
        transform.rotation += velocity.angular * dt_secs;
    }
}
pub fn boundary_system(world: &mut World, _events: &mut EventBus, config: &GameConfig) {
    let w = config.world_width;
    let h = config.world_height;
    for (_entity, (transform, mut velocity, collider)) in
        world.query_mut::<(&mut Transform, &mut Velocity, &Collider)>()
    {
        let r = collider.radius;
        if transform.position.x - r < 0.0 {
            transform.position.x = r;
            velocity.linear.x = velocity.linear.x.abs() * config.ship_bounce_damping;
        } else if transform.position.x + r > w {
            transform.position.x = w - r;
            velocity.linear.x = -velocity.linear.x.abs() * config.ship_bounce_damping;
        }
        if transform.position.y - r < 0.0 {
            transform.position.y = r;
            velocity.linear.y = velocity.linear.y.abs() * config.ship_bounce_damping;
        } else if transform.position.y + r > h {
            transform.position.y = h - r;
            velocity.linear.y = -velocity.linear.y.abs() * config.ship_bounce_damping;
        }
    }
}
pub fn collision_system(world: &mut World, events: &mut EventBus, config: &GameConfig) {
    let mut collisions = Vec::new();
    let mut colliders: Vec<(hecs::Entity, Transform, Collider)> = Vec::new();
    for (entity, (transform, collider)) in world.query::<(&Transform, &Collider)>().iter() {
        colliders.push((entity, *transform, *collider));
    }
    for i in 0..colliders.len() {
        for j in (i + 1)..colliders.len() {
            let (entity_a, transform_a, collider_a) = &colliders[i];
            let (entity_b, transform_b, collider_b) = &colliders[j];
            let dist = transform_a.position.distance(transform_b.position);
            let min_dist = collider_a.radius + collider_b.radius + config.collision_margin;
            if dist < min_dist {
                collisions.push((*entity_a, *entity_b));
            }
        }
    }
    for (entity_a, entity_b) in collisions {
        events.push(crate::ecs::events::GameEvent::Collision {
            entity_a,
            entity_b,
        });
    }
}
