// SPDX-License-Identifier: Unlicense
//! Read-only Linux and WSL hardware probes.

use std::{
    env, fs,
    path::{Path, PathBuf},
};

use crate::{
    commands::CommandRunner,
    models::{GpuDevice, SystemReport, VulkanDevice},
};

/// Classify the current Linux kernel environment.
pub fn detect_platform(proc_version: &str, dxg_exists: bool) -> String {
    if dxg_exists || proc_version.to_lowercase().contains("microsoft") {
        "wsl".into()
    } else if proc_version.is_empty() {
        "unknown".into()
    } else {
        "linux".into()
    }
}

/// Identify a GPU vendor from a kernel or PCI description.
pub fn detect_vendor(description: &str) -> String {
    let value = description.to_lowercase();
    if value.contains("nvidia") {
        "nvidia".into()
    } else if value.contains("advanced micro")
        || value.contains("amd/ati")
        || value.contains(" amd ")
    {
        "amd".into()
    } else if value.contains("intel") {
        "intel".into()
    } else {
        "unknown".into()
    }
}

/// Parse `vulkaninfo --summary` without treating software Vulkan as hardware.
pub fn parse_vulkan_summary(output: &str) -> Vec<VulkanDevice> {
    let mut devices = Vec::new();
    let mut name = None;
    let mut device_type = None;
    let mut driver = None;
    let finish = |devices: &mut Vec<VulkanDevice>,
                  name: &mut Option<String>,
                  device_type: &mut Option<String>,
                  driver: &mut Option<String>| {
        if let Some(device_name) = name.take() {
            let software = ["llvmpipe", "lavapipe", "softpipe", "swiftshader"]
                .iter()
                .any(|word| device_name.to_lowercase().contains(word));
            let hardware = !software
                && !matches!(
                    device_type.as_deref(),
                    Some("PHYSICAL_DEVICE_TYPE_CPU") | Some("CPU")
                );
            devices.push(VulkanDevice {
                name: device_name,
                device_type: device_type.take(),
                driver: driver.take(),
                hardware,
            });
        }
    };
    for line in output.lines() {
        if line.starts_with("GPU") && line.ends_with(':') {
            finish(&mut devices, &mut name, &mut device_type, &mut driver);
        } else if let Some((key, value)) = line.split_once('=') {
            let value = value.trim().to_string();
            match key.trim() {
                "deviceName" => name = Some(value),
                "deviceType" => device_type = Some(value),
                "driverName" => driver = Some(value),
                _ => {}
            }
        }
    }
    finish(&mut devices, &mut name, &mut device_type, &mut driver);
    devices
}

/// Parse the model names printed by `nvidia-smi -L`.
pub fn parse_nvidia_smi_list(output: &str) -> Vec<String> {
    output
        .lines()
        .filter_map(|line| line.split_once(':').map(|(_, value)| value))
        .filter_map(|value| value.split(" (UUID").next())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect()
}

