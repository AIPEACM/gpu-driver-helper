// SPDX-License-Identifier: Unlicense
//! Command-line interface for diagnosis, planning, applying, and verification.

use std::io::{self, Write};

use clap::{Parser, Subcommand};

use crate::{
    commands::SystemRunner, executor::apply_with_system_runner, probe::diagnose,
    recipes::plan_repair,
};

#[derive(Debug, Parser)]
#[command(
    name = "gpu-driver-helper",
    version,
    about = "Linux and WSL GPU/Vulkan support helper"
)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Diagnose {
        #[arg(long)]
        json: bool,
    },
    Plan {
        #[arg(long)]
        json: bool,
    },
    Apply {
        #[arg(long)]
        confirm: Option<String>,
        #[arg(long)]
        json: bool,
    },
    Verify {
        #[arg(long)]
        json: bool,
    },
}

/// Run the requested CLI command and return its process exit code.
pub fn run() -> Result<i32, Box<dyn std::error::Error>> {
    let args = Args::parse();
    let report = diagnose(&SystemRunner);
    match args.command {
        Command::Diagnose { json } => {
            print_value(&report, json)?;
            Ok(0)
        }
        Command::Plan { json } => {
            print_value(&plan_repair(&report), json)?;
            Ok(0)
        }
        Command::Apply { confirm, json } => {
            let plan = plan_repair(&report);
            let confirmation = match confirm {
                Some(value) => value,
                None => {
                    print_value(&plan, false)?;
                    print!("Type YES to execute the proposed command: ");
                    io::stdout().flush()?;
                    let mut value = String::new();
                    io::stdin().read_line(&mut value)?;
                    value.trim().to_string()
                }
            };
            let result = apply_with_system_runner(&plan, &confirmation);
            print_value(&result, json)?;
            Ok(if result.applied { 0 } else { 1 })
        }
        Command::Verify { json } => {
            let passed = report.vulkan_devices.iter().any(|device| device.hardware);
            if json {
                println!(
                    "{}",
                    serde_json::json!({ "passed": passed, "report": report })
                );
            } else {
                println!(
                    "{}",
                    if passed {
                        "hardware Vulkan: pass"
                    } else {
                        "hardware Vulkan: fail"
                    }
                );
            }
            Ok(if passed { 0 } else { 1 })
        }
    }
}

fn print_value<T: serde::Serialize>(value: &T, _json: bool) -> Result<(), serde_json::Error> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}
