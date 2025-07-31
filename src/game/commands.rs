/// Commands for [crate::game::entities::Camera]
pub mod camera {
    use std::{
        ops::{Div, Mul},
        sync::Arc,
    };

    use crate::game::{ecs::ECS, state::State};

    /// State for commands over [crate::game::entities::Camera] entity
    #[derive(Clone)]
    pub struct CameraCommandState {
        game_state: Arc<State>,
        ecs: Arc<ECS>,
    }

    impl CameraCommandState {
        /// Creates new instance of [CameraCommandState]
        pub fn new(game_state: Arc<State>, ecs: Arc<ECS>) -> CameraCommandState {
            CameraCommandState { game_state, ecs }
        }
    }

    /// Toggles current camera following behavior
    pub fn camera_follow_command(_: &[crate::commands::Arg], state: &CameraCommandState) -> bool {
        if let Some(camera_id) = state.game_state.get_camera() {
            state
                .ecs
                .write()
                .modify(camera_id, |entity| {
                    if let Some(camera) = entity.camera_mut() {
                        camera.follow = !camera.follow;
                    }
                });
        }

        true
    }

    /// Zoom outs current camera
    pub fn camera_zoom_out_command(_: &[crate::commands::Arg], state: &CameraCommandState) -> bool {
        camera_zoom(CameraZoom::Out, state);

        true
    }

    /// Zoom ins current camera
    pub fn camera_zoom_in_command(_: &[crate::commands::Arg], state: &CameraCommandState) -> bool {
        camera_zoom(CameraZoom::In, state);

        true
    }

    /// INTERNAL: [crate::game::entities::Camera] zoom direction
    enum CameraZoom {
        In,
        Out,
    }

    /// INTERNAL: [crate::game::entities::Camera] zoom logic
    fn camera_zoom(zoom: CameraZoom, state: &CameraCommandState) {
        const MIN_DISTANCE: f32 = 1.0;
        const MAX_DISTANCE: f32 = 32.0;
        const DISTANCE_MULTIPLIER: f32 = 2.0;

        if let Some(camera_id) = state.game_state.get_camera() {
            state
                .ecs
                .write()
                .modify(camera_id, |entity| {
                    if let Some(camera) = entity.camera_mut() {
                        camera.distance = match zoom {
                            CameraZoom::In => camera.distance.div(DISTANCE_MULTIPLIER),
                            CameraZoom::Out => camera.distance.mul(DISTANCE_MULTIPLIER),
                        };

                        camera.distance = camera.distance.clamp(MIN_DISTANCE, MAX_DISTANCE);
                    }
                });
        }
    }
}
