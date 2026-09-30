use crate::ecs::components::grid::GridPosition;
use crate::ecs::components::pieces::{Piece, Trap};
use crate::ecs::components::player::{PlayerId, PlayerStats};
use crate::ecs::systems::combat::CombatSystem;
use crate::ecs::systems::movement::MovementSystem;
use crate::ecs::systems::trap::TrapSystem;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// PLACEHOLDER: Flutter-to-Rust Game Bridge
//
// INSTRUCTIONS FOR IMPLEMENTATION:
// 1. Expose `GameState` struct containing `HashMap<GridPosition, Piece>` and `HashMap<GridPosition, Trap>` via flutter_rust_bridge.
// 2. Define standard C-API or flutter_rust_bridge compatible functions:
//    - `pub fn start_game() -> Result<(), String>`
//    - `pub fn deploy_item(player: PlayerId, x: i32, y: i32, item_type: SetupItemType) -> Result<(), String>`
//    - `pub fn move_piece(player: PlayerId, from_x: i32, from_y: i32, to_x: i32, to_y: i32) -> Result<(), String>`
// 3. Ensure this file is included in your `frb` configuration or root `lib.rs` module (`pub mod ffi;`).
// 4. Hook up `movement.rs`, `combat.rs`, and `trap.rs` ECS logic to these bridge methods.

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SetupItemType {
    Piece,
    Trap { value: i32, is_flag: bool },
}

#[derive(Default, Debug, Clone)]
pub struct GameStateBridge {
    pub pieces: HashMap<GridPosition, Piece>,
    pub traps: HashMap<GridPosition, Trap>,
    pub players: HashMap<PlayerId, PlayerStats>,
}

impl GameStateBridge {
    #[flutter_rust_bridge::frb(sync)]
    pub fn new() -> Self {
        Self::default()
    }

    #[flutter_rust_bridge::frb(sync)]
    pub fn start_game(&mut self) -> Result<(), String> {
        self.players
            .insert(PlayerId::P1, PlayerStats::new(PlayerId::P1));
        self.players
            .insert(PlayerId::P2, PlayerStats::new(PlayerId::P2));
        Ok(())
    }

    #[flutter_rust_bridge::frb(sync)]
    pub fn deploy_item(
        &mut self,
        player: PlayerId,
        x: i32,
        y: i32,
        item_type: SetupItemType,
    ) -> Result<(), String> {
        let pos = GridPosition::new(x, y);
        match item_type {
            SetupItemType::Piece => {
                self.pieces.insert(pos, Piece { owner: player });
            }
            SetupItemType::Trap { value, is_flag } => {
                self.traps.insert(
                    pos,
                    Trap {
                        owner: player,
                        value,
                        is_flag,
                        revealed: false,
                    },
                );
            }
        }
        Ok(())
    }

    #[flutter_rust_bridge::frb(sync)]
    pub fn move_piece(
        &mut self,
        player: PlayerId,
        from_x: i32,
        from_y: i32,
        to_x: i32,
        to_y: i32,
    ) -> Result<(), String> {
        let from = GridPosition::new(from_x, from_y);
        let to = GridPosition::new(to_x, to_y);

        let mut p_stats = self
            .players
            .get_mut(&player)
            .ok_or("Player not found")?
            .clone();

        // Check for traps
        if let Some(trap) = self.traps.get_mut(&to) {
            if trap.owner != player {
                let hit = TrapSystem::resolve_trap(&mut p_stats, trap)?;
                self.players.insert(player, p_stats.clone());
                if hit {
                    return Ok(()); // Movement stops on trap hit
                }
            }
        }

        // Check for combat
        let defender_owner = self.pieces.get(&to).map(|p| p.owner);
        if let Some(owner) = defender_owner {
            if owner != player {
                let mut d_stats = self
                    .players
                    .get_mut(&owner)
                    .ok_or("Defender not found")?
                    .clone();
                CombatSystem::resolve_combat(&mut p_stats, &mut d_stats, to, &mut self.pieces)?;
                self.players.insert(player, p_stats.clone());
                self.players.insert(owner, d_stats);
            }
        }

        MovementSystem::move_piece(&mut p_stats, &mut self.pieces, from, to)?;
        self.players.insert(player, p_stats);

        Ok(())
    }
}
