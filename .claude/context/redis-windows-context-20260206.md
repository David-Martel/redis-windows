# Project Context: redis-windows
## ctx-redis-windows-20260206 | Schema v2.0

**Created:** 2026-02-06
**Branch:** main @ 0086b7d
**Status:** Clean (only gitignored obj/ artifacts pending)
**Pushed:** Yes, origin/main is up to date

---

## Project Summary

Redis for Windows with native LLM agent integration. A production-ready Redis server (upstream: redis-windows/redis-windows, now at 8.4.0) wrapped in a .NET 8 Windows Service, with an `agent-sync` Rust library for high-performance AI agent coordination.

## Recent Session Work

### Commits Made (this session, 8 clusters)
1. `747ae84` docs(claude): Fix dependency versions and inaccurate documentation
2. `50621cd` fix(config): Correct agent Redis port and add Windows/hybrid configs
3. `26cd516` feat(scripts): Add Redis service management and dual-instance scripts
4. `6a029a5` chore(agent-sync): Add CI/CD workflows and cross-compilation config
5. `e245b19` docs: Add agent-sync analysis and Redis hybrid deployment guides
6. `d8434bf` chore: Add taskmaster and editor configuration
7. `00788fe` chore: Remove deprecated RedisUtilities and untrack build artifacts
8. `0086b7d` Merge upstream/main (Redis 8.4.0 updates)

### Key Changes
- Merged 12 upstream commits (Redis 8.0.3 -> 8.4.0)
- Program.cs: Merged our logging/monitoring with upstream's graceful shutdown via redis-cli SHUTDOWN
- RedisService.csproj: Hosting.WindowsServices 8.0.0 -> 8.0.1
- Fixed all CLAUDE.md inaccuracies (dependency versions, bare python commands)
- Removed obj/ files from git tracking
- Deleted deprecated RedisUtilities.psm1

---

## Architecture

```
redis-windows/
├── Program.cs              # .NET 8 Windows Service (BackgroundService)
├── RedisService.csproj     # .NET 8 Worker SDK, win-x64
├── redis-agent.conf        # Primary config (port 6380, 2GB, LRU)
├── redis-windows.conf      # Standard Windows config (port 6380, 256MB)
├── redis.hybrid.conf       # Comprehensive reference config
├── build.ps1               # Main build script
├── build-modular.ps1       # Modular build approach
├── configure-agent-redis.ps1   # WSL/Windows switcher
├── install-redis-service.ps1   # Windows Service installer
├── manage-dual-redis.ps1       # Dual-instance orchestration
├── optimize-wsl-redis.sh       # WSL optimization
├── agent-sync/             # Rust library (23 source files)
│   ├── Cargo.toml          # redis 0.25, thiserror 1.0, tokio 1.35, bb8
│   ├── src/coordinator/    # TaskQueue, SharedContext, NotificationBus
│   ├── src/testing/        # Mock Redis, fixtures, helpers
│   ├── benches/            # Criterion benchmarks
│   └── examples/           # 4 usage examples
└── .claude/
    ├── CLAUDE.md           # Agent-sync coding standards
    └── context/            # This directory
```

## Decisions

| ID | Topic | Decision | Rationale |
|----|-------|----------|-----------|
| dec-001 | Connection pooling | bb8 + bb8-redis (NOT deadpool) | Already in Cargo.toml, working |
| dec-002 | Redis version | 8.4.0 via upstream merge | Native vector sets, latest security |
| dec-003 | Graceful shutdown | redis-cli SHUTDOWN + Kill fallback | Upstream improvement, prevents PID residue |
| dec-004 | Port assignment | 6380 (Windows/agent), 6379 (WSL/dev) | Avoid conflicts, both run simultaneously |
| dec-005 | Logging | ILogger + EventLog (our addition) | Windows Service best practice |
| dec-006 | Python commands | Always `uv run python` | Global CLAUDE.md mandate |

## Key Dependency Versions (agent-sync)

| Crate | Version | Note |
|-------|---------|------|
| redis | 0.25 | NOT 0.27 |
| thiserror | 1.0 | NOT 2 |
| tokio | 1.35 | Full features |
| bb8 | 0.8 | Connection pooling |
| bb8-redis | 0.13 | Redis adapter for bb8 |
| testcontainers | 0.15 | Dev dependency |
| rstest | 0.18 | Dev dependency |
| mockall | 0.12 | Dev dependency |

## Agent Work Registry

| Agent | Task | Files | Status |
|-------|------|-------|--------|
| search-specialist | Research Redis Windows updates | (research only) | Complete |
| Explore | Audit untracked file contents | (research only) | Complete |
| (direct) | CLAUDE.md audit and fixes | CLAUDE.md, .claude/CLAUDE.md | Complete |
| (direct) | Commit clustering (7 local + 1 merge) | 27 files | Complete |
| (direct) | Upstream merge + conflict resolution | Program.cs | Complete |
| (direct) | Build verification | (dotnet build) | Complete, 0 errors |

## Upstream Status

- **Origin:** david-t-martel/redis-windows (pushed, up to date)
- **Upstream:** redis-windows/redis-windows (merged at b560097)
- **No open PRs or extra branches**
- **Redis latest upstream tag:** 8.4.0

## Research Findings (Redis Ecosystem)

| Topic | Finding |
|-------|---------|
| Redis Windows latest | 8.4.0 (merged) |
| RedisSearch on Windows | Not available natively; use Docker/WSL |
| Native vector sets | Available in Redis 8.0+, works on Windows |
| WSL-Windows sync | Use REPLICAOF 127.0.0.1 6380 in WSL config |
| Memurai (commercial) | Redis 8 support expected Q3 2025 |

## Recommended Next Actions

1. **Test the merged Program.cs** - Run the Windows Service to verify graceful shutdown works
2. **Add REPLICAOF to WSL config** - Enable Windows-WSL Redis synchronization
3. **Build agent-sync** - `cd agent-sync && cargo build` to verify compilation
4. **Run agent-sync tests** - `cargo test` with a local Redis instance
5. **Explore Redis 8.4.0 vector sets** - Native vector support without RedisSearch module

## Recommended Next Agents

| Agent | Reason |
|-------|--------|
| rust-pro | Build and test agent-sync crate |
| csharp-pro | Review merged Program.cs for .NET best practices |
| test-automator | Create integration tests for Windows Service + agent-sync |
| security-auditor | Audit redis-agent.conf and network exposure |
