// 游戏模式模块 - 预留扩展接口
// 未来可在此实现各种游戏模式

/// 游戏模式枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameMode {
    /// 自由对战模式（默认）
    FreeForAll,
    /// 死亡竞赛模式
    DeathMatch,
    /// 生存模式
    Survival,
    /// 夺旗模式
    CaptureTheFlag,
}

/// 游戏模式配置
#[derive(Debug, Clone)]
pub struct ModeConfig {
    pub mode: GameMode,
    pub time_limit: Option<std::time::Duration>,
    pub score_limit: Option<u32>,
    pub respawn_enabled: bool,
}

impl Default for ModeConfig {
    fn default() -> Self {
        Self {
            mode: GameMode::FreeForAll,
            time_limit: None,
            score_limit: None,
            respawn_enabled: true,
        }
    }
}

/// 游戏模式系统
pub fn mode_system(_world: &mut hecs::World, _config: &ModeConfig) {
    // 预留扩展接口
}
