// SPDX-License-Identifier: Unlicense

fn main() {
    match gpu_driver_helper::cli::run() {
        Ok(code) => std::process::exit(code),
        Err(error) => {
            eprintln!("error: {error}");
            std::process::exit(101);
        }
    }
}
