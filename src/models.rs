// SPDX-License-Identifier: Unlicense
//! Data exchanged by the Linux and WSL probes and repair planner.

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct GpuDevice {
    pub vendor: String,
    pub model: String,
    pub pci_id: Option<String>,
    pub driver: Option<String>,
    pub source: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct VulkanDevice {
    pub name: String,
    pub device_type: Option<String>,
    pub driver: Option<String>,
    pub hardware: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct SystemReport {
    pub platform: String,
    pub distribution: Option<String>,
    pub devices: Vec<GpuDevice>,
    pub vulkan_devices: Vec<VulkanDevice>,
    pub nvidia_smi: bool,
    pub package_managers: Vec<String>,
    pub available_commands: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CommandPlan {
    pub argv: Vec<String>,
    pub reason: String,
    pub privileged: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct RepairPlan {
    pub status: String,
    pub message: String,
    pub commands: Vec<CommandPlan>,
}
