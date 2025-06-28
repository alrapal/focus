#! /bin/bash

# To change if not default esp32 rust toolchain installation
# Import the rust toolchain using default script location
. $HOME/export-esp.sh

# Build for esp32s3 target
cargo build --target xtensa-esp32s3-none-elf