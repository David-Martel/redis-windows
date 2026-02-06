# Redis Hybrid WSL-Windows Configuration Guide

## Overview

This guide explains how to configure and manage Redis in a hybrid WSL-Windows environment for optimal agent-memory-server performance.

## Current Setup

- **WSL Redis**: Full-featured Linux Redis (version 7.0.15+)
- **Windows Redis**: Your custom Windows service implementation
- **Agent Memory Server**: Python application connecting to Redis on localhost:6379

## Key Benefits of Hybrid Configuration

### WSL Redis Advantages
- ✅ Full Redis feature set
- ✅ No memory limitations
- ✅ Standard Linux Redis behavior
- ✅ Better ecosystem support

### Windows Redis Advantages  
- ✅ Native Windows service integration
- ✅ Windows Event Log integration
- ✅ Easier Windows deployment
- ✅ Your custom optimizations

## Configuration Files

### 1. `redis.hybrid.conf`
Optimized Redis configuration with agent-specific settings:
- Memory limit: 2GB
- LRU eviction policy
- Active defragmentation enabled
- Agent-optimized persistence settings
- Enhanced keyspace notifications

### 2. `optimize-wsl-redis.sh`
Bash script to apply optimizations to WSL Redis:
```bash
# Run from Windows PowerShell
wsl -e bash ./optimize-wsl-redis.sh
```

### 3. `redis-hybrid.ps1`
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
   # Copy optimized config
   copy redis.hybrid.conf redis.conf
   
   # Install/start service
   .\build.ps1 -InstallService
   ```

### Option 3: Dual Configuration (Advanced)

Run both Redis instances on different ports:

1. **WSL Redis**: Keep on port 6379
2. **Windows Redis**: Configure for port 6380

Edit Windows `redis.conf`:
```conf
port 6380
```

Update agent-memory-server `.env`:
```env
# For WSL Redis
REDIS_URL=redis://localhost:6379

# For Windows Redis  
REDIS_URL=redis://localhost:6380
```

## Agent-Specific Optimizations

The hybrid configuration includes these agent-optimized settings:

### Memory Management
```conf
# 2GB limit for agent operations
maxmemory 2gb
maxmemory-policy allkeys-lru

# Lazy freeing for large datasets
lazyfree-lazy-eviction yes
lazyfree-lazy-expire yes
lazyfree-lazy-server-del yes
```

### Vector Storage Optimization
```conf
# Active defragmentation for embedding vectors
activedefrag yes
active-defrag-ignore-bytes 100mb
active-defrag-threshold-lower 10
active-defrag-threshold-upper 100
```

### Session Persistence
```conf
# Frequent saves for agent sessions
save 900 1 300 10 60 1000

# AOF for session durability
appendonly yes
appendfsync everysec
```

### Event Notifications
```conf
# Enable expiration events for session cleanup
notify-keyspace-events "Ex"
```

### Connection Optimization
```conf
# Higher timeout for agent processing
timeout 300

# More I/O threads for concurrency
io-threads 4
io-threads-do-reads yes

# Increased client limits
maxclients 1000
```

## Testing & Monitoring

### Connection Test
```powershell
# Test from agent-memory-server directory
cd C:\Users\david\agent-memory-server
.\.venv\Scripts\python.exe -c "import redis; r = redis.Redis(); print('Connected:', r.ping())"
```

### Performance Monitoring
```bash
# WSL Redis monitoring
wsl -e redis-cli info memory
wsl -e redis-cli info stats
wsl -e redis-cli info clients

# Check slow queries
wsl -e redis-cli slowlog get 10
```

### Agent Memory Server Test
```powershell
# Start agent memory server
cd C:\Users\david\agent-memory-server
.\.venv\Scripts\activate
python -m agent_memory_server.cli api --port 8000

# Test MCP integration
python -m agent_memory_server.cli mcp --mode stdio
```

## Troubleshooting

### Port Conflicts
```
Error: "Address already in use"
```
**Solution**: Only one Redis can use port 6379
```powershell
.\redis-hybrid.ps1 status  # Check what's running
.\redis-hybrid.ps1 switch-to-wsl  # Switch to WSL
```

### WSL Redis Won't Start
```bash
# Check WSL Redis status
wsl -e sudo systemctl status redis-server

# Check configuration
wsl -e redis-server --test-config -c /etc/redis/redis.conf

# View logs
wsl -e sudo journalctl -u redis-server -f
```

### Agent Memory Server Connection Failed
```
ConnectionError: Error connecting to Redis
```
**Solutions**:
1. Check Redis is running: `.\redis-hybrid.ps1 status`
2. Verify port: Check `REDIS_URL` in `.env`
3. Test connection: `wsl -e redis-cli ping`

### Performance Issues
```bash
# Check memory usage
wsl -e redis-cli info memory

# Monitor slow operations
wsl -e redis-cli config set slowlog-log-slower-than 1000
wsl -e redis-cli slowlog get 5

# Check fragmentation
wsl -e redis-cli info memory | grep fragmentation
```

## Best Practices

### Development Workflow
1. Use WSL Redis for development (full features, no limits)
2. Apply agent optimizations with `redis-hybrid.ps1 wsl`
3. Monitor performance with Redis CLI commands
4. Test agent-memory-server integration regularly

### Production Deployment
1. Switch to Windows Redis for production (`switch-to-windows`)
2. Use Windows service management features
3. Configure Windows Event Log monitoring
4. Implement backup strategies for agent data

### Configuration Management
1. Keep `redis.hybrid.conf` as the optimized template
2. Use version control for configuration changes
3. Test configurations before applying to production
4. Document any custom modifications

## Integration with Agent Memory Server

The hybrid configuration ensures optimal performance for:

- **Session Storage**: Fast working memory operations
- **Vector Operations**: Efficient embedding storage and retrieval  
- **Memory Management**: LRU eviction for session cleanup
- **Persistence**: Durable storage for long-term memories
- **Notifications**: Event-driven session expiration
- **Concurrency**: Multiple agent sessions simultaneously

This setup provides the best balance of development flexibility and production reliability for your agent memory system.
