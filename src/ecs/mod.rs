// ECS模块总入口，声明所有子模块，统一导出供全局使用
pub mod components;
pub mod events;

// 统一导出，其他模块无需写全路径，直接 use crate::ecs::* 即可
pub use components::*;
pub use events::*;
