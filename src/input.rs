use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, Mutex, RwLock},
};

use winit::{
    event::{ElementState, KeyEvent},
    keyboard::{KeyCode, PhysicalKey},
};

use crate::commands::Commands;

/// Enumeration of input keys: contains keyboard (`Kbd...`), mouse (`Mouse...`), gamepad (`G`) keys
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Key {
    KbdSpace,
    KbdEscape,
    KbdEnter,

    KbdArrowUp,
    KbdArrowDown,
    KbdArrowLeft,
    KbdArrowRight,

    KbdA,
    KbdB,
    KbdC,
    KbdD,
    KbdE,
    KbdF,
    KbdG,
    KbdH,
    KbdI,
    KbdJ,
    KbdK,
    KbdL,
    KbdM,
    KbdN,
    KbdO,
    KbdP,
    KbdQ,
    KbdR,
    KbdS,
    KbdT,
    KbdU,
    KbdV,
    KbdW,
    KbdX,
    KbdY,
    KbdZ,

    Kbd0,
    Kbd1,
    Kbd2,
    Kbd3,
    Kbd4,
    Kbd5,
    Kbd6,
    Kbd7,
    Kbd8,
    Kbd9,
    // TODO: add more keys
}

impl TryFrom<KeyCode> for Key {
    type Error = ();

    fn try_from(value: KeyCode) -> Result<Self, Self::Error> {
        match value {
            KeyCode::Space => Ok(Key::KbdSpace),
            KeyCode::Escape => Ok(Key::KbdEscape),
            KeyCode::Enter => Ok(Key::KbdEnter),

            KeyCode::ArrowUp => Ok(Key::KbdArrowUp),
            KeyCode::ArrowDown => Ok(Key::KbdArrowDown),
            KeyCode::ArrowLeft => Ok(Key::KbdArrowLeft),
            KeyCode::ArrowRight => Ok(Key::KbdArrowRight),

            KeyCode::KeyA => Ok(Key::KbdA),
            KeyCode::KeyB => Ok(Key::KbdB),
            KeyCode::KeyC => Ok(Key::KbdC),
            KeyCode::KeyD => Ok(Key::KbdD),
            KeyCode::KeyE => Ok(Key::KbdE),
            KeyCode::KeyF => Ok(Key::KbdF),
            KeyCode::KeyG => Ok(Key::KbdG),
            KeyCode::KeyH => Ok(Key::KbdH),
            KeyCode::KeyI => Ok(Key::KbdI),
            KeyCode::KeyJ => Ok(Key::KbdJ),
            KeyCode::KeyK => Ok(Key::KbdK),
            KeyCode::KeyL => Ok(Key::KbdL),
            KeyCode::KeyM => Ok(Key::KbdM),
            KeyCode::KeyN => Ok(Key::KbdN),
            KeyCode::KeyO => Ok(Key::KbdO),
            KeyCode::KeyP => Ok(Key::KbdP),
            KeyCode::KeyQ => Ok(Key::KbdQ),
            KeyCode::KeyR => Ok(Key::KbdR),
            KeyCode::KeyS => Ok(Key::KbdS),
            KeyCode::KeyT => Ok(Key::KbdT),
            KeyCode::KeyU => Ok(Key::KbdU),
            KeyCode::KeyV => Ok(Key::KbdV),
            KeyCode::KeyW => Ok(Key::KbdW),
            KeyCode::KeyX => Ok(Key::KbdX),
            KeyCode::KeyY => Ok(Key::KbdY),
            KeyCode::KeyZ => Ok(Key::KbdZ),

            KeyCode::Digit0 => Ok(Key::Kbd0),
            KeyCode::Digit1 => Ok(Key::Kbd1),
            KeyCode::Digit2 => Ok(Key::Kbd2),
            KeyCode::Digit3 => Ok(Key::Kbd3),
            KeyCode::Digit4 => Ok(Key::Kbd4),
            KeyCode::Digit5 => Ok(Key::Kbd5),
            KeyCode::Digit6 => Ok(Key::Kbd6),
            KeyCode::Digit7 => Ok(Key::Kbd7),
            KeyCode::Digit8 => Ok(Key::Kbd8),
            KeyCode::Digit9 => Ok(Key::Kbd9),

            _ => Err(()),
        }
    }
}

/// Input manager
pub struct Manager {
    commands: Arc<Commands>,
    key_maps: RwLock<BTreeMap<Key, String>>,
    pressed: Mutex<BTreeSet<Key>>,
}

impl Manager {
    /// Creates new instance of [Manager]
    pub fn new(commands: Arc<Commands>) -> Manager {
        Manager {
            commands,
            key_maps: Default::default(),
            pressed: Default::default(),
        }
    }

    /// Sets mapping between key and command
    pub fn set_key_map<N>(&self, key: Key, name: N)
    where
        N: Into<String>,
    {
        let mut key_maps = self.key_maps.write().unwrap();

        key_maps.insert(key, name.into());
    }

    /// Removes mapping for key
    pub fn remove_key_map(&self, key: Key) {
        let mut key_maps = self.key_maps.write().unwrap();

        key_maps.remove(&key);
    }

    /// Dispatches [winit::event::KeyEvent] by our key mapping
    pub fn dispatch_key_event(&self, event: KeyEvent) {
        if event.repeat {
            return;
        }

        let key = match event.physical_key {
            PhysicalKey::Code(code) => {
                if let Ok(key) = code.try_into() {
                    Some(key)
                } else {
                    None
                }
            }
            _ => None,
        };

        if let Some(key) = key {
            let mut pressed = self.pressed.lock().unwrap();

            match event.state {
                ElementState::Pressed => pressed.insert(key),
                ElementState::Released => pressed.remove(&key),
            };
        }
    }

    /// Dispatches all pressed keys
    pub fn dispatch(&self) {
        let pressed = self.pressed.lock().unwrap();
        let key_maps = self.key_maps.read().unwrap();

        pressed
            .iter()
            .copied()
            .filter_map(|key| key_maps.get(&key).map(|command| (key, command)))
            .for_each(|(key, command)| self.commands.invoke(command, &[key.into()]));
    }
}
