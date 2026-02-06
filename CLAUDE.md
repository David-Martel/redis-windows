# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Redis for Windows with native LLM agent integration - a production-ready Redis server wrapped in a .NET 8 Windows Service, optimized for agent memory management and vector storage.

## Architecture

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   LLM Agents    │───▶│ Agent Memory    │───▶│ Redis Service   │
│ (Claude/GPT)    │    │ Server (Python) │    │ (This Project)  │
└─────────────────┘    └─────────────────┘    └─────────────────┘
                              │                       │
                    MCP/REST API            Port 6379/6380
```

**Core Components:**
- `.NET 8 Windows Service` (Program.cs) - Wraps redis-server.exe with Windows integration
- `Redis Configurations` - Agent-optimized settings (redis-agent.conf)
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
.\build-modular.ps1 -DownloadBinaries

# Setup for LLM agents (includes agent-memory-server)
.\build.ps1 -PrepareForAgents
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
# Test connection
redis-cli.exe -p 6380 ping

# Run comprehensive tests
.\build.ps1 -TestRedis

# Check service status
Get-Service Redis
Get-EventLog -LogName Application -Source "Redis*" -Newest 5

# Performance monitoring
redis-cli.exe --latency-history -p 6380
redis-cli.exe info memory
```

### Agent Memory Server Integration
```powershell
# Setup UV environment for Python agents
.\build.ps1 -SetupUvEnvironment

# Configure agent memory server
.\build.ps1 -ConfigureMemoryServer

# Test agent integration
curl http://localhost:8000/health
uv run python -m agent_memory_server.cli mcp --mode stdio
```

## High-Level Architecture

### Service Layer (Program.cs)
- **BackgroundService Pattern**: Manages Redis process lifecycle
- **Windows Service Integration**: Runs as system service with event logging
- **Path Translation**: MSYS2/Cygwin compatibility for Redis executable
- **Process Monitoring**: Automatic restart on failure

### Configuration Strategy
```
redis-agent.conf     - Agent-optimized (2GB memory, port 6380, LRU eviction)
redis-windows.conf   - Standard Windows config (256MB, basic settings)
redis.hybrid.conf    - Comprehensive reference configuration
redis.conf          - Minimal default configuration
```

**Agent Optimizations in redis-agent.conf:**
- `maxmemory 2gb` with `allkeys-lru` eviction
- `proto-max-bulk-len 512mb` for large agent data
- `activedefrag yes` for memory efficiency
- `notify-keyspace-events "Ex"` for monitoring
- Persistence: RDB snapshots + AOF for durability

### Build System Architecture
- **build.ps1**: Main script with GitHub integration, UV support, interactive menu
- **build-modular.ps1**: Simplified modular approach
- **install-redis-service.ps1**: Professional service installation with ACLs
- **configure-agent-redis.ps1**: Switch between Redis instances
- **manage-dual-redis.ps1**: Dual instance orchestration

### Agent Integration Points
1. **MCP Server** (Port 8001): Claude Desktop integration via stdio
2. **REST API** (Port 8000): HTTP endpoints for memory operations
3. **Direct Redis** (6379/6380): Low-level access for custom implementations

### Memory Architecture for Agents
- **Working Memory**: Session-scoped conversation storage
- **Long-term Memory**: Persistent knowledge across sessions
- **Vector Storage**: Embeddings with semantic search
- **Session Isolation**: Multi-agent support with isolated contexts

## Key Implementation Details

### Windows Service Implementation
The service wrapper (`Program.cs`) uses .NET's hosted service model:
- Converts Windows paths to Unix-style for MSYS2 Redis executable
- Redirects stdout/stderr to Windows Event Log
- Implements graceful shutdown with process cleanup
- Monitors Redis health and auto-recovers on failure

### Dual Redis Strategy
- **Development**: WSL Redis on port 6379 (faster startup, Linux-native)
- **Production**: Windows Service on port 6380 (integrated monitoring, auto-start)
- Both can run simultaneously without conflicts
- Configuration switching via `configure-agent-redis.ps1`

### Agent Memory Server Integration
- UV-managed Python environment at `C:\users\david\.venv`
- Environment variables in `C:\Users\david\agent-memory-server\.env`
- MCP configuration in Claude Desktop's `claude_desktop_config.json`
- Automatic setup via `.\build.ps1 -PrepareForAgents`

### Performance Optimizations
- Active defragmentation for memory efficiency
- Lazy freeing for non-blocking operations
- Optimized data structures for agent workloads
- Connection pooling with 10000 max clients
- Background persistence (RDB + AOF)

## Project Dependencies

### Required
- **.NET SDK 8.0** - For building Windows service
- **Redis binaries** - Auto-downloaded from GitHub releases
- **PowerShell 5.1+** - For build scripts

### Optional
- **UV Package Manager** - For Python agent integration
- **GitHub CLI** - For release management
- **Python 3.11+** - For agent-memory-server
- **Administrator privileges** - For service installation

## Service Management

### Installation
```powershell
# Install service (requires admin)
.\install-redis-service.ps1

# Custom installation
.\install-redis-service.ps1 -ServiceName "RedisAgent" -ConfigFile "redis-agent.conf"
```

### Monitoring
- Windows Event Log: `Application` log with source "Redis Service"
- Redis logs: Configured location in redis-agent.conf
- Performance metrics: `redis-cli.exe info`
- Slow query log: Configured at 10ms threshold

### Troubleshooting
```powershell
# Check both Redis instances
.\manage-dual-redis.ps1 -Action status

# View service logs
Get-EventLog -LogName Application -Source "Redis*" -Newest 20

# Test connectivity
redis-cli.exe -p 6380 ping
Test-NetConnection localhost -Port 6380

# Restart service
net stop Redis
net start Redis
```

## Configuration Files

### Key Settings for Agent Workloads
- **Memory**: 2GB limit with LRU eviction
- **Persistence**: Dual strategy (RDB + AOF)
- **Network**: Localhost only, ports 6379/6380
- **Buffers**: Optimized for large agent payloads
- **Monitoring**: Keyspace events and slow query logging

### Environment Variables
- `REDIS_URL`: Connection string for agent-memory-server
- `OPENAI_API_KEY`: Required for embedding generation
- `PORT`/`MCP_PORT`: API and MCP server ports

## Agent-Sync Rust Framework

The `agent-sync` crate (`agent-sync/`) provides high-performance distributed coordination primitives for AI agents using Redis as the backbone.

### Current Implementation
- **TaskQueue** (`src/coordinator/task_queue.rs`): Distributed task scheduling with priorities and dependencies
- **SharedContext** (`src/coordinator/shared_context.rs`): Type-safe key-value storage with TTL and semantic search
- **NotificationBus** (`src/coordinator/notification_bus.rs`): Pub/sub messaging with topic patterns and direct messaging
- **Testing infrastructure** (`src/testing/`): Mock Redis, fixtures, and helpers

### Integration
- Connects to the existing Redis service (port 6380) using agent-optimized config
- Provides async Rust APIs complementing the Python agent-memory-server
- Targets 100K+ tasks/second throughput
- Supports both Windows Service and WSL deployment

### Building agent-sync
```bash
cd agent-sync
cargo build --release
cargo test
cargo clippy -- -D warnings
```

## Notes

- Service runs as "Redis Server" by default
- Configuration file path can be specified with `-c` parameter
- Agent configuration (`redis-agent.conf`) is optimized for LLM workloads
- Both WSL and Windows Redis can run simultaneously on different ports
- The system is designed for persistent agent memory with semantic search capabilities
- The `agent-sync` Rust framework extends capabilities for high-performance agent coordination