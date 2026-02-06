# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Redis 8.4.0 for Windows wrapped in a .NET 8 Windows Service, with `agent-sync` - a high-performance Rust library for distributed LLM agent coordination via Redis.

## Architecture

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   LLM Agents    │───▶│   agent-sync    │───▶│  Redis Server   │
│ (Claude, GPT,   │    │   (Rust lib)    │    │  Windows Svc    │
│  local models)  │    │                 │    │  Port 6380      │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

**Core Components:**
- `.NET 8 Windows Service` (Program.cs) - Wraps redis-server.exe with Windows integration
- `agent-sync` (Rust) - Distributed coordination: task queues, shared context, pub/sub, agent registry
- `Redis Configurations` - Agent-optimized settings (redis-agent.conf, port 6380)
- `PowerShell Build System` - Automated setup and deployment
- `Dual Redis Support` - WSL (dev, port 6379) and Windows (prod, port 6380)

## Common Development Commands

### Build and Setup
```powershell
# Interactive setup (recommended first time)
.\build.ps1

# Build .NET service only
dotnet build RedisService.csproj -c Release
dotnet publish RedisService.csproj -c Release -r win-x64 --self-contained -o publish

# Download Redis binaries
.\build.ps1 -DownloadOnly

# Build agent-sync Rust library
cd agent-sync
cargo build --release
cargo test
cargo clippy -- -D warnings
```

### Running Redis
```powershell
# Start Redis directly (development)
redis-server.exe redis-agent.conf

# Start as Windows Service (production, requires admin)
.\install-redis-service.ps1
net start Redis

# Switch between WSL/Windows Redis
.\configure-agent-redis.ps1 -Target wsl      # Port 6379 (dev)
.\configure-agent-redis.ps1 -Target windows  # Port 6380 (prod)

# Manage dual Redis instances
.\manage-dual-redis.ps1 -Action start -Target both
.\manage-dual-redis.ps1 -Action status
```

### Testing
```powershell
# Test Redis connection
redis-cli.exe -p 6380 ping

# Check service status
Get-Service Redis
Get-EventLog -LogName Application -Source "Redis*" -Newest 5

# Performance monitoring
redis-cli.exe --latency-history -p 6380
redis-cli.exe info memory

# Run agent-sync tests
cd agent-sync && cargo test
```

## High-Level Architecture

### Service Layer (Program.cs)
- **BackgroundService Pattern**: Manages Redis process lifecycle
- **Windows Service Integration**: Runs as system service with event logging
- **Path Translation**: MSYS2/Cygwin compatibility for Redis executable
- **Graceful Shutdown**: redis-cli SHUTDOWN + process termination with timeout

### Configuration Strategy
```
redis-agent.conf     - Agent-optimized (2GB memory, port 6380, LRU eviction)
redis-windows.conf   - Standard Windows config (256MB, basic settings)
redis.hybrid.conf    - Comprehensive reference configuration
redis.conf          - Minimal default configuration
```

### Build System
- **build.ps1**: Main script with GitHub integration, interactive menu
- **install-redis-service.ps1**: Professional service installation with ACLs
- **configure-agent-redis.ps1**: Switch between Redis instances
- **manage-dual-redis.ps1**: Dual instance orchestration

## Agent-Sync Rust Library

The `agent-sync` crate (`agent-sync/`) is the primary agent integration layer.

### Core Modules
- **TaskQueue** (`src/coordinator/task_queue.rs`): Distributed task scheduling with priorities and dependencies
- **SharedContext** (`src/coordinator/shared_context.rs`): Type-safe key-value storage with TTL
- **NotificationBus** (`src/coordinator/notification_bus.rs`): Pub/sub messaging with topic patterns
- **AgentRegistry** (`src/coordinator/agent_registry.rs`): Agent liveness, heartbeats, capability discovery
- **ConnectionPool** (`src/connection/mod.rs`): bb8 connection pooling
- **Platform** (`src/platform/mod.rs`): Windows/WSL/Linux abstractions

### Key Dependencies
```toml
redis = "0.32"         # Redis client with async support
tokio = "1.35"         # Async runtime
bb8 = "0.9"            # Connection pooling
bb8-redis = "0.24"     # Redis adapter for bb8
thiserror = "2.0"      # Error handling
serde = "1.0"          # Serialization
tracing = "0.1"        # Structured logging
```

### Performance Targets
- Task submission: < 1ms
- Context retrieval: < 500us
- Notification broadcast: < 100us
- Throughput: 100K+ tasks/second

## Key Implementation Details

### Windows Service (Program.cs)
- Converts Windows paths to Unix-style for MSYS2 Redis executable
- Redirects stdout/stderr to Windows Event Log
- Implements async graceful shutdown (TryGracefulShutdownAsync + WaitForExitAsync)
- Monitors Redis health and auto-recovers on failure

### Dual Redis Strategy
- **Development**: WSL Redis on port 6379 (faster startup, Linux-native)
- **Production**: Windows Service on port 6380 (integrated monitoring, auto-start)
- Both run simultaneously without conflicts

## Project Dependencies

### Required
- **.NET SDK 8.0** - Windows service (Hosting.WindowsServices 8.0.1)
- **Rust 1.75+** - agent-sync library
- **Redis binaries** - Auto-downloaded from GitHub releases
- **PowerShell 5.1+** - Build scripts

### Optional
- **GitHub CLI** - Release management
- **Administrator privileges** - Service installation

## Service Management

### Monitoring
- Windows Event Log: `Application` source "Redis Service"
- Redis metrics: `redis-cli.exe -p 6380 info`
- Slow query log: 10ms threshold

### Troubleshooting
```powershell
.\manage-dual-redis.ps1 -Action status
Get-EventLog -LogName Application -Source "Redis*" -Newest 20
redis-cli.exe -p 6380 ping
Test-NetConnection localhost -Port 6380
```

## Notes

- Service name: "Redis Server"
- Config path specified with `-c` parameter
- Agent config (`redis-agent.conf`) is the primary production config
- Python usage: always `uv run python`, never bare `python`/`pip`
