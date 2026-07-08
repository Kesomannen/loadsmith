use std::{collections::HashMap, ffi::OsString, fmt::Debug, path::Path, path::PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LaunchArgs {
    args: Vec<OsString>,
    env: HashMap<OsString, OsString>,
    wrapper: Option<PathBuf>,
}

impl LaunchArgs {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn arg(mut self, arg: impl Into<OsString>) -> Self {
        self.args.push(arg.into());
        self
    }

    pub fn env(mut self, key: impl Into<OsString>, value: impl Into<OsString>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }

    pub fn wrapper(mut self, wrapper: impl Into<PathBuf>) -> Self {
        self.wrapper = Some(wrapper.into());
        self
    }

    pub fn get_args(&self) -> &[OsString] {
        &self.args
    }

    pub fn get_env(&self) -> &HashMap<OsString, OsString> {
        &self.env
    }

    pub fn get_wrapper(&self) -> Option<&Path> {
        self.wrapper.as_deref()
    }

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
