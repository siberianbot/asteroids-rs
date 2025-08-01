use std::sync::Arc;

use crate::game::controller::{CameraZoomDirection, Controller};

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
