use fluorite_core::ecs::components::grid::GridPosition;
use fluorite_core::ecs::components::player::PlayerId;
use fluorite_core::ffi::game_bridge::{GameStateBridge, SetupItemType};

#[test]
fn test_game_bridge_start_and_deploy() {
    let mut bridge = GameStateBridge::new();
    assert!(bridge.start_game().is_ok());

    assert!(bridge
        .deploy_item(PlayerId::P1, 0, 0, SetupItemType::Piece)
        .is_ok());
    assert!(bridge
        .deploy_item(
            PlayerId::P2,
            1,
            1,
            SetupItemType::Trap {
                value: 5,
                is_flag: false
            }
        )
        .is_ok());

    assert_eq!(bridge.pieces.len(), 1);
    assert_eq!(bridge.traps.len(), 1);
}

#[test]
fn test_game_bridge_move_piece() {
    let mut bridge = GameStateBridge::new();
    bridge.start_game().unwrap();

    bridge
        .players
        .get_mut(&PlayerId::P1)
        .unwrap()
        .movement_points = 10;

    bridge
        .deploy_item(PlayerId::P1, 0, 0, SetupItemType::Piece)
        .unwrap();

    // Move to adjacent cell
    assert!(bridge.move_piece(PlayerId::P1, 0, 0, 0, 1).is_ok());
    assert!(bridge.pieces.contains_key(&GridPosition::new(0, 1)));
    assert!(!bridge.pieces.contains_key(&GridPosition::new(0, 0)));
}
