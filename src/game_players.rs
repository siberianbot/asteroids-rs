use std::sync::RwLock;

use crate::game_entity::EntityId;

/// A player
pub struct GamePlayer {
    /// Identifier of spacecraft entity
    pub spacecraft_id: EntityId,
}

/// List of all players
pub struct GamePlayers {
    /// Players
    pub players: RwLock<Vec<GamePlayer>>,
}

impl Default for GamePlayers {
    fn default() -> Self {
        Self {
            players: Default::default(),
        }
    }
}
