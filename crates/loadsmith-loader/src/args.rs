use std::{collections::HashMap, ffi::OsString, fmt::Debug, path::Path, path::PathBuf};

/// Arguments, environment variables, and an optional wrapper binary to use
/// when launching a game with a mod loader.
///
/// Build one using the builder methods ([`arg`](LaunchArgs::arg),
/// [`env`](LaunchArgs::env), [`wrapper`](LaunchArgs::wrapper)), then call
/// [`apply`](LaunchArgs::apply) to configure a
/// [`std::process::Command`].
///
/// # Examples
///
/// ```rust
/// use loadsmith_loader::LaunchArgs;
/// use std::process::Command;
///
/// let args = LaunchArgs::new()
///     .arg("--doorstop-enable")
///     .arg("true")
///     .env("DOORSTOP_ENABLE", "TRUE");
///
/// let mut cmd = Command::new("game.exe");
/// args.apply(&mut cmd);
///
/// assert_eq!(cmd.get_args().collect::<Vec<_>>(), ["--doorstop-enable", "true"]);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LaunchArgs {
    args: Vec<OsString>,
    env: HashMap<OsString, OsString>,
    wrapper: Option<PathBuf>,
}

impl LaunchArgs {
    /// Creates an empty set of launch arguments.
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends a positional argument.
    pub fn arg(mut self, arg: impl Into<OsString>) -> Self {
        self.args.push(arg.into());
        self
    }

    /// Sets an environment variable.
    pub fn env(mut self, key: impl Into<OsString>, value: impl Into<OsString>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }

    /// Sets a wrapper binary (e.g. `wine` or `proton`) that the command
    /// should be launched through.
    pub fn wrapper(mut self, wrapper: impl Into<PathBuf>) -> Self {
        self.wrapper = Some(wrapper.into());
        self
    }

    /// Returns the positional arguments.
    pub fn get_args(&self) -> &[OsString] {
        &self.args
    }

    /// Returns the environment variables.
    pub fn get_env(&self) -> &HashMap<OsString, OsString> {
        &self.env
    }

    /// Returns the wrapper binary path, if set.
    pub fn get_wrapper(&self) -> Option<&Path> {
        self.wrapper.as_deref()
    }

    /// Applies these arguments, environment, and optional wrapper to a
    /// [`std::process::Command`].
    ///
    /// When a wrapper is set the original command's program and arguments
    /// become sub-arguments of the wrapper.
    pub fn apply(self, command: &mut std::process::Command) {
        if let Some(wrapper) = self.wrapper {
            let mut new_command = std::process::Command::new(wrapper);
            new_command
                .arg(command.get_program())
                .args(command.get_args());

            for (key, value) in command.get_envs() {
                if let Some(value) = value {
                    new_command.env(key, value);
                } else {
                    new_command.env_remove(key);
                }
            }

            *command = new_command;
        }

        for arg in self.args {
            command.arg(arg);
        }

        for (key, value) in self.env {
            command.env(key, value);
        }
    }
}
