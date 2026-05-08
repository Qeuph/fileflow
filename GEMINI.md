# FileFlow Project Context

FileFlow is an autonomous file orchestration platform designed for predictable, auditable, and self-healing file manipulation workflows. It features a visual pipeline editor, a domain-specific language (FlowLang), and an immutable ledger for time-travel capabilities.

## Project Overview

### Architecture
- **Monorepo Structure:** Managed with Rust workspaces and pnpm.
- **Backend (Rust):**
    - `fileflow-engine`: The core orchestrator. Includes a gRPC server, topological scheduler, and AST-to-pipeline compiler.
    - `fileflow-flowlang`: Domain-specific language (DSL) for file operations. Uses `pest` for parsing.
    - `fileflow-journal`: Immutable ledger using Merkle DAGs and SQLite to record all state transitions.
    - `fileflow-hyperquery`: Semantic selection engine for AI-powered file searching and filtering.
    - `fileflow-vfs`: Virtual File System for snapshot isolation and sandboxing.
    - `fileflow-worker-runtime`: Execution environment for pipeline tasks.
- **Frontend (SvelteKit + Tauri):**
    - `studio-ui`: A desktop studio for visual pipeline editing, featuring a node-graph editor (`svelte-flow`).
- **Interfaces:**
    - **gRPC:** Defined in `proto/orchestrator.proto` for communication between the Studio UI and the Orchestrator.
    - **CLI:** A command-line interface for running pipelines and managing snapshots.

### Core Technologies
- **Rust:** Tokio, tonic (gRPC), SQLite, BLAKE3, Pest, Clap.
- **Frontend:** TypeScript, Svelte 5, Tauri, Vite, `@xyflow/svelte`.

---

## Building and Running

### Prerequisites
- Rust (latest stable)
- Node.js (>=20) and pnpm

### Backend (Rust)
- **Build:** `cargo build`
- **Run CLI:** `cargo run --bin fileflow -- <COMMAND>`
    - Example: `cargo run --bin fileflow -- run --pipeline test.flow --target test_dir`
    - Example: `cargo run --bin fileflow -- snapshot --dir test_dir`
- **Run Orchestrator Server:** `cargo run --bin orchestrator`
- **Test:** `cargo test`

### Frontend (Tauri/Svelte)
- **Install Dependencies:** `pnpm install`
- **Run Dev Mode (with Tauri):** `pnpm --filter studio-ui tauri dev`
- **Build:** `pnpm --filter studio-ui build`

---

## Development Conventions

### Code Style
- **Rust:** Adhere to standard Rust idioms. Use `anyhow` for error handling in applications and `thiserror` for library errors.
- **Frontend:** Use Svelte 5 runes and TypeScript.

### Project Structure
- `packages/`: Contains all workspace members.
- `proto/`: Contains gRPC service definitions.
- `test_dir/`: Sample directory for testing file operations.

### Journaling & Safety
- All file operations MUST be recorded in the `journal`.
- Operations should ideally run within a `vfs` sandbox before being materialized.
- Pre-state and post-state hashes (Merkle roots) are used for verification.

### FlowLang
- The DSL grammar is defined in `packages/flowlang/src/flowlang.pest`.
- Supports `pipeline`, `let` bindings, `for` loops, and call chains using `->`.

---

## Roadmap Status
The project is currently in **Phase 0/1 (Genesis Foundation & Core Engines)**.
- **Done:** Monorepo setup, Basic Journal (SQLite), Basic HyperQuery, FlowLang Parser, gRPC scaffold.
- **In Progress:** VFS Copy-on-Write, Scheduler execution, Studio UI skeleton.
- **Planned:** AI-powered culling, Time-travel debugging, WASM-based plugins.

For more details, see `DEVELOPMENT_PLAN.md`.
