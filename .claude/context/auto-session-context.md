# Auto Session Context - redis-windows
Generated: 2026-02-06T08:00Z

## Project State
- Branch: `main` @ `0086b7d` + uncommitted agent-sync overhaul
- Upstream: merged at `b560097` (Redis 8.4.0)
- Major uncommitted changes: agent-sync Rust library rewrite, documentation overhaul

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
