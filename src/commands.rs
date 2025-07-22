use std::{
    collections::BTreeMap,
    sync::{
        Arc, RwLock,
        atomic::{AtomicUsize, Ordering},
    },
};

use crate::input::Key;

/// Enumeration of possible command arguments
#[derive(Clone, Copy)]
pub enum Arg {
    /// Argument is an input key
    Key(Key),
}

impl From<Key> for Arg {
    fn from(value: Key) -> Self {
        Self::Key(value)
    }
}

/// A command delegate
pub struct Command {
    delegate: Box<dyn Fn(&[Arg]) -> bool>,
}

impl Command {
    /// INTERNAL: invokes command with specified list of arguments
    fn invoke(&self, args: &[Arg]) -> bool {
        (self.delegate)(args)
    }
}

impl<F> From<F> for Command
where
    F: Fn(&[Arg]) -> bool + 'static,
{
    fn from(value: F) -> Self {
        Self {
            delegate: Box::new(value),
        }
    }
}

/// INTERNAL: command identifier type alias
type CommandId = usize;
/// INTERNAL: list of commands
type CommandList = BTreeMap<CommandId, Command>;

/// INTERNAL: command infrastructure state
#[derive(Default)]
struct Inner {
    commands: RwLock<BTreeMap<String, CommandList>>,
    id_counter: AtomicUsize,
}

impl Inner {
    /// INTERNAL: adds command
    fn add(&self, name: String, command: Command) -> CommandId {
        let mut commands = self.commands.write().unwrap();
        let command_list = commands.entry(name).or_default();

        let id = self.id_counter.fetch_add(1, Ordering::Relaxed);

        command_list.insert(id, command.into());

        id
    }

    /// INTERNAL: removes command
    fn remove(&self, name: &String, command_id: CommandId) {
        let mut commands = self.commands.write().unwrap();

        if let Some(command_list) = commands.get_mut(name) {
            command_list.remove(&command_id);
        }
    }

    /// INTERNAL: invokes command
    fn invoke(&self, name: &String, args: &[Arg]) {
        let commands = self.commands.read().unwrap();

        if let Some(command_list) = commands.get(name) {
            for (_, command) in command_list.iter() {
                if !command.invoke(&args) {
                    return;
                }
            }
        }
    }
}

/// Command registration, removes command delegate on drop
pub struct Registration {
    inner: Arc<Inner>,
    name: String,
    command_id: CommandId,
}

impl Drop for Registration {
    fn drop(&mut self) {
        self.inner.remove(&self.name, self.command_id);
    }
}

/// Commands infrastructure
#[derive(Default)]
pub struct Commands {
    inner: Arc<Inner>,
}

impl Commands {
    /// Adds command
    #[must_use = "registration removes command on drop"]
    pub fn add<N, C>(&self, name: N, command: C) -> Registration
    where
        N: Into<String>,
        C: Fn(&[Arg]) -> bool + 'static,
    {
        let name = name.into();

        Registration {
            inner: self.inner.clone(),
            name: name.clone(),
            command_id: self.inner.add(name, command.into()),
        }
    }

    /// Invokes command
    pub fn invoke<N>(&self, name: N, args: &[Arg])
    where
        N: Into<String>,
    {
        self.inner.invoke(&name.into(), args);
    }
}
