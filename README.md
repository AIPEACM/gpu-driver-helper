# gpu-driver-helper

Unlicensed native Linux and WSL GPU/Vulkan diagnosis and repair planning.

## Commands

Build:

```bash
cargo build --release
```

Commands:

```bash
gpu-driver-helper diagnose --json
gpu-driver-helper plan --json
gpu-driver-helper apply
gpu-driver-helper verify
```

No arbitrary shell command is accepted. WSL detection prevents Linux NVIDIA
kernel-driver installation; WSL GPU drivers belong to the Windows side.

## Status

The helper provides deterministic Linux/WSL diagnosis, structured package
plans, explicit `YES` confirmation, and JSON output for the image2x caller.
Vendor-specific download recipes require a pinned official source and checksum.

## License

This helper project is released under the Unlicense. Files copied from
`../vendor` retain their original upstream licenses and are not relicensed by
this project.
