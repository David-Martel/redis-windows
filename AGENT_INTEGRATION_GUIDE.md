# LLM Agent Integration Guide

## Overview

This Redis setup is optimized for LLM agent workloads with the `agent-sync` Rust library providing high-performance coordination primitives on top of Redis.

## Architecture

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   LLM Agents    │───▶│   agent-sync    │───▶│  Redis Server   │
│ (Claude, GPT,   │    │   (Rust lib)    │    │  Port 6380      │
│  local models)  │    │                 │    │  Windows Svc    │
└─────────────────┘    └─────────────────┘    └─────────────────┘
                        Task queues
                        Shared context
                        Pub/sub notifications
                        Agent registry
```

## Quick Setup

### 1. Install Redis Service

```powershell
# Build and install Redis service
.\build.ps1 -BuildOnly
.\install-redis-service.ps1

# Start the service
net start Redis
```

### 2. Build agent-sync

```bash
cd agent-sync
cargo build --release
```

### 3. Connect Your Agents

```rust
use agent_sync::{AgentSync, Config};

let config = Config::builder()
    .redis_url("redis://localhost:6380")
    .build()?;
let client = AgentSync::new(config).await?;
```

## Agent Configuration

### Connection Settings

| Endpoint | URL | Purpose |
|----------|-----|---------|
| Redis (Agent) | `redis://localhost:6380` | Production agent workloads |
| Redis (Dev) | `redis://localhost:6379` | Development via WSL |

### Environment Variables

```bash
REDIS_URL=redis://localhost:6380
REDIS_PASSWORD=your_password_if_set
```

## Core Coordination Features

### Task Queue

Distributed task scheduling with priorities and dependencies:

```rust
// Submit a task
let task_id = client.task_queue().submit("process_data", payload).await?;

// Claim and process tasks
let task = client.task_queue().claim("worker-1").await?;
client.task_queue().complete(task.id, result).await?;
```

### Shared Context

Type-safe key-value storage for agent state:

```rust
// Store shared state with TTL
client.shared_context().set_with_ttl("agent:state", value, Duration::from_secs(300)).await?;

// Retrieve state
let state: AgentState = client.shared_context().get("agent:state").await?;
```

### Notification Bus

Pub/sub messaging between agents:

```rust
// Subscribe to topic patterns
let mut stream = client.notification_bus().subscribe("tasks.*").await?;

// Publish notifications
client.notification_bus().publish("tasks.completed", message).await?;
```

### Agent Registry

Track agent liveness and capabilities:

```rust
// Register an agent
client.registry().register(agent_info).await?;

// Heartbeat
client.registry().heartbeat(agent_id).await?;

// Discover agents
let agents = client.registry().list_active().await?;
```

## Redis Configuration for Agents

The `redis-agent.conf` is pre-tuned for agent workloads:

```conf
# Memory: 2GB with LRU eviction
maxmemory 2gb
maxmemory-policy allkeys-lru

# Non-blocking operations
lazyfree-lazy-eviction yes

# Memory optimization
activedefrag yes

# Large agent payloads
proto-max-bulk-len 512mb

# Keyspace event notifications
notify-keyspace-events "Ex"

# Durability
appendonly yes
appendfsync everysec
```

## Performance Targets

| Operation | Target Latency | Target Throughput |
|-----------|---------------|-------------------|
| Task submission | < 1ms | 100K tasks/sec |
| Context retrieval | < 500us | 1M ops/sec |
| Notification broadcast | < 100us | 50K msgs/sec |
| Agent registration | < 5ms | 10K agents |

## Production Deployment

### Security

```conf
# In redis-agent.conf
requirepass your_very_secure_password
```

```powershell
# Firewall
New-NetFirewallRule -DisplayName "Redis Agent" -Direction Inbound -Protocol TCP -LocalPort 6380 -RemoteAddress 127.0.0.1
```

### Monitoring

```powershell
redis-cli.exe -p 6380 info memory
redis-cli.exe -p 6380 info clients
redis-cli.exe -p 6380 slowlog get 10
Get-EventLog -LogName Application -Source "Redis*" -Newest 20
```

## Troubleshooting

### Connection Issues

```powershell
# Check service
Get-Service Redis

# Test connectivity
redis-cli.exe -p 6380 ping
Test-NetConnection localhost -Port 6380
```

### Performance Issues

```powershell
redis-cli.exe -p 6380 slowlog get 10
redis-cli.exe -p 6380 info memory
```

## Next Steps

1. **Scale**: Consider Redis Cluster for multiple agents
2. **Monitor**: Prometheus/Grafana integration
3. **Backup**: Automated Redis backup strategy
4. **Load Test**: Benchmark with realistic agent workloads

For full API documentation, see [agent-sync/README.md](agent-sync/README.md).
