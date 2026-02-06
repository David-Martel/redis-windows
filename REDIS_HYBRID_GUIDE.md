# Redis Hybrid WSL-Windows Configuration Guide

## Overview

This guide explains how to configure and manage Redis in a hybrid WSL-Windows environment for optimal agent-sync performance.

## Current Setup

- **WSL Redis**: Full-featured Linux Redis (version 7.0.15+) on port 6379
- **Windows Redis**: Custom Windows service on port 6380
- **agent-sync**: Rust coordination library connecting to either instance

## Key Benefits of Hybrid Configuration

### WSL Redis Advantages
- Full Redis feature set
- No memory limitations
- Standard Linux Redis behavior
- Better ecosystem support

### Windows Redis Advantages
- Native Windows service integration
- Windows Event Log integration
- Easier Windows deployment
- Agent-optimized configuration

## Configuration Files

### 1. `redis.hybrid.conf`
Optimized Redis configuration with agent-specific settings:
- Memory limit: 2GB
- LRU eviction policy
- Active defragmentation enabled
- Agent-optimized persistence settings
- Enhanced keyspace notifications

### 2. `redis-hybrid.ps1`
PowerShell management script for hybrid configuration:
```powershell
# Check status
.\redis-hybrid.ps1 status

# Switch between Redis instances
.\redis-hybrid.ps1 switch-to-wsl
.\redis-hybrid.ps1 switch-to-windows

# Optimize configurations
.\redis-hybrid.ps1 wsl
```

## Setup Instructions

### Option 1: Use WSL Redis (Recommended for Development)

1. **Apply Optimizations**:
   ```powershell
   cd T:\projects\redis-windows
   .\redis-hybrid.ps1 wsl
   ```

2. **Start WSL Redis**:
   ```powershell
   .\redis-hybrid.ps1 switch-to-wsl
   ```

3. **Verify Connection**:
   ```powershell
   .\redis-hybrid.ps1 status
   ```

### Option 2: Use Windows Redis (Production)

1. **Stop WSL Redis**:
   ```powershell
   .\redis-hybrid.ps1 switch-to-windows
   ```

2. **Configure Windows Redis**:
   ```powershell
   copy redis.hybrid.conf redis.conf
   .\install-redis-service.ps1
   ```

### Option 3: Dual Configuration (Advanced)

Run both Redis instances on different ports:

1. **WSL Redis**: Port 6379 (development)
2. **Windows Redis**: Port 6380 (agent workloads)

```bash
# agent-sync connects to either
REDIS_URL=redis://localhost:6379  # WSL dev
REDIS_URL=redis://localhost:6380  # Windows agent
```

## Agent-Specific Optimizations

### Memory Management
```conf
maxmemory 2gb
maxmemory-policy allkeys-lru
lazyfree-lazy-eviction yes
lazyfree-lazy-expire yes
lazyfree-lazy-server-del yes
```

### Vector Storage Optimization
```conf
activedefrag yes
active-defrag-ignore-bytes 100mb
active-defrag-threshold-lower 10
active-defrag-threshold-upper 100
```

### Session Persistence
```conf
save 900 1 300 10 60 1000
appendonly yes
appendfsync everysec
```

### Event Notifications
```conf
notify-keyspace-events "Ex"
```

### Connection Optimization
```conf
timeout 300
io-threads 4
io-threads-do-reads yes
maxclients 1000
```

## Testing & Monitoring

### Connection Test
```powershell
redis-cli -p 6379 ping   # WSL
redis-cli -p 6380 ping   # Windows
```

### Performance Monitoring
```bash
# WSL Redis monitoring
wsl -e redis-cli info memory
wsl -e redis-cli info stats
wsl -e redis-cli slowlog get 10

# Windows Redis monitoring
redis-cli -p 6380 info memory
redis-cli -p 6380 info stats
```

## Troubleshooting

### Port Conflicts
```powershell
.\redis-hybrid.ps1 status  # Check what's running
netstat -an | findstr ":6379"
netstat -an | findstr ":6380"
```

### WSL Redis Won't Start
```bash
wsl -e sudo systemctl status redis-server
wsl -e redis-server --test-config -c /etc/redis/redis.conf
wsl -e sudo journalctl -u redis-server -f
```

### agent-sync Connection Failed
1. Check Redis is running: `.\redis-hybrid.ps1 status`
2. Verify REDIS_URL environment variable
3. Test connection: `redis-cli -p <port> ping`

## Best Practices

### Development Workflow
1. Use WSL Redis for development (full features, no limits)
2. Apply agent optimizations with `redis-hybrid.ps1 wsl`
3. Monitor performance with Redis CLI commands
4. Run agent-sync tests against both instances

### Production Deployment
1. Switch to Windows Redis (`switch-to-windows`)
2. Use Windows service management features
3. Configure Windows Event Log monitoring
4. Implement backup strategies for agent data

## Integration with agent-sync

The hybrid configuration ensures optimal performance for:

- **Task Distribution**: Fast task queue operations across agents
- **Shared Context**: Efficient key-value storage with TTL
- **Pub/Sub Notifications**: Low-latency inter-agent messaging
- **Agent Registry**: Liveness tracking and capability discovery
- **Persistence**: Durable storage for agent state
- **Concurrency**: Multiple agent sessions simultaneously
