use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{
        Arc, Mutex, RwLock,
        atomic::{AtomicUsize, Ordering},
    },
};

use winit::{
    event::{ElementState, KeyEvent},
    keyboard::{KeyCode, PhysicalKey},
};

use crate::{commands, handle};

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

/// Enumeration of possible input [Key] state
#[derive(Clone, Copy)]
pub enum State {
    Pressed,
    Released,
}

impl From<ElementState> for State {
    fn from(value: ElementState) -> Self {
        match value {
            ElementState::Pressed => Self::Pressed,
            ElementState::Released => Self::Released,
        }
    }
}

/// Input scheme
pub struct Scheme {
    mapping: BTreeMap<String, BTreeSet<Key>>,
}

impl Scheme {
    /// Adds mapping of keys to command
    pub fn add<S, I>(mut self, command: S, inputs: I) -> Scheme
    where
        S: Into<String>,
        I: IntoIterator<Item = Key>,
    {
        let command = command.into();

        self.mapping
            .entry(command)
            .or_default()
            .extend(inputs.into_iter());

        self
    }
}

impl Default for Scheme {
    fn default() -> Self {
        Self {
            mapping: Default::default(),
        }
    }
}

/// Input manager
pub struct Input {
    commands: Arc<commands::Commands>,
    scheme_counter: AtomicUsize,
    schemes: Arc<Mutex<BTreeMap<usize, Scheme>>>,
    mapping: Arc<RwLock<BTreeMap<Key, BTreeMap<String, usize>>>>,
}

impl Input {
    /// Creates new instance of [Input]
    pub fn new(commands: Arc<commands::Commands>) -> Arc<Input> {
        let input = Input {
            commands,
            scheme_counter: Default::default(),
            schemes: Default::default(),
            mapping: Default::default(),
        };

        Arc::new(input)
    }

    /// Adds input scheme
    #[must_use = "returned handle removes mapping provided by scheme"]
    pub fn add_scheme(&self, scheme: Scheme) -> handle::Handle {
        let mut schemes = self.schemes.lock().unwrap();
        let mut mapping = self.mapping.write().unwrap();

        let scheme_id = self.scheme_counter.fetch_add(1, Ordering::Relaxed);

        scheme
            .mapping
            .iter()
            .flat_map(|(command, keys)| keys.iter().copied().map(move |key| (key, command.clone())))
            .for_each(|(key, command)| {
                mapping
                    .entry(key)
                    .or_default()
                    .entry(command)
                    .and_modify(|ref_count| *ref_count += 1)
                    .or_insert(1);
            });

        schemes.insert(scheme_id, scheme);

        let schemes = self.schemes.clone();
        let mapping = self.mapping.clone();

        let drop = move || {
            if let Some(scheme) = schemes.lock().unwrap().remove(&scheme_id) {
                let mut mapping = mapping.write().unwrap();

                scheme
                    .mapping
                    .iter()
                    .flat_map(|(command, keys)| keys.iter().map(move |key| (key, command)))
                    .for_each(|(key, command)| {
                        let mapping = mapping.entry(*key).or_default();

                        if let Some(ref_count) = mapping.get_mut(command) {
                            *ref_count -= 1;

                            if *ref_count == 0 {
                                mapping.remove(command);
                            }
                        }
                    });
            }
        };

        drop.into()
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
            let state = event.state.into();

            self.dispatch(key, state);
        }
    }

    /// INTERNAL: dispatches key state
    fn dispatch(&self, key: Key, state: State) {
        let arg = (key, state).into();
        let mapping = self.mapping.read().unwrap();

        mapping.get(&key).map(|commands| {
            for command in commands.keys() {
                self.commands.invoke(command, &[arg]);
            }
        });
    }
}
