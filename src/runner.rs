use std::time::{Duration, SystemTime};

use execute::shell;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RunnerError {
    #[error("Error running command {0}:\n {1}")]
    RuntimeError(String, String),
}

pub struct CommandInfo {
    pub time: Duration,
    pub command: String,
    pub stdout: String,
    pub stderr: String,
}

impl CommandInfo {
    pub fn new(command: String, time: Duration, stdout: String, stderr: String) -> Self {
        Self {
            time,
            command,
            stdout,
            stderr,
        }
    }
}

pub fn run(command: String) -> Result<CommandInfo, RunnerError> {
    let start = SystemTime::now();

    let output = match shell(&command).output() {
        Ok(out) => out,
        Err(e) => return Err(RunnerError::RuntimeError(command, e.to_string())),
    };

    let stdout = String::from_utf8(output.stdout).unwrap_or("Failed to parse stdout".to_string());
    let stderr = String::from_utf8(output.stderr).unwrap_or("Failed to parse stderr".to_string());

    println!("{}\n{}", stdout, stderr);

    let info = CommandInfo::new(
        command.to_owned(),
        start.elapsed().unwrap_or_default(),
        stdout,
        stderr,
    );

    return Ok(info);
}
