# Auto Session Context - redis-windows
Generated: 2026-02-06

## Project State
- Branch: `main` (1 commit ahead of origin/main)
- Last commit: `003eb5f` - feat: Add agent-sync high-performance Rust library
- Active work: agent-sync Rust library, Redis Windows Service

## Components
| Component | Status | Location |
|-----------|--------|----------|
| .NET 8 Windows Service | Production | Program.cs, RedisService.csproj |
| Redis configs | Production | redis-agent.conf (primary), redis-windows.conf, redis.hybrid.conf |
| PowerShell build system | Production | build.ps1, build-modular.ps1, install-redis-service.ps1 |
| agent-sync Rust lib | Development | agent-sync/ (23 source files) |

## Key Facts
- Redis port: 6380 (Windows/agent), 6379 (WSL/dev)
- Service name: "Redis Server"
- .NET 8 SDK required, targets win-x64
- agent-sync uses: redis 0.25, thiserror 1.0, tokio 1.35, bb8 for pooling
- Agent memory server: Python via UV, ports 8000 (REST) + 8001 (MCP)

## Untracked Files (notable)
- agent-sync/ workflow configs (.github/, .cargo/)
- Several PowerShell management scripts
- Multiple analysis/guide markdown files

## Recent CLAUDE.md Improvements (this session)
- Fixed bare `python` command to `uv run python`
- Updated agent-sync section from aspirational "Vision" to factual
- Fixed all dependency versions in .claude/CLAUDE.md (redis, thiserror, pooling lib, dev-deps)
- Removed references to crates not in Cargo.toml (SmallVec, zeroize, deadpool-redis)
