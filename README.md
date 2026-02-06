# Redis for Windows - LLM Agent Coordination

A production-ready Redis 8.4.0 server for Windows, wrapped in a .NET 8 Windows Service, with a high-performance Rust coordination library (`agent-sync`) for LLM agents.

## Architecture

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   LLM Agents    │───▶│   agent-sync    │───▶│  Redis Server   │
│ (Claude, GPT,   │    │   (Rust lib)    │    │  (Windows Svc)  │
│  local models)  │    │                 │    │  Port 6380      │
└─────────────────┘    └─────────────────┘    └─────────────────┘
                        Distributed tasks
                        Shared context
                        Pub/sub notifications
                        Agent registry
```

## Components

| Component | Technology | Status |
|-----------|-----------|--------|
| Redis Server | Redis 8.4.0 (upstream merged) | Production |
| Windows Service | .NET 8 BackgroundService | Production |
| agent-sync | Rust async library | Development |
| Build System | PowerShell + GitHub Actions | Production |

## Quick Start

```powershell
# Clone and build
git clone https://github.com/david-t-martel/redis-windows.git
cd redis-windows

# Build .NET service
dotnet build RedisService.csproj -c Release

# Install as Windows service (admin required)
.\install-redis-service.ps1

# Start Redis
net start Redis

# Verify
redis-cli.exe -p 6380 ping   # PONG
```

## Agent Integration (Rust)

The `agent-sync` crate provides distributed coordination primitives for AI agents:

```toml
[dependencies]
agent-sync = "0.1.0"
```

```rust
use agent_sync::{AgentSync, Config};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Config::builder()
        .redis_url("redis://localhost:6380")
        .build()?;
    let client = AgentSync::new(config).await?;

    // Submit a task
    let task_id = client.task_queue().submit("process_data", payload).await?;

    // Share context between agents
    client.shared_context().set("key", value).await?;

    // Subscribe to notifications
    client.notification_bus().subscribe("topic.*").await?;

    Ok(())
}
```

### Core Capabilities

- **TaskQueue** - Priority-based distributed task scheduling with dependencies
- **SharedContext** - Type-safe key-value storage with TTL and semantic search
- **NotificationBus** - Pub/sub messaging with topic patterns and direct messaging
- **AgentRegistry** - Agent liveness tracking with heartbeats and capabilities

### Building agent-sync

```bash
cd agent-sync
cargo build --release
cargo test
cargo clippy -- -D warnings
```

See [`agent-sync/README.md`](agent-sync/README.md) for full API documentation.

## Redis Configuration

### Dual Instance Strategy

| Instance | Port | Purpose | Config |
|----------|------|---------|--------|
| Windows Service | 6380 | Production agent workloads | `redis-agent.conf` |
| WSL Redis | 6379 | Development / testing | System default |

```powershell
# Manage both instances
.\manage-dual-redis.ps1 -Action status
.\manage-dual-redis.ps1 -Action start -Target both

# Switch agent target
.\configure-agent-redis.ps1 -Target windows   # Port 6380
.\configure-agent-redis.ps1 -Target wsl        # Port 6379
```

### Agent-Optimized Settings (`redis-agent.conf`)

```conf
maxmemory 2gb
maxmemory-policy allkeys-lru
lazyfree-lazy-eviction yes
activedefrag yes
proto-max-bulk-len 512mb
notify-keyspace-events "Ex"
appendonly yes
```

## Build Options

```powershell
# Interactive menu
.\build.ps1

# Direct commands
.\build.ps1 -BuildOnly               # Build .NET service
.\build.ps1 -CheckLatestRelease      # Check GitHub releases
.\build.ps1 -DownloadOnly            # Download Redis binaries
.\build.ps1 -GitHubWorkflow          # Trigger CI build
```

## Monitoring

```powershell
redis-cli.exe -p 6380 info memory
redis-cli.exe -p 6380 info clients
redis-cli.exe --latency-history -p 6380
Get-EventLog -LogName Application -Source "Redis*" -Newest 10
```

## Documentation

| Guide | Description |
|-------|-------------|
| [Setup Guide](SETUP_GUIDE.md) | Detailed installation and configuration |
| [Quick Start](QUICKSTART.md) | Get running in 5 minutes |
| [Dual Redis Guide](DUAL_REDIS_GUIDE.md) | WSL + Windows dual instance setup |
| [agent-sync README](agent-sync/README.md) | Rust library API and architecture |
| [agent-sync Testing](RUST_TESTING_STRATEGY.md) | Testing strategy for agent-sync |

## Dependencies

### Required
- **.NET SDK 8.0** - Windows service wrapper
- **Redis binaries** - Auto-downloaded from GitHub releases
- **PowerShell 5.1+** - Build scripts

### Optional
- **Rust 1.75+** - For building agent-sync
- **GitHub CLI** - Release management
- **Administrator privileges** - Service installation

## Contributing

Contributions welcome, especially for:
- Agent framework integrations
- Performance optimizations
- Cross-platform testing
- Documentation improvements

## License

This project is licensed under the same terms as Redis - see [LICENSE](LICENSE) for details.

## Disclaimer

This GitHub organization and its repositories are not affiliated with, endorsed by, or sponsored by Redis Ltd. The license provided here applies only to the content of this repository, and does not apply to the official [Redis](https://github.com/redis/redis) project or its software.

We suggest that you use it for local development and follow Redis official guidance to deploy it on Linux for production environment. This project doesn't bear any responsibility for any losses caused by using it and is only for learning and exchange purposes.
