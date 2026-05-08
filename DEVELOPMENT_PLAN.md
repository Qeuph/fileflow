# FileFlow Development Plan

A Phased Roadmap for AI Agent-Driven Construction

---

## Phase 0 · Genesis Foundation [IN PROGRESS]

**Goal**: Establish the skeleton system, build system, core data structures, and the immutable journal.

### 0.1 Project Structure & Monorepo Setup [DONE]
*   [x] Init monorepo with packages: `engine`, `worker-runtime`, `journal`, `hyperquery`, `flowlang`, `studio-ui`, `plugins`, `vfs`.
*   [x] Configure Rust and TypeScript workspaces.
*   [x] CI baseline (GitHub Actions scaffolded).

### 0.2 SafeGuard Ledger & Snapshot Isolation (Alpha) [IN PROGRESS]
*   [x] Define `.flowchain` block format (CBOR/JSON in `journal`).
*   [x] Implement Merkle DAG over file hashes (Basic Merkle Tree in `journal`).
*   [x] Implement SQLite journal store (`journal`).
*   [x] Implement Snapshot and Sandbox primitives (`vfs`).
*   [ ] Copy-on-Write Virtual FS (Currently using library-based `vfs`, FUSE/ProjFS driver pending).
*   [ ] Time-Travel Debugging CLI.

### 0.3 Orchestrator Shell & gRPC API [IN PROGRESS]
*   [x] Define protobuf service `Orchestrator`.
*   [x] Implement minimal Rust orchestrator server with gRPC.
*   [x] Basic topological scheduler in `engine`.
*   [ ] Headless daemon mode.

---

## Phase 1 · The Core Engines [IN PROGRESS]

**Goal**: Build the three foundational subsystems—HyperQuery, FlowForge engine, and FlowLang.

### 1.1 HyperQuery · Semantic Selection [IN PROGRESS]
*   [x] Basic File Indexer (SQLite metadata in `hyperquery`).
*   [x] Basic Query Engine (SQL-based filters).
*   [ ] Vector Embedding Pipeline (On-device ONNX integration).
*   [ ] NL-to-query parser.
*   [ ] Dynamic Collections (Event-based re-evaluation).

### 1.2 FlowLang Compiler & Runtime [IN PROGRESS]
*   [x] Language Specification & Pest Grammar (`flowlang`).
*   [x] Basic Parser (AST generation).
*   [ ] AST to Pipeline compilation (Started in `engine`).
*   [ ] WASM Compilation & Sandboxed Execution (wasmtime).
*   [ ] LSP Server.

### 1.3 FlowForge Pipeline Engine [IN PROGRESS]
*   [x] Pipeline DAG representation.
*   [x] Topological Scheduler.
*   [ ] Adaptive Branching & Retry policies.
*   [ ] Full Node Type set (Rename, Transcode, etc. as WASM plugins).

---

## Phase 2 · The Desktop Studio [SKELETON]

**Goal**: Build the user interface with visual pipeline editing and temporal preview.

### 2.1 Studio Shell (Tauri-based) [IN PROGRESS]
*   [x] Scaffold Tauri app with SvelteKit.
*   [ ] gRPC/WebSocket communication with Orchestrator.
*   [ ] Multi-window support.

### 2.2 Flow Canvas & Visual Editor [NOT STARTED]
*   [ ] Node-graph editor implementation.
*   [ ] Bidirectional FlowLang sync.

### 2.3 Temporal Preview Grid & Semantic Diff [NOT STARTED]
*   [ ] Virtual spreadsheet for file timeline.
*   [ ] Semantic Diff viewers.

### 2.4 Source Nexus & Command Bar [NOT STARTED]

---

## Phase 3 · Resilience, Collaboration & Intelligence [NOT STARTED]

*   **3.1 Self-Healing & Failure Recovery**: Logic placeholders exist in `engine`, requires full `vfs` integration.
*   **3.2 Multi-User Presence & Collaboration**: CRDT layer pending.
*   **3.3 AI-Powered Culling & Smart Alerts**: Mocked in `engine`, requires ONNX models.
*   **3.4 Plugin Marketplace & WASM Distribution**.

---

## Phase 4 · Enterprise Scale & Ecosystem [NOT STARTED]

*   **4.1 Headless Daemon & Watchdog**.
*   **4.2 Federated Query & Mesh**.
*   **4.3 Audit Export & Compliance**.
*   **4.4 Performance Optimization (io_uring, B-tree index)**.

---

## Technology Stack Summary

| Layer | Components | Status |
| :--- | :--- | :--- |
| **Core Engine** | Rust, Tokio, gRPC (tonic), protobuf | Core ready |
| **Journal & FS** | BLAKE3, SQLite, Merkle Tree, `vfs` lib | Base ready |
| **HyperQuery** | SQLite, Pest, (Pending: ONNX, sqlite-vec) | Basic ready |
| **FlowLang** | Pest Parser, AST, (Pending: WASM) | Parser ready |
| **UI** | Tauri, SvelteKit, (Pending: Svelte Flow) | Skeleton ready |
