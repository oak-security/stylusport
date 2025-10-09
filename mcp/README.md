# StylusPort::Solana MCP Server

An MCP (Model Context Protocol) server for migrating Solana programs to Stylus contracts on Arbitrum. Provides tools, resources, and prompts to assist LLM agents in planning and executing migrations.

## Overview

This server exposes:
- **Tools**: Detect Solana program types, generate Stylus boilerplate, search migration handbook
- **Resources**: Handbook chapters covering migration patterns and case studies
- **Prompts**: Guided migration planning workflow

## Architecture

- `src/main.rs` - Entry point and message dispatcher
- `src/server.rs` - Multi-threaded stdio-based MCP server implementation
- `src/handler.rs` - MCP request handlers (initialize, ping, list/read resources, etc.)
- `src/tools.rs` - Tool definitions and implementations
- `src/resources.rs` - Resrouce definigions (Handbook chapters) and search infrastructure
- `src/resources/bm25.rs` - BM25 ranking algorithm for resource search
- `src/prompts.rs` - Prompt definitions

## Tools

### `detect_solana_program_kind`
Analyzes a Cargo.toml to determine if a Solana program is "native" or "anchor" framework.

### `generate_stylus_contract_cargo_manifest`
Generates a Cargo.toml for a new Stylus contract with appropriate dependencies.

### `generate_stylus_contract_main_rs`
Generates the main.rs boilerplate for a Stylus contract.

### `search_handbook`
Searches the migration handbook using BM25 ranking, returning resource URIs in descending relevance order. The search implementation handles both prose and code tokens with identifier splitting for Rust-specific patterns.

## Resources

The server exposes 13 handbook chapters covering:
- Program structure comparison
- State storage patterns
- Access control migration
- External calls
- Token operations (native, fungible, non-fungible)
- Errors and events
- Testing and debugging
- Gas optimization
- Security considerations
- Complete case study (Bonafida token vesting)

All resources use `file://` URIs and markdown content.

## Prompts

### `plan_solana_program_stylus_migration`
Provides a structured workflow for analyzing a Solana program and creating a comprehensive migration plan. Guides the agent through codebase analysis, handbook consultation, design decisions, and phased implementation planning.

## Development

Build and run:
```bash
cargo build -p stylusport-mcp-server --release
cargo run -p stylusport-mcp-server --release
```

Test:
```bash
cargo -p stylusport-mcp-server test
```

Inspect with MCP inspector:
```bash
npx @modelcontextprotocol/inspector cargo run -p stylusport-mcp-server --release
```

### Inspector Screenshots

**Resources**

![MCP Inspector Resources Page](./docs/mcp-inspector-resources.png)

**Prompts**

![MCP Inspector Prompts Page](./docs/mcp-inspector-prompts.png)

**Tools**

![MCP Inspector Tools Page](./docs/mcp-inspector-tools.png)

## Implementation Notes

- Protocol version negotiation follows MCP 2025-06-18 spec
- Server uses 4 worker threads for concurrent request handling
- BM25 implementation includes Rust-specific tokenization (handles `::`, `_`, PascalCase, snake_case)
- Handbook search scores both prose and code tokens with field-specific weights
- All responses use JSON-RPC 2.0 format over stdio

## Dependencies

The goal is to reduce dependencies as much as possible.

- `rust-mcp-schema 0.7.4` - MCP protocol types (in turn only depends on serde/serde-json)
- `serde_json 1.0.145` - JSON serialization

Both dependencies are patched to use GitHub sources to reduce supply chain surface area.
