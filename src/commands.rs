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

/// Trait of a command
pub trait Command: Send + Sync {
    /// Invokes command with specified list of arguments
    fn invoke(&self, args: &[Arg]) -> bool;
}

/// Stateful [Command] implementation
pub struct StatefulCommand<S> {
    state: S,
    delegate: Box<dyn Fn(&[Arg], &S) -> bool>,
}

impl<S> StatefulCommand<S> {
    /// Creates new instance of [StatefulCommand] with some predefined state
    pub fn new<F>(state: S, delegate: F) -> StatefulCommand<S>
    where
        F: Fn(&[Arg], &S) -> bool + 'static,
    {
        StatefulCommand {
            state,
            delegate: Box::new(delegate),
        }
    }
}

impl<S> Command for StatefulCommand<S>
where
    S: Send + Sync,
{
    fn invoke(&self, args: &[Arg]) -> bool {
        (self.delegate)(args, &self.state)
    }
}

unsafe impl<S> Send for StatefulCommand<S> where S: Send + Sync {}

unsafe impl<S> Sync for StatefulCommand<S> where S: Send + Sync {}

/// INTERNAL: command identifier type alias
type CommandId = usize;
/// INTERNAL: list of commands
type CommandList = BTreeMap<CommandId, Box<dyn Command>>;

/// INTERNAL: command infrastructure state
#[derive(Default)]
struct Inner {
    commands: RwLock<BTreeMap<String, CommandList>>,
    id_counter: AtomicUsize,
}

impl Inner {
    /// INTERNAL: adds command
    fn add<C>(&self, name: String, command: C) -> CommandId
    where
        C: Command + 'static,
    {
        let mut commands = self.commands.write().unwrap();
        let command_list = commands.entry(name).or_default();

        let id = self.id_counter.fetch_add(1, Ordering::Relaxed);

        command_list.insert(id, Box::new(command));

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
        C: Command + 'static,
    {
        let name = name.into();

        Registration {
            inner: self.inner.clone(),
            name: name.clone(),
            command_id: self.inner.add(name, command),
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
