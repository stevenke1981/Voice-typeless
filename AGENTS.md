# Voice-typeless Agent Guide

<!-- codebase-memory-mcp:start -->
# Codebase Knowledge Graph (codebase-memory-mcp)

This project uses codebase-memory-mcp to maintain a knowledge graph of the codebase.
Always prefer MCP graph tools over grep, glob, or file search for code discovery.

## Priority Order

1. `search_graph` - find functions, classes, routes, and variables
2. `trace_path` - inspect callers, callees, and data flow
3. `get_code_snippet` - read a known function or class
4. `query_graph` - run complex Cypher queries
5. `get_architecture` - review the high-level project structure

Use text search only for literals, errors, configuration, documentation, or when the graph is insufficient.
<!-- codebase-memory-mcp:end -->

## Required Reading

Before implementation work, read:

- `docs/agent.md` - product specification and agent responsibilities
- `docs/architecture.md` - current Rust/Tauri/Svelte architecture
- `docs/knowledge-graph.md` - generated architecture map and hotspots
- `docs/lessons.md` - append-only implementation lessons

## Project Rules

- The reusable core is the Rust crate in `core-rs/`; there is no Go sidecar.
- Keep Tauri commands thin and place reusable behavior in `vtl-core`.
- Do not commit `target/`, `dist/`, model binaries, or local screenshots.
- Run Rust formatting and tests plus frontend checks before committing.
- Re-index codebase-memory after structural code changes and commit `.codebase-memory/graph.db.zst`.
