use winit::{
    event::{ElementState, KeyEvent},
    keyboard::{KeyCode, PhysicalKey},
};

use crate::{
    dispatch::{Command, Dispatcher, Sender},
    game::entities::PlayerAction,
};

pub struct InputController {
    command_sender: Sender<Command>,
}

impl InputController {
    pub fn new(command_dispatcher: &Dispatcher<Command>) -> InputController {
        InputController {
            command_sender: command_dispatcher.create_sender(),
        }
    }

    pub fn dispatch(&self, event: KeyEvent) {
        if event.repeat {
            return;
        }

        let command = match event.physical_key {
            PhysicalKey::Code(KeyCode::ArrowUp) | PhysicalKey::Code(KeyCode::KeyW) => {
                Some(Self::map_movement_command(event, PlayerAction::ACCELERATE))
            }

            PhysicalKey::Code(KeyCode::ArrowDown) | PhysicalKey::Code(KeyCode::KeyS) => {
                Some(Self::map_movement_command(event, PlayerAction::DECELERATE))
            }

            PhysicalKey::Code(KeyCode::ArrowLeft) | PhysicalKey::Code(KeyCode::KeyA) => Some(
                Self::map_movement_command(event, PlayerAction::INCLINE_LEFT),
            ),

            PhysicalKey::Code(KeyCode::ArrowRight) | PhysicalKey::Code(KeyCode::KeyD) => Some(
                Self::map_movement_command(event, PlayerAction::INCLINE_RIGHT),
            ),

            PhysicalKey::Code(KeyCode::Space) => {
                Some(Self::map_movement_command(event, PlayerAction::FIRE))
            }

            PhysicalKey::Code(KeyCode::KeyF) if event.state == ElementState::Released => {
                Some(Command::ToggleCameraFollow)
            }

            PhysicalKey::Code(KeyCode::KeyQ) if event.state == ElementState::Released => {
                Some(Command::CameraZoomIn)
            }

            PhysicalKey::Code(KeyCode::KeyE) if event.state == ElementState::Released => {
                Some(Command::CameraZoomOut)
            }

            _ => None,
        };

        if let Some(command) = command {
            self.command_sender.send(command);
        }
    }

    fn map_movement_command(event: KeyEvent, movement: PlayerAction) -> Command {
        match event.state {
            ElementState::Pressed => Command::PlayerActionDown(movement),
            ElementState::Released => Command::PlayerActionUp(movement),
        }
    }
}
