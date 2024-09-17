ReferenceRover is currently under active development! The most interesting features (namely, semantic search) are functional, and we are preparing for alpha testing - feel free to reach out if interested in participating.

It project is currently source-available, but not open source (though that's in the roadmap).

# ReferenceRover

ReferenceRover is a tool for collecting, organizing, and viewing artistic reference materials.

Features (and planned features) include:

* Natural language / semantic search of image data.
  * This is done locally via a ORT/ONNX, never touching our servers; your data and IP are yours. Index and search your local files, compatible with any machine running a DirectX compatible GPU.
  * This feature is functional, with some optimization work underway (notably: quantizing and spinning down inactive ONNX runtime sessions to free up VRAM, and providing alternative execution providers to support a wider variety of machines)
* Tagging system, with suggested tags (under development)
* Tight integration with tools such as PureRef and Miro (under development)
* Tracking source information, licensing, and other metadata for images (under development)
* Privacy first design philosophy.
  * User data will never be used to train generative image models. 

Build your visual library and unleash your creative potential!

# Development

## Quick Start

Use `pnpm tauri dev` to build and run the program.

Note that the files in `src-tauri/rover/models` and `src-tauri/rover/onnx-dll` must be copied into the `target/debug` and `target/release` directories manually, so that they are next to the executable.
For tests and examples, they must similarly be copied into the `deps/` and `examples/` directories as specified by the [ORT docs](https://docs.rs/ort/latest/ort/#windows). 
(TODO - Create a build script to handle this.)

To pass a CLI option to the *rust executable*, precede them with `-- --` e.g. `pnpm tauri dev -- -- -p` or `pnpm tauri dev --release -- -- -p`. Feel free to dig into why that's necessary for Tauri.

The `rover/.env` and `rover/diesel.toml` files need to be updated to reflect your development environment's paths before running e.g. `diesel migration run`.

TODO - Discuss how to package. Basically handled by Tauri.

### Windows

Requires MSVC 2017 Build Tools to be installed, so that linking with the ONNX runtime providers (e.g. DirectML.dll) can work.


### Linux

TODO

### ONNX graph generation

See [the exporter](https://github.com/jalberse/CLIP-to-onnx-converter).