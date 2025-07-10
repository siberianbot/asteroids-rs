use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Mutex, RwLock},
};

use winit::{
    event::{ElementState, KeyEvent},
    keyboard::{KeyCode, PhysicalKey},
};

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

/// Type alias for input action delegate
pub type Delegate = Box<dyn Fn(Key)>;

/// Input manager
#[derive(Default)]
pub struct Manager {
    actions: RwLock<BTreeMap<String, Delegate>>,
    key_maps: RwLock<BTreeMap<Key, String>>,
    pressed: Mutex<BTreeSet<Key>>,
}

impl Manager {
    /// Adds action
    pub fn set_action<N, D>(&self, name: N, delegate: D)
    where
        N: Into<String>,
        D: Fn(Key) + 'static,
    {
        let mut actions = self.actions.write().unwrap();

        actions.insert(name.into(), Box::new(delegate));
    }

    /// Removes action
    pub fn remove_action<N>(&self, name: N)
    where
        N: Into<String>,
    {
        let mut actions = self.actions.write().unwrap();

        actions.remove(&name.into());
    }

    /// Sets mapping between key and action
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

    /// Dispatches all pressed keys to actions
    pub fn dispatch(&self) {
        let actions = self.actions.read().unwrap();
        let pressed = self.pressed.lock().unwrap();
        let key_maps = self.key_maps.read().unwrap();

        pressed
            .iter()
            .copied()
            .filter_map(|key| {
                key_maps
                    .get(&key)
                    .and_then(|action| actions.get(action))
                    .map(|action| (key, action.as_ref()))
            })
            .for_each(|(key, action)| action(key));
    }
}
