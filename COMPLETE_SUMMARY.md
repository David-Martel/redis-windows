# Redis for Windows - Project Summary

## What's Included

### Production Components
- **Redis 8.4.0** - Native Windows port (upstream merged)
- **RedisService.exe** - .NET 8 Windows service wrapper (Program.cs)
- **redis-agent.conf** - Agent-optimized configuration (2GB, LRU, port 6380)
- **agent-sync** - Rust library for distributed agent coordination

### Build & Deployment
- **build.ps1** - Interactive PowerShell build automation
- **install-redis-service.ps1** - Windows service installation with ACLs
- **manage-dual-redis.ps1** - Dual instance orchestration (WSL + Windows)
- **configure-agent-redis.ps1** - Switch between Redis instances
- **GitHub Actions** - CI/CD pipeline for Redis binary compilation

### Agent Coordination (agent-sync Rust Library)
- **TaskQueue** - Distributed task scheduling with priorities
- **SharedContext** - Type-safe key-value storage with TTL
- **NotificationBus** - Pub/sub messaging with topic patterns
- **AgentRegistry** - Agent liveness and capability discovery

## Quick Start

```powershell
# Build .NET service
dotnet build RedisService.csproj -c Release

# Install and start
.\install-redis-service.ps1
net start Redis

# Verify
redis-cli.exe -p 6380 ping

# Build agent-sync
cd agent-sync && cargo build --release
```

## Directory Structure

```
redis-windows/
├── Program.cs                 # .NET 8 Windows service
├── RedisService.csproj        # .NET project file
├── redis-agent.conf           # Agent-optimized Redis config
├── build.ps1                  # Main build automation
├── install-redis-service.ps1  # Service installer
├── manage-dual-redis.ps1      # Dual instance management
├── agent-sync/                # Rust coordination library
│   ├── Cargo.toml
│   ├── src/                   # Library source (23 files)
│   ├── examples/              # Usage examples
│   └── benches/               # Performance benchmarks
├── .github/workflows/         # CI/CD pipelines
└── publish/                   # Built executables
```

## Documentation

- [README.md](README.md) - Project overview and quick start
- [SETUP_GUIDE.md](SETUP_GUIDE.md) - Detailed installation guide
- [QUICKSTART.md](QUICKSTART.md) - 5-minute setup
- [AGENT_INTEGRATION_GUIDE.md](AGENT_INTEGRATION_GUIDE.md) - Agent developer guide
- [agent-sync/README.md](agent-sync/README.md) - Rust library documentation
- [DUAL_REDIS_GUIDE.md](DUAL_REDIS_GUIDE.md) - Dual instance configuration
