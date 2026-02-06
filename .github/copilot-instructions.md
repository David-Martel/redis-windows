# Redis for Windows - AI Coding Agent Instructions

## Project Overview

This project provides a **production-ready Redis 8.4.0 server for Windows** with:

- **C# Windows Service Wrapper** (`Program.cs`, `RedisService.csproj`) - .NET 8 service managing Redis lifecycle
- **agent-sync Rust Library** (`agent-sync/`) - High-performance distributed coordination for LLM agents
- **GitHub Actions Build Pipeline** (`.github/workflows/`) - Automated compilation from source using MSYS2/Cygwin
- **PowerShell Build System** (`build.ps1`, `install-redis-service.ps1`) - Setup automation
- **Agent-Optimized Configuration** (`redis-agent.conf`) - Tuned for LLM agent workloads

### Architecture

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   LLM Agents    │───▶│   agent-sync    │───▶│  Redis Server   │
│ (Claude, GPT,   │    │   (Rust lib)    │    │  Windows Svc    │
│  local models)  │    │                 │    │  Port 6380      │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

The service wrapper converts Windows paths to Unix-style for MSYS2/Cygwin compatibility:

```csharp
var diskSymbol = configFilePath[..configFilePath.IndexOf(":")];
var fileConf = configFilePath.Replace(diskSymbol + ":", "/cygdrive/" + diskSymbol).Replace("\\", "/");
```

## Key Components

### .NET Windows Service (Program.cs)
- **BackgroundService pattern** managing redis-server.exe process
- Redirects stdout/stderr to Windows Event Log
- Graceful shutdown via redis-cli SHUTDOWN + process termination
- MSYS2/Cygwin path translation for Redis executable

### agent-sync Rust Library (agent-sync/)
- **TaskQueue**: Distributed task scheduling with priorities and dependencies
- **SharedContext**: Type-safe key-value storage with TTL
- **NotificationBus**: Pub/sub messaging with topic patterns
- **AgentRegistry**: Agent liveness tracking with heartbeats
- Uses redis 0.32, tokio 1.35, bb8 0.9 for connection pooling

### Build System
```powershell
.\build.ps1 -BuildOnly               # Build .NET service
.\install-redis-service.ps1           # Install Windows service
.\manage-dual-redis.ps1 -Action start # Manage dual Redis instances
.\configure-agent-redis.ps1 -Target windows # Switch Redis target
```

### Redis Configuration
- **Port 6380**: Agent-optimized (redis-agent.conf)
- **Port 6379**: WSL development
- **Memory**: 2GB limit with LRU eviction
- **Persistence**: RDB + AOF for durability

## Development Patterns

### Rust Code (agent-sync)
```bash
cd agent-sync
cargo build --release
cargo test
cargo clippy -- -D warnings
cargo fmt --check
```

### .NET Service
```powershell
dotnet build RedisService.csproj -c Release
dotnet publish RedisService.csproj -c Release -r win-x64 --self-contained -o publish
```

### Testing
```powershell
redis-cli.exe -p 6380 ping
Get-Service Redis
Get-EventLog -LogName Application -Source "Redis*" -Newest 5
```

## Code Conventions

- **Logging**: Use structured logging with `ILogger<T>` (.NET) or `tracing` (Rust)
- **Error handling**: Custom error types with `thiserror`, no `.unwrap()` in production
- **Paths**: Use `PathBuf` + `dunce` for Windows UNC compatibility
- **Async**: `tokio::select!` for concurrent operations, always set timeouts
- **Python**: Always use `uv run python`, never bare `python` or `pip`

## File Structure

```
redis-windows/
├── Program.cs                 # .NET 8 Windows service
├── RedisService.csproj        # .NET project
├── redis-agent.conf           # Agent-optimized Redis config
├── build.ps1                  # Main build automation
├── install-redis-service.ps1  # Service installation
├── manage-dual-redis.ps1      # Dual instance management
├── configure-agent-redis.ps1  # Instance switching
├── agent-sync/                # Rust coordination library
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs
│   │   ├── client.rs
│   │   ├── coordinator/       # TaskQueue, SharedContext, NotificationBus
│   │   ├── connection/        # Connection pooling (bb8)
│   │   └── platform/          # Windows/WSL abstractions
│   ├── examples/
│   └── benches/
└── .github/workflows/         # CI/CD pipelines
```
