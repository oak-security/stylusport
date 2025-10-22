# StylusPort: A guide to porting Solana Programs to Stylus Contracts

A comprehensive handbook and Model Context Protocol (MCP) server for developers transitioning from Solana Programs to Stylus Contracts.

## ⚠️ Disclaimer

An internal code/security review has been completed — see [StylusPort Internal Review v1.0 (2025-10-22)](https://github.com/oak-security/audit-reports/blob/main/StylusPort/2025-10-22%20Audit%20Report%20-%20StylusPort%20Internal%20Review%20v1.0.pdf). This was **not** a third-party audit and does **not** eliminate risk. The handbook and accompanying code examples are provided for educational/reference use; the examples have **not** been extensively tested. Perform your own review, threat modeling, and rigorous testing before using any concepts or code in production. No warranties or guarantees are provided.

## Project Structure

- **`handbook/`** - mdbook source files containing the complete guide for porting Solana Programs to Stylus Contracts
- **[`mcp/`](./mcp/README.md)** - an MCP server that serves the handbook as resources as well as utility tools and prompts

## Development

### With Nix (Recommended)

The easiest way to get started is with the Nix package manager. If you don't have Nix installed:

```bash
curl --proto '=https' --tlsv1.2 -sSf -L https://install.determinate.systems/nix | sh -s -- install
```

For more installation options, visit the [Determinate Systems Nix Installer](https://github.com/DeterminateSystems/nix-installer).

Clone the repository and enter the development shell:

```bash
git clone <repository-url>
cd stylusport
nix develop
```

### Alternative: Docker with Nix

If you prefer not to install Nix directly:

```bash
docker run -v $(pwd):/workspace -w /workspace -p 3000:3000 --entrypoint /bin/sh -ti ghcr.io/nixos/nix -c \
   "git config --global --add safe.directory /workspace && nix develop --extra-experimental-features 'nix-command flakes'"
```

### Manual Installation

If you prefer to install dependencies manually, you'll need:

- **Rust** (latest stable) - Install via [rustup](https://rustup.rs/)
- **mdbook** - `cargo install mdbook`

> NOTE: You may require other transient dependencies to be installed on your system. 
> Only issues relating to the Nix configuration or the docker alternative are considered.

### Useful Commands

1. Build the handbook:
   ```bash
   mdbook build handbook/
   ```

1. Serve the handbook locally:
   ```bash
   mdbook serve handbook/

   # if running in the Nix Docker container
   mdbook serve --hostname 0.0.0.0 handbook/
   ```

The handbook will be available at `http://localhost:3000`.

1. Run tests for all stylus examples
   ```
   cargo test --package *-stylus
   ```

> NOTE: There is not official Nix support for the custom Solana Rust tools and toolchain.
> If you need to run the Solana example tests, check the [Solana toolchain install instructions](https://solana.com/docs/intro/installation).

The project uses pre-commit hooks for code quality:

- **clippy** - Rust code linting
- **rustfmt** - Rust code formatting
- **vale** - Spelling and prose linting

These hooks are automatically installed when you enter the Nix development shell.

