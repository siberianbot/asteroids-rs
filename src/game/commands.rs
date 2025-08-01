use std::sync::Arc;

use crate::{
    game::controller::{
        CameraZoomDirection, Controller, SpacecraftAccelerationDirection,
        SpacecraftInclineDirection,
    },
    input,
};

/// Dispatches `camera_follow` to controller
pub fn camera_follow_command(_: &[crate::commands::Arg], controller: &Arc<Controller>) -> bool {
    controller.camera_follow_toggle();

    true
}

/// Dispatches `camera_zoom_out` to controller
pub fn camera_zoom_out_command(_: &[crate::commands::Arg], controller: &Arc<Controller>) -> bool {
    controller.camera_zoom(CameraZoomDirection::Out);

    true
}

/// Dispatches `camera_zoom_in` to controller
pub fn camera_zoom_in_command(_: &[crate::commands::Arg], controller: &Arc<Controller>) -> bool {
    controller.camera_zoom(CameraZoomDirection::In);

    true
}

/// Dispatches `player_forward` to controller
pub fn player_forward_command(args: &[crate::commands::Arg], controller: &Arc<Controller>) -> bool {
    let (_, state) = args[0].to_input().expect("invalid usage of player_forward");

    match state {
        input::State::Pressed => {
            controller.player_accelerate(SpacecraftAccelerationDirection::Forward)
        }

        input::State::Released => {
            controller.player_stop_accelerate();
        }
    }

    true
}

/// Dispatches `player_backward` to controller
pub fn player_backward_command(
    args: &[crate::commands::Arg],
    controller: &Arc<Controller>,
) -> bool {
    let (_, state) = args[0]
        .to_input()
        .expect("invalid usage of player_backward");

    match state {
        input::State::Pressed => {
            controller.player_accelerate(SpacecraftAccelerationDirection::Backward)
        }

        input::State::Released => {
            controller.player_stop_accelerate();
        }
    }

    true
}

/// Dispatches `player_incline_left` to controller
pub fn player_incline_left_command(
    args: &[crate::commands::Arg],
    controller: &Arc<Controller>,
) -> bool {
    let (_, state) = args[0]
        .to_input()
        .expect("invalid usage of player_incline_left");

    match state {
        input::State::Pressed => controller.player_incline(SpacecraftInclineDirection::Left),

        input::State::Released => {
            controller.player_stop_incline();
        }
    }

    true
}

/// Dispatches `player_incline_right` to controller
pub fn player_incline_right_command(
    args: &[crate::commands::Arg],
    controller: &Arc<Controller>,
) -> bool {
    let (_, state) = args[0]
        .to_input()
        .expect("invalid usage of player_incline_right");

    match state {
        input::State::Pressed => controller.player_incline(SpacecraftInclineDirection::Right),

        input::State::Released => {
            controller.player_stop_incline();
        }
    }

    true
}
