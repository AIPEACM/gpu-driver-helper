// SPDX-License-Identifier: Unlicense
//! Bounded argv-only process execution.

use std::{
    io::Read,
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use nix::{
    sys::signal::{kill, Signal},
    unistd::{setpgid, Pid},
};
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct CommandResult {
    pub argv: Vec<String>,
    pub return_code: i32,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Error)]
pub enum CommandError {
    #[error("command is empty")]
    Empty,
    #[error("command failed to start: {0}")]
    Start(#[source] std::io::Error),
    #[error("command output failed: {0}")]
    Output(#[source] std::io::Error),
}

pub trait CommandRunner {
    fn run(&self, argv: &[String], timeout: Duration) -> Result<CommandResult, CommandError>;
}

pub struct SystemRunner;

impl CommandRunner for SystemRunner {
    fn run(&self, argv: &[String], timeout: Duration) -> Result<CommandResult, CommandError> {
        if argv.is_empty() || argv.iter().any(String::is_empty) {
            return Err(CommandError::Empty);
        }
        let mut command = Command::new(&argv[0]);
        command
            .args(&argv[1..])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = command.spawn().map_err(CommandError::Start)?;

        // A process group lets timeout cleanup terminate descendants too. Without
        // it, a package manager child could survive after its parent is killed.
        let pid = Pid::from_raw(child.id() as i32);
        let _ = setpgid(pid, pid);
        let deadline = Instant::now() + timeout;
        let return_code = loop {
            if let Some(status) = child.try_wait().map_err(CommandError::Start)? {
                break status.code().unwrap_or(128);
            }
            if Instant::now() >= deadline {
                let _ = kill(Pid::from_raw(-(child.id() as i32)), Signal::SIGKILL);
                let _ = child.kill();
                let _ = child.wait();
                break 124;
            }
            thread::sleep(Duration::from_millis(50));
        };
        let (stdout, stderr) = read_output(&mut child)?;
        Ok(CommandResult {
            argv: argv.to_vec(),
            return_code,
            stdout,
            stderr,
        })
    }
}

fn read_output(child: &mut Child) -> Result<(String, String), CommandError> {
    let mut stdout = String::new();
    let mut stderr = String::new();
    if let Some(mut pipe) = child.stdout.take() {
        pipe.read_to_string(&mut stdout)
            .map_err(CommandError::Output)?;
    }
    if let Some(mut pipe) = child.stderr.take() {
        pipe.read_to_string(&mut stderr)
            .map_err(CommandError::Output)?;
    }
    Ok((stdout, stderr))
}
