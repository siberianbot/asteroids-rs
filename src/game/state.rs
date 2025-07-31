use std::{
    collections::BTreeMap,
    ptr::NonNull,
    sync::{
        Arc, RwLock, RwLockReadGuard, RwLockWriteGuard,
        atomic::{AtomicUsize, Ordering},
    },
};

use crate::{
    dispatch::{Dispatcher, Event},
    game::entities::EntityId,
};

/// A player
#[derive(Default)]
pub struct Player {
    /// Identifier of spacecraft entity
    pub spacecraft_id: Option<EntityId>,

    /// Respawn timer
    pub respawn_timer: f32,

    /// Player score
    pub score: u32,
}

/// Type alias for player identifier
pub type PlayerId = usize;

/// Read-only iterator over list of players
pub struct PlayerIter<'a> {
    lock: RwLockReadGuard<'a, BTreeMap<PlayerId, Player>>,
    player_id: PlayerId,
    max_player_id: PlayerId,
}

impl<'a> Iterator for PlayerIter<'a> {
    type Item = (PlayerId, &'a Player);

    fn next(&mut self) -> Option<Self::Item> {
        while self.player_id <= self.max_player_id {
            let player_id = self.player_id;
            self.player_id += 1;

            let tuple = self.lock.get(&player_id).map(|player| unsafe {
                // HACK this iterator have a lock to container, references are valid 'till iterator lifetime
                (player_id, NonNull::from(player).as_ref())
            });

            if tuple.is_some() {
                return tuple;
            }
        }

        None
    }
}

/// Iterator over list of players, allows mutability
pub struct PlayerIterMut<'a> {
    lock: RwLockWriteGuard<'a, BTreeMap<PlayerId, Player>>,
    player_id: PlayerId,
    max_player_id: PlayerId,
}

impl<'a> Iterator for PlayerIterMut<'a> {
    type Item = (PlayerId, &'a mut Player);

    fn next(&mut self) -> Option<Self::Item> {
        while self.player_id <= self.max_player_id {
            let player_id = self.player_id;
            self.player_id += 1;

            let tuple = self.lock.get(&player_id).map(|player| unsafe {
                // HACK this iterator have a lock to container, references are valid 'till iterator lifetime
                (player_id, NonNull::from(player).as_mut())
            });

            if tuple.is_some() {
                return tuple;
            }
        }

        None
    }
}

/// Global game state
pub struct State {
    player_counter: AtomicUsize,
    players: RwLock<BTreeMap<PlayerId, Player>>,
    camera_id: RwLock<Option<EntityId>>,
}

impl State {
    /// Creates new instance of [State]
    pub fn new(events: &Dispatcher<Event>) -> Arc<State> {
        let state = State {
            player_counter: Default::default(),
            players: Default::default(),
            camera_id: Default::default(),
        };

        let state = Arc::new(state);

        {
            let state = state.clone();

            events.add_handler(move |event| match event {
                Event::EntityDestroyed(entity_id) => state.handle_entity_destroy(*entity_id),
                _ => {}
            });
        }

        state
    }

    /// Returns read-only iterator over list of players
    pub fn iter_players(&self) -> PlayerIter {
        let lock = self.players.read().unwrap();

        PlayerIter {
            player_id: lock
                .first_key_value()
                .map(|(player_id, _)| *player_id)
                .unwrap_or(0),

            max_player_id: lock
                .last_key_value()
                .map(|(player_id, _)| *player_id)
                .unwrap_or(0),

            lock,
        }
    }

    /// Returns iterator over list of players, allows mutability
    pub fn iter_players_mut(&self) -> PlayerIterMut {
        let lock = self.players.write().unwrap();

        PlayerIterMut {
            player_id: lock
                .first_key_value()
                .map(|(player_id, _)| *player_id)
                .unwrap_or(0),

            max_player_id: lock
                .last_key_value()
                .map(|(player_id, _)| *player_id)
                .unwrap_or(0),

            lock,
        }
    }

    /// Sets current camera [EntityId]
    pub fn set_camera(&self, camera_id: Option<EntityId>) {
        *self.camera_id.write().unwrap() = camera_id;
    }

    /// Gets current camera [EntityId]
    pub fn get_camera(&self) -> Option<EntityId> {
        self.camera_id.read().unwrap().clone()
    }

    /// Visits player by its [PlayerId]
    pub fn visit_player<V, R>(&self, player_id: &PlayerId, visitor: V) -> Option<R>
    where
        V: FnOnce(&Player) -> R,
    {
        let players = self.players.read().unwrap();

        players.get(player_id).map(visitor)
    }

    /// Creates new player and returns its [PlayerId]
    pub fn new_player(&self) -> PlayerId {
        let mut players = self.players.write().unwrap();
        let player_id = self.player_counter.fetch_add(1, Ordering::Relaxed);

        players.insert(player_id, Default::default());

        player_id
    }

    /// Kicks player by its [PlayerId]
    pub fn kick_player(&self, player_id: PlayerId) {
        let mut players = self.players.write().unwrap();

        players.remove(&player_id);
    }

    /// INTERNAL: handles [crate::dispatch::Event::EntityDestroyed]
    fn handle_entity_destroy(&self, entity_id: EntityId) {
        let player_id = self
            .players
            .read()
            .unwrap()
            .iter()
            .filter(|(_, player)| player.spacecraft_id == Some(entity_id))
            .map(|(player_id, _)| *player_id)
            .next();

        if let Some(player_id) = player_id {
            if let Some(player) = self.players.write().unwrap().get_mut(&player_id) {
                player.spacecraft_id = None;
            }
        }
    }
}
