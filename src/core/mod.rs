// 核心业务模块总入口，声明所有子模块，统一导出供全局使用
pub mod ai;
pub mod combat;
pub mod physics;
pub mod render;
pub mod modes;
pub mod skills;

// 统一导出，其他模块无需写全路径，直接 use crate::core::* 即可
pub use ai::*;
pub use combat::*;
pub use physics::*;
pub use render::*;
