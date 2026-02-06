# Dual Redis Configuration Guide

# ==============================

## Overview

This guide explains how to run both WSL Redis and Windows Redis simultaneously on different ports, allowing for independent development and production environments.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     Dual Redis Setup                       │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  🐧 WSL Redis (Development)     🪟 Windows Redis (Production) │
│  ├─ Port: 6379                 ├─ Port: 6380                │
│  ├─ Memory: Unlimited          ├─ Memory: 2GB limit         │
│  ├─ Config: /etc/redis/        ├─ Config: redis-agent.conf  │
│  ├─ Service: systemctl         ├─ Service: Windows Service  │
│  └─ Use: Development/Testing   └─ Use: Production/Staging   │
│                                                             │
│                    Agent Memory Server                      │
│                    ├─ Can connect to either                │
│                    ├─ WSL: redis://localhost:6379          │
│                    └─ Windows: redis://localhost:6380      │
└─────────────────────────────────────────────────────────────┘
```

## Port Configuration

### WSL Redis (Port 6379)

- **Purpose**: Development and testing
- **Configuration**: `/etc/redis/redis.conf`
- **Memory**: No artificial limits
- **Persistence**: Full RDB + AOF
- **Management**: `systemctl` commands

### Windows Redis (Port 6380)

- **Purpose**: Production and staging
- **Configuration**: `redis-agent.conf`
- **Memory**: 2GB limit (configurable)
- **Persistence**: Optimized RDB + AOF
- **Management**: Windows Service

## Quick Start

### 1. Check Current Status

```powershell
.\manage-dual-redis.ps1 -Action status
```

### 2. Optimize and Setup Both Redis Instances

```powershell
.\manage-dual-redis.ps1 -Action optimize
```

### 3. Start Both Redis Instances

```powershell
.\manage-dual-redis.ps1 -Action start
```

### 4. Test the Configuration

```powershell
.\manage-dual-redis.ps1 -Action test
```

### 5. Configure Agent Memory Server

```powershell
# For development (WSL Redis)
.\configure-agent-redis.ps1 -Target wsl

# For production (Windows Redis)
.\configure-agent-redis.ps1 -Target windows
```

## Management Commands

### Dual Redis Management Script

```powershell
# Start both instances
.\manage-dual-redis.ps1 -Action start

# Start only WSL Redis
.\manage-dual-redis.ps1 -Action start -Target wsl

# Start only Windows Redis
.\manage-dual-redis.ps1 -Action start -Target windows

# Stop both instances
.\manage-dual-redis.ps1 -Action stop

# Restart specific instance
.\manage-dual-redis.ps1 -Action restart -Target wsl

# Check status of both
.\manage-dual-redis.ps1 -Action status

# Test functionality
.\manage-dual-redis.ps1 -Action test

# Optimize configurations
.\manage-dual-redis.ps1 -Action optimize
```

### Agent Configuration Script

```powershell
# Switch to WSL Redis (development)
.\configure-agent-redis.ps1 -Target wsl

# Switch to Windows Redis (production)
.\configure-agent-redis.ps1 -Target windows

# Check current configuration
.\configure-agent-redis.ps1 -Target status
```

## Manual Management

### WSL Redis Commands

```bash
# Start WSL Redis
wsl -e sudo systemctl start redis-server

# Stop WSL Redis
wsl -e sudo systemctl stop redis-server

# Check WSL Redis status
wsl -e sudo systemctl status redis-server

# Connect to WSL Redis
redis-cli -p 6379

# View WSL Redis logs
wsl -e sudo journalctl -u redis-server -f
```

### Windows Redis Commands

```powershell
# Start Windows Redis Service
net start Redis

# Stop Windows Redis Service
net stop Redis

# Check Windows Redis Service status
Get-Service Redis

# Connect to Windows Redis
redis-cli -p 6380

# View Windows Redis logs
Get-EventLog -LogName Application -Source "Redis Service" -Newest 10
```

## Testing Connections

### Direct Redis Testing

```powershell
# Test WSL Redis
redis-cli -p 6379 ping

# Test Windows Redis
redis-cli -p 6380 ping

# Set test data
redis-cli -p 6379 set "test-key" "wsl-value"
redis-cli -p 6380 set "test-key" "windows-value"

# Retrieve test data
redis-cli -p 6379 get "test-key"  # Should return: wsl-value
redis-cli -p 6380 get "test-key"  # Should return: windows-value
```

### Agent Memory Server Testing

```powershell
# Switch to WSL Redis and test
.\configure-agent-redis.ps1 -Target wsl
cd C:\Users\david\agent-memory-server
.venv\Scripts\activate
python -c "import redis; r = redis.Redis(host='localhost', port=6379); print('WSL Redis:', r.ping())"

# Switch to Windows Redis and test
.\configure-agent-redis.ps1 -Target windows
python -c "import redis; r = redis.Redis(host='localhost', port=6380); print('Windows Redis:', r.ping())"
```

## Configuration Files

### WSL Redis Configuration

**Location**: `/etc/redis/redis.conf`

Key settings for agent workloads:

```ini
# Port configuration
port 6379

# Memory management
maxmemory 2gb
maxmemory-policy allkeys-lru

# Agent optimizations
lazyfree-lazy-eviction yes
activedefrag yes
notify-keyspace-events "Ex"

