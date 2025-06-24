use winit::{
    event::{ElementState, KeyEvent},
    keyboard::{KeyCode, PhysicalKey},
};

use crate::{
    dispatch::{Command, Dispatcher, Sender},
    game::entities::PlayerMovement,
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

        let movement = match event.physical_key {
            PhysicalKey::Code(KeyCode::ArrowUp) => Some(PlayerMovement::ACCELERATE),
            PhysicalKey::Code(KeyCode::ArrowDown) => Some(PlayerMovement::DECELERATE),
            PhysicalKey::Code(KeyCode::ArrowLeft) => Some(PlayerMovement::INCLINE_LEFT),
            PhysicalKey::Code(KeyCode::ArrowRight) => Some(PlayerMovement::INCLINE_RIGHT),

            _ => None,
        };

        let command = movement.map(|movement| match event.state {
            ElementState::Pressed => Command::PlayerMovementDown(movement),
            ElementState::Released => Command::PlayerMovementUp(movement),
        });

        if let Some(command) = command {
            self.command_sender.send(command);
        }
    }
}
