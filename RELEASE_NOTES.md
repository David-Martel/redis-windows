# Redis Windows Release Notes

## v3.0 - Rust Agent Coordination

### Breaking Changes
- **Deprecated agent-memory-server (Python)** - Agent integration is now provided by the `agent-sync` Rust library
- **Default agent port is 6380** - All agent tools standardize on port 6380

### New Features

#### agent-sync Rust Library
- **Distributed Task Queue** - Priority-based task scheduling with dependencies
- **Shared Context** - Type-safe key-value storage with TTL and semantic search
- **Notification Bus** - Pub/sub messaging with topic patterns and direct messaging
- **Agent Registry** - Liveness tracking with heartbeats and capability discovery
- **Connection Pooling** - bb8-based async connection pool
- **Cross-Platform** - Native support for Windows, Linux, and WSL

#### Redis 8.4.0 (Upstream Merge)
- Merged upstream Redis 8.4.0 release
- Graceful shutdown via `redis-cli SHUTDOWN`
- Improved build pipeline

#### Enhanced Windows Service
- Async graceful shutdown (TryGracefulShutdownAsync + WaitForExitAsync)
- Structured logging with Windows Event Log + ILogger
- Process health monitoring with auto-recovery

### Build System
- Interactive PowerShell script (`build.ps1`) with menu options
- GitHub Actions CI/CD for Redis binary compilation
- Dual Redis instance management (WSL dev + Windows prod)

### Performance Targets
- Task submission: < 1ms
- Context retrieval: < 500us
- Notification broadcast: < 100us
- Throughput: 100K+ tasks/second

---

## v2.0 - LLM Agent Ready

### Features
- Agent-optimized Redis configuration (`redis-agent.conf`)
- 2GB memory limit with LRU eviction
- Windows Service wrapper with event logging
- Dual Redis support (WSL + Windows)
- PowerShell build automation

---

## v1.0 - Initial Release

### Features
- Basic Redis for Windows service wrapper
- .NET 8 BackgroundService pattern
- GitHub Actions build pipeline (MSYS2 + Cygwin)
- Configuration management
