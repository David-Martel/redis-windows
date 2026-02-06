# Auto Session Context - redis-windows
Generated: 2026-02-06T20:00Z

## Project State
- Branch: `main` @ `66be1eb` + uncommitted clippy fixes
- Upstream: merged at `b560097` (Redis 8.4.0)
- Last 6 commits pushed to origin/main (docs, agent-sync overhaul, scripts)
- Pending: 13 modified files (clippy fix pass) + 1 new context file

## Architecture (Current)
```
LLM Agents → agent-sync (Rust) → Redis 8.4.0 (Windows Service, port 6380)
```
- Python agent-memory-server: **DEPRECATED** (replaced by agent-sync)

## Components
| Component | Status | Key File |
|-----------|--------|----------|
| .NET 8 Windows Service | Production | Program.cs |
| Redis 8.4.0 configs | Production | redis-agent.conf (port 6380) |
| PowerShell scripts (6) | Production | build.ps1, manage-dual-redis.ps1 |
| agent-sync Rust lib | Development (23 src files) | agent-sync/Cargo.toml |
| CI/CD | Production | .github/workflows/ |

## Build Status (ALL CLEAN)
- `cargo clippy --all-targets -- -D warnings` → 0 errors
- `cargo check --features service` → 0 errors
- `cargo build --all-targets` → 0 errors, 0 warnings
- `dotnet build RedisService.csproj -c Release` → 0 errors, 0 warnings

## Key Dependencies (agent-sync)
- redis: 0.32 (with tokio-comp, connection-manager, aio)
- tokio: 1.35
- bb8: 0.9, bb8-redis: 0.24
- thiserror: 2.0
- serde: 1.0, serde_json: 1.0

## Key Facts
- Redis: 8.4.0, ports 6380 (agent) / 6379 (WSL dev)
- .NET: 8.0, Hosting.WindowsServices 8.0.1
- Python: always `uv run python`
- Service name: "Redis Server"