# Persistence
save 900 1 300 10 60 1000
appendonly yes
appendfsync everysec
```

### Windows Redis Configuration

**Location**: `T:\projects\redis-windows\redis-agent.conf`

Key settings:

```ini
# Port configuration
port 6380

# Memory management
maxmemory 2gb
maxmemory-policy allkeys-lru

# Agent optimizations
lazyfree-lazy-eviction yes
activedefrag yes
notify-keyspace-events "Ex"

# Windows-specific settings
supervised no
daemonize no
```

## Agent Memory Server Configuration

### Environment File Location

`C:\Users\david\agent-memory-server\.env`

### Development Configuration (WSL Redis)

```ini
REDIS_URL=redis://localhost:6379
PORT=8000
LONG_TERM_MEMORY=true
WINDOW_SIZE=12
GENERATION_MODEL=gpt-4o-mini
EMBEDDING_MODEL=text-embedding-3-small
DISABLE_AUTH=true
```

### Production Configuration (Windows Redis)

```ini
REDIS_URL=redis://localhost:6380
PORT=8000
LONG_TERM_MEMORY=true
WINDOW_SIZE=12
GENERATION_MODEL=gpt-4o-mini
EMBEDDING_MODEL=text-embedding-3-small
DISABLE_AUTH=true
```

## Use Cases

### Development Workflow

1. Start WSL Redis for development:

   ```powershell
   .\manage-dual-redis.ps1 -Action start -Target wsl
   ```

2. Configure agent for development:

   ```powershell
   .\configure-agent-redis.ps1 -Target wsl
   ```

3. Start agent memory server:

   ```powershell
   cd C:\Users\david\agent-memory-server
   .venv\Scripts\activate
   python -m agent_memory_server.cli api --port 8000
   ```

### Production Testing

1. Start Windows Redis for production testing:

   ```powershell
   .\manage-dual-redis.ps1 -Action start -Target windows
   ```

2. Configure agent for production:

   ```powershell
   .\configure-agent-redis.ps1 -Target windows
   ```

3. Start agent memory server:

   ```powershell
   cd C:\Users\david\agent-memory-server
   .venv\Scripts\activate
   python -m agent_memory_server.cli api --port 8000
   ```

### Simultaneous Testing

Both Redis instances can run simultaneously, allowing you to:

- Test data migration between environments
- Compare performance characteristics
- Run parallel development and staging environments

## Monitoring and Troubleshooting

### Port Conflicts

Check which processes are using the ports:

```powershell
netstat -an | findstr ":6379"
netstat -an | findstr ":6380"
```

### Memory Usage

```powershell
# WSL Redis memory
redis-cli -p 6379 info memory

# Windows Redis memory
redis-cli -p 6380 info memory
```

### Performance Monitoring

```powershell
# WSL Redis stats
redis-cli -p 6379 info stats

# Windows Redis stats
redis-cli -p 6380 info stats

# Slow query log
redis-cli -p 6379 slowlog get 10
redis-cli -p 6380 slowlog get 10
```

### Common Issues

#### Issue: Port already in use

**Solution**: Stop the conflicting service or change the port configuration.

#### Issue: WSL Redis won't start

**Solution**:

```bash
wsl -e sudo systemctl status redis-server
wsl -e sudo journalctl -u redis-server -n 20
```

#### Issue: Windows Redis Service fails to start

**Solution**:

```powershell
# Check if service is installed
Get-Service Redis -ErrorAction SilentlyContinue

# Reinstall if needed
.\manage-dual-redis.ps1 -Action optimize
```

#### Issue: Agent Memory Server connection fails

**Solution**:

1. Check Redis is running: `.\manage-dual-redis.ps1 -Action status`
2. Verify configuration: `.\configure-agent-redis.ps1 -Target status`
3. Test connection: `redis-cli -p <port> ping`

## Security Considerations

### Development Environment (WSL Redis)

- Runs on localhost only
- No password required
- Suitable for development

### Production Environment (Windows Redis)

- Consider adding password authentication
- Enable Windows Service security
- Monitor access logs

### Network Security

Both instances are configured to bind to `127.0.0.1` only, preventing external access.

## Backup and Migration

### Data Export/Import

```powershell
# Export from WSL Redis
redis-cli -p 6379 --rdb backup-wsl.rdb

# Export from Windows Redis  
redis-cli -p 6380 --rdb backup-windows.rdb

# Import data (stop Redis first)
# Copy backup file to Redis directory and restart
```

### Configuration Backup

```powershell
# Backup WSL config
wsl -e sudo cp /etc/redis/redis.conf /etc/redis/redis.conf.backup

# Backup Windows config
copy redis-agent.conf redis-agent.conf.backup
```

## Performance Comparison

| Feature | WSL Redis (6379) | Windows Redis (6380) |
|---------|------------------|----------------------|
| Memory Limit | Unlimited | 2GB |
| Startup Time | ~2-3 seconds | ~1-2 seconds |
| Native Performance | Linux-optimized | Windows-optimized |
| Service Integration | systemctl | Windows Service |
| Monitoring | journalctl | Event Viewer |
| Best Use | Development | Production |

This dual Redis setup provides maximum flexibility for development and production testing while maintaining clear separation between environments.
