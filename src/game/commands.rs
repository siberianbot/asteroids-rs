use std::sync::Arc;

use crate::game::controller::{
    CameraZoomDirection, Controller, SpacecraftAccelerationDirection, SpacecraftInclineDirection,
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
pub fn player_forward_command(_: &[crate::commands::Arg], controller: &Arc<Controller>) -> bool {
    controller.player_accelerate(SpacecraftAccelerationDirection::Forward);

    true
}

/// Dispatches `player_backward` to controller
pub fn player_backward_command(_: &[crate::commands::Arg], controller: &Arc<Controller>) -> bool {
    controller.player_accelerate(SpacecraftAccelerationDirection::Backward);

    true
}

/// Dispatches `player_incline_left` to controller
pub fn player_incline_left_command(
    _: &[crate::commands::Arg],
    controller: &Arc<Controller>,
) -> bool {
    controller.player_incline(SpacecraftInclineDirection::Left);

    true
}

/// Dispatches `player_incline_right` to controller
pub fn player_incline_right_command(
    _: &[crate::commands::Arg],
    controller: &Arc<Controller>,
) -> bool {
    controller.player_incline(SpacecraftInclineDirection::Right);

    true
}
