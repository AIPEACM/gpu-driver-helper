// SPDX-License-Identifier: Unlicense

use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use gpu_driver_helper::{
    commands::{CommandResult, CommandRunner},
    executor::apply_plan,
    models::{CommandPlan, GpuDevice, RepairPlan, SystemReport},
    probe::{detect_platform, detect_vendor, parse_nvidia_smi_list, parse_vulkan_summary},
    recipes::plan_repair,
};

#[test]
fn detects_wsl_and_vendor() {
    assert_eq!(detect_platform("Linux microsoft WSL2", false), "wsl");
    assert_eq!(
        detect_vendor("Advanced Micro Devices, Inc. [AMD/ATI]"),
        "amd"
    );
}

#[test]
fn classifies_software_and_hardware_vulkan() {
    let output = "GPU0:\n deviceType = PHYSICAL_DEVICE_TYPE_CPU\n deviceName = llvmpipe\nGPU1:\n deviceType = PHYSICAL_DEVICE_TYPE_DISCRETE_GPU\n deviceName = NVIDIA T1200\n driverName = Dozen\n";
    let devices = parse_vulkan_summary(output);
    assert_eq!(devices.len(), 2);
    assert!(!devices[0].hardware);
    assert!(devices[1].hardware);
}

#[test]
fn parses_wsl_nvidia_device_list() {
    let devices = parse_nvidia_smi_list("GPU 0: NVIDIA T1200 Laptop GPU (UUID: GPU-test)\n");
    assert_eq!(devices, vec!["NVIDIA T1200 Laptop GPU"]);
}

#[test]
fn wsl_plan_never_installs_linux_driver() {
    let report = SystemReport {
        platform: "wsl".into(),
        distribution: Some("debian".into()),
        devices: vec![GpuDevice {
            vendor: "nvidia".into(),
            model: "T1200".into(),
            pci_id: None,
            driver: Some("dxgkrnl".into()),
            source: "sysfs".into(),
        }],
        vulkan_devices: Vec::new(),
        nvidia_smi: true,
        package_managers: vec!["apt-get".into()],
        available_commands: vec!["apt-get".into()],
    };
    let plan = plan_repair(&report);
    assert_eq!(plan.status, "WSL_WINDOWS_DRIVER_PRESENT_VULKAN_MISSING");
    assert!(plan.commands.is_empty());
}

#[derive(Clone)]
struct FakeRunner {
    calls: Arc<Mutex<Vec<Vec<String>>>>,
}

impl CommandRunner for FakeRunner {
    fn run(
        &self,
        argv: &[String],
        _timeout: Duration,
    ) -> Result<CommandResult, gpu_driver_helper::commands::CommandError> {
        self.calls
            .lock()
            .expect("test mutex is not poisoned")
            .push(argv.to_vec());
        Ok(CommandResult {
            argv: argv.to_vec(),
            return_code: 0,
            stdout: String::new(),
            stderr: String::new(),
        })
    }
}

#[test]
fn apply_requires_exact_confirmation() {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let runner = FakeRunner {
        calls: calls.clone(),
    };
    let plan = RepairPlan {
        status: "READY".into(),
        message: "test".into(),
        commands: vec![CommandPlan {
            argv: vec!["sudo".into(), "true".into()],
            reason: "test".into(),
            privileged: true,
        }],
    };
    assert!(!apply_plan(&plan, "yes", &runner).applied);
    assert!(calls.lock().expect("test mutex is not poisoned").is_empty());
    assert!(apply_plan(&plan, "YES", &runner).applied);
}