/// Collect the current Linux/WSL report using optional system probes.
pub fn diagnose(runner: &impl CommandRunner) -> SystemReport {
    let proc_version = read_text(Path::new("/proc/version"));
    let platform = detect_platform(&proc_version, Path::new("/dev/dxg").exists());
    let mut devices = sysfs_devices();
    if devices.is_empty() {
        devices = lspci_devices(runner);
    }
    let nvidia_smi = nvidia_smi_path();
    if devices.is_empty() {
        devices = nvidia_devices(runner, nvidia_smi.as_deref());
    }
    let available_commands = [
        "lspci",
        "vulkaninfo",
        "nvidia-smi",
        "ubuntu-drivers",
        "apt-get",
        "dnf",
        "pacman",
        "zypper",
        "apk",
    ]
    .iter()
    .filter(|command| command_available(command))
    .map(|command| (*command).to_string())
    .collect();
    let vulkan_devices = if command_available("vulkaninfo") {
        runner
            .run(
                &[String::from("vulkaninfo"), String::from("--summary")],
                std::time::Duration::from_secs(20),
            )
            .map(|result| parse_vulkan_summary(&result.stdout))
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    let package_managers = ["apt-get", "dnf", "pacman", "zypper", "apk"]
        .iter()
        .filter(|command| command_available(command))
        .map(|command| (*command).to_string())
        .collect();
    SystemReport {
        platform,
        distribution: distribution(),
        devices,
        vulkan_devices,
        nvidia_smi: nvidia_smi.is_some(),
        package_managers,
        available_commands,
    }
}

fn nvidia_smi_path() -> Option<String> {
    ["/usr/bin/nvidia-smi", "/usr/lib/wsl/lib/nvidia-smi"]
        .iter()
        .find(|path| Path::new(path).is_file())
        .map(|path| (*path).to_string())
        .or_else(|| command_available("nvidia-smi").then(|| "nvidia-smi".into()))
}

fn nvidia_devices(runner: &impl CommandRunner, executable: Option<&str>) -> Vec<GpuDevice> {
    let Some(executable) = executable else {
        return Vec::new();
    };
    let Ok(result) = runner.run(
        &[executable.into(), "-L".into()],
        std::time::Duration::from_secs(10),
    ) else {
        return Vec::new();
    };
    if result.return_code != 0 {
        return Vec::new();
    }
    parse_nvidia_smi_list(&result.stdout)
        .into_iter()
        .map(|model| GpuDevice {
            vendor: "nvidia".into(),
            model,
            pci_id: None,
            driver: None,
            source: "nvidia-smi".into(),
        })
        .collect()
}

fn read_text(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_default()
}

fn distribution() -> Option<String> {
    read_text(Path::new("/etc/os-release"))
        .lines()
        .find_map(|line| {
            let (key, value) = line.split_once('=')?;
            (key == "ID").then(|| value.trim_matches('"').to_string())
        })
}

fn command_available(command: &str) -> bool {
    let paths: Vec<PathBuf> = env::var_os("PATH")
        .map(|value| env::split_paths(&value).collect::<Vec<PathBuf>>())
        .unwrap_or_default();
    paths
        .into_iter()
        .map(|path| path.join(command))
        .any(|path| path.is_file())
}

fn sysfs_devices() -> Vec<GpuDevice> {
    let mut devices = Vec::new();
    let Ok(entries) = fs::read_dir("/sys/class/drm") else {
        return devices;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.starts_with("card") || name[4..].contains('-') {
            continue;
        }
        let path = entry.path().join("device");
        let vendor_id = read_text(&path.join("vendor"));
        let vendor = match vendor_id.trim() {
            "0x10de" => "nvidia".to_string(),
            "0x1002" => "amd".to_string(),
            "0x8086" => "intel".to_string(),
            _ => detect_vendor(&read_text(&path.join("modalias"))),
        };
        let driver = fs::read_link(path.join("driver")).ok().and_then(|link| {
            link.file_name()
                .map(|name| name.to_string_lossy().to_string())
        });
        devices.push(GpuDevice {
            vendor,
            model: name,
            pci_id: (!vendor_id.trim().is_empty()).then(|| vendor_id.trim().into()),
            driver,
            source: "sysfs".into(),
        });
    }
    devices
}

fn lspci_devices(runner: &impl CommandRunner) -> Vec<GpuDevice> {
    let Ok(result) = runner.run(
        &[String::from("lspci"), String::from("-nnk")],
        std::time::Duration::from_secs(10),
    ) else {
        return Vec::new();
    };
    if result.return_code != 0 {
        return Vec::new();
    }
    let mut devices = Vec::new();
    for line in result
        .stdout
        .lines()
        .filter(|line| !line.starts_with(char::is_whitespace))
    {
        if !(line.contains("VGA compatible controller")
            || line.contains("3D controller")
            || line.contains("Display controller"))
        {
            continue;
        }
        let vendor = detect_vendor(line);
        if vendor != "unknown" {
            devices.push(GpuDevice {
                vendor,
                model: line.trim().into(),
                pci_id: None,
                driver: None,
                source: "lspci".into(),
            });
        }
    }
    devices
}
