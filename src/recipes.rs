// SPDX-License-Identifier: Unlicense
//! Allowlisted Linux repair plans.

use crate::models::{CommandPlan, RepairPlan, SystemReport};

/// Select a safe package action from a detected system report.
pub fn plan_repair(report: &SystemReport) -> RepairPlan {
    if report.devices.is_empty() {
        return no_plan("NO_DEVICE", "No supported GPU was detected.");
    }
    if report.platform == "wsl" {
        let status = if report.nvidia_smi {
            "WSL_WINDOWS_DRIVER_PRESENT_VULKAN_MISSING"
        } else {
            "WSL_WINDOWS_DRIVER_REQUIRED"
        };
        return no_plan(status, "WSL GPU driver state is controlled by the Windows side; no Linux NVIDIA kernel driver will be installed.");
    }
    let Some(manager) = report.package_managers.first() else {
        return no_plan(
            "PACKAGE_PROVIDER_UNAVAILABLE",
            "No supported package manager was detected.",
        );
    };
    let vendor = &report.devices[0].vendor;
    if vendor == "nvidia"
        && report.distribution.as_deref() == Some("ubuntu")
        && report
            .available_commands
            .iter()
            .any(|command| command == "ubuntu-drivers")
    {
        return ready(
            "Use the distribution's signed NVIDIA driver selector.",
            vec!["sudo", "ubuntu-drivers", "install"],
        );
    }
    let packages: &[&str] = match (manager.as_str(), vendor.as_str()) {
        ("apt-get", "amd") | ("apt-get", "intel") => &["mesa-vulkan-drivers", "vulkan-tools"],
        ("dnf", "amd") | ("dnf", "intel") => &["mesa-vulkan-drivers", "vulkan-tools"],
        ("pacman", "amd") => &["vulkan-tools", "vulkan-radeon"],
        ("pacman", "intel") => &["vulkan-tools", "vulkan-intel"],
        ("zypper", "amd") | ("zypper", "intel") => &["Mesa-vulkan-drivers", "vulkan-tools"],
        ("apk", "amd") | ("apk", "intel") => &["mesa-vulkan-lavapipe", "vulkan-tools"],
        _ => &[],
    };
    if packages.is_empty() {
        return no_plan(
            "DRIVER_PACKAGE_UNAVAILABLE",
            "No verified repair recipe is available for this GPU and package manager.",
        );
    }
    let mut argv = vec![
        "sudo".to_string(),
        manager.clone(),
        "install".to_string(),
        "-y".to_string(),
    ];
    argv.extend(packages.iter().map(|package| (*package).to_string()));
    RepairPlan {
        status: "READY".into(),
        message: format!("Install Vulkan user-space packages for {vendor}."),
        commands: vec![CommandPlan {
            argv,
            reason: "Use packages from the detected distribution.".into(),
            privileged: true,
        }],
    }
}

fn no_plan(status: &str, message: &str) -> RepairPlan {
    RepairPlan {
        status: status.into(),
        message: message.into(),
        commands: Vec::new(),
    }
}

fn ready(message: &str, argv: Vec<&str>) -> RepairPlan {
    RepairPlan {
        status: "READY".into(),
        message: message.into(),
        commands: vec![CommandPlan {
            argv: argv.into_iter().map(str::to_string).collect(),
            reason: message.into(),
            privileged: true,
        }],
    }
}
