# StylusPort: A guide to porting Solana Programs to Stylus Contracts

A comprehensive handbook and AI-augmented CLI tool for developers transitioning from Solana Programs to Stylus Contracts.

## Project Structure

- **`handbook/`** - mdbook source files containing the complete guide for porting Solana Programs to Stylus Contracts
- **`cli/`** - AI-augmented tool that ingests the handbook content and provides users with contextual insights and assistance

## Prerequisites

### Recommended: Nix Package Manager

The easiest way to get started is with the Nix package manager. If you don't have Nix installed:

```bash
curl --proto '=https' --tlsv1.2 -sSf -L https://install.determinate.systems/nix | sh -s -- install
```

For more installation options, visit the [Determinate Systems Nix Installer](https://github.com/DeterminateSystems/nix-installer).

### Alternative: Docker with Nix

If you prefer not to install Nix directly:

```bash
docker run -it --rm -v $(pwd):/workspace nixos/nix:latest
cd /workspace
nix develop
```

### Manual Installation

If you prefer to install dependencies manually, you'll need:

- **Rust** (latest stable) - Install via [rustup](https://rustup.rs/)
- **mdbook** - `cargo install mdbook`

## Getting Started

### With Nix (Recommended)

1. Clone the repository and enter the development shell:
   ```bash
   git clone <repository-url>
   cd stylusport
   nix develop
   ```

2. Build the project:
   ```bash
   make build
   ```

3. Serve the handbook locally:
   ```bash
   make serve-book
   ```

The handbook will be available at `http://localhost:3000`.

### Manual Setup

1. Ensure you have Rust and mdbook installed
2. Build the handbook:
   ```bash
   mdbook build handbook/
   ```
3. Build the CLI tool:
   ```bash
   cargo build --release --package stylus-port-cli
   ```

## Available Commands

- `make build` - Build both the handbook and CLI tool
- `make build-book` - Build only the handbook
- `make build-cli` - Build only the CLI tool
- `make serve-book` - Serve the handbook locally for development

## Development

The project uses pre-commit hooks for code quality:

- **clippy** - Rust linting with warnings treated as errors
- **rustfmt** - Rust code formatting
- **vale** - Spelling and prose linting

These hooks are automatically installed when you enter the Nix development shell.

