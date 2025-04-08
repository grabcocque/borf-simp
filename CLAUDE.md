# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Development Commands
- 🔨 Build: `npm run build` (TypeScript + Rust)
- 🧶 TypeScript only: `npm run compile`
- 📦 Rust only: `cargo build --target wasm32-unknown-unknown`
- 🔄 Watch: `npm run watch`
- 🧹 Lint: `npm run lint`
- ⚙️ Generate WIT bindings: `npm run wit-bindgen`
- 🚀 Run: `vscode:samples.wasm-component-model.run` (VS Code command)

## Code Style Guidelines
- **TypeScript**: Strict mode with semi-colons required; camelCase for variables; no implicit returns/any
- **Rust**: Use 2021 edition; use RefCell for mutable borrows across API boundaries
- **Formatting**: Tabs for indentation, curly braces required for all blocks
- **Naming**: camelCase or PascalCase for imports
- **Error Handling**: Unwrap is acceptable in this sample project; handle Option with pattern matching
- **WIT Interface**: Follow Component Model resource patterns for cross-language calls