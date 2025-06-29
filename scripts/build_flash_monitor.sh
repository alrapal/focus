#! /bin/bash

# To change if not default esp32 rust toolchain installation
# Import the rust toolchain using default script location
. $HOME/export-esp.sh

# run with defined target, the runner defined in .cargo/config.toml will flash the firmware and start monitoring
cargo run --target xtensa-esp32s3-none-elf