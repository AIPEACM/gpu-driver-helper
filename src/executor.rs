// SPDX-License-Identifier: Unlicense
//! Execute only an explicit, allowlisted repair plan.

use std::time::Duration;

use serde::Serialize;

use crate::{
    commands::{CommandResult, CommandRunner, SystemRunner},
    models::RepairPlan,
};

#[derive(Debug, Serialize)]
pub struct ApplyResult {
    pub applied: bool,
    pub message: String,
    pub commands: Vec<CommandResultJson>,
}

#[derive(Debug, Serialize)]
pub struct CommandResultJson {
    pub argv: Vec<String>,
    pub return_code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl From<CommandResult> for CommandResultJson {
    fn from(result: CommandResult) -> Self {
        Self {
            argv: result.argv,
            return_code: result.return_code,
            stdout: result.stdout,
            stderr: result.stderr,
        }
    }
}

/// Apply a plan only when the caller supplied the exact confirmation word.
pub fn apply_plan(
    plan: &RepairPlan,
    confirmation: &str,
    runner: &impl CommandRunner,
) -> ApplyResult {
    if confirmation != "YES" {
        return ApplyResult {
            applied: false,
            message: "Confirmation was not exactly YES.".into(),
            commands: Vec::new(),
        };
    }
    if plan.status != "READY" || plan.commands.is_empty() {
        return ApplyResult {
            applied: false,
            message: plan.message.clone(),
            commands: Vec::new(),
        };
    }
    let mut results = Vec::new();
    for command in &plan.commands {
        let result = match runner.run(&command.argv, Duration::from_secs(900)) {
            Ok(result) => result,
            Err(error) => {
                return ApplyResult {
                    applied: false,
                    message: error.to_string(),
                    commands: results,
                }
            }
        };
        let failed = result.return_code != 0;
        results.push(result.into());
        if failed {
            return ApplyResult {
                applied: false,
                message: "A proposed command failed.".into(),
                commands: results,
            };
        }
    }
    ApplyResult {
        applied: true,
        message: "All approved commands completed.".into(),
        commands: results,
    }
}

/// Apply a plan with the production process runner.
pub fn apply_with_system_runner(plan: &RepairPlan, confirmation: &str) -> ApplyResult {
    apply_plan(plan, confirmation, &SystemRunner)
}
