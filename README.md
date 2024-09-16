# VizLib

VizLib is a tool for collecting, organizing, and viewing artistic reference materials. 

Build your visual library and unleash your creative potential!


# Development

## Quick Start

Use `pnpm taur dev` to build and run the program. Use `pnpm tauri dev` to 

Note that the files in `src-tauri/rover/models` and `src-tauri/rover/onnx-dll` must be copied into the `target/debug` and `target/release` directories manually, so that they are next to the executable. TODO - Create a build script to handle this.

To pass a CLI option to the *rust executable*, precede them with `-- --` e.g. `pnpm tauri dev -- -- -p` or `pnpm tauri dev --release -- -- -p`. Feel free to dig into why that's necessary for Tauri.

The `rover/.env` and `rover/diesel.toml` files need to be updated to reflect your development environment's paths before running e.g. `diesel migration run`.

TODO - Discuss how to package. Basically handled by Tauri.

### Windows

Requires MSVC 2017 Build Tools to be installed, so that linking with the ONNX runtime providers (e.g. DirectML.dll) can work.

### Linux

TODO
