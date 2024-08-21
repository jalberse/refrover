# VizLib

VizLib is a tool for collecting, organizing, and viewing artistic reference materials. 

Build your visual library and unleash your creative potential!


# Development

## Quick Start

### Windows

Requires MSVC 2017 Build Tools to be installed, so that linking with the ONNX runtime providers (e.g. DirectML.dll) can work.

### Linux

TODO

## Project configuration

This project was built from [Tauri + Next.js Template](https://github.com/kvnxiao/tauri-nextjs-template/tree/main), which in turn is simply a nice configuration from [`create-next-app`](https://github.com/vercel/next.js/tree/canary/packages/create-next-app) and [`create tauri-app`](https://tauri.app/v1/guides/getting-started/setup).

- TypeScript frontend using Next.js React framework
- [TailwindCSS](https://tailwindcss.com/) as a utility-first atomic CSS framework
  - The example page in this template app has been updated to use only TailwindCSS
  - While not included by default, consider using
    [React Aria components](https://react-spectrum.adobe.com/react-aria/index.html)
    and/or [HeadlessUI components](https://headlessui.com/) for completely unstyled and
    fully accessible UI components, which integrate nicely with TailwindCSS
- Opinionated formatting and linting already setup and enabled
  - [ESLint](https://eslint.org/) for pure React + TypeScript linting, and
    [Biome](https://biomejs.dev/) for a combination of fast formatting, linting, and
    import sorting of JavaScript and TypeScript code
  - [clippy](https://github.com/rust-lang/rust-clippy) and
    [rustfmt](https://github.com/rust-lang/rustfmt) for Rust code
- GitHub Actions to check code formatting and linting for both TypeScript and Rust

This project uses [`pnpm`](https://pnpm.io/) as the Node.js dependency
manager.