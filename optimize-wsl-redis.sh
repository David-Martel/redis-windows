#!/bin/bash
# Script to optimize WSL Redis for agent-memory-server workloads
# This script updates the WSL Redis configuration with agent-optimized settings

echo "🔧 Optimizing WSL Redis for Agent Memory Server..."

# Backup original configuration
sudo cp /etc/redis/redis.conf /etc/redis/redis.conf.backup.$(date +%Y%m%d_%H%M%S)
echo "✅ Backed up original configuration"

# Add agent optimizations to Redis config
cat << 'EOF' | sudo tee -a /etc/redis/redis.conf

################################# AGENT MEMORY OPTIMIZATIONS ####################
# Optimized configuration for LLM Agent Memory Server workloads
# These settings are specifically tuned for agent-memory-server performance

# Port Configuration - WSL Redis on default port
# This allows Windows Redis to run on port 6380 simultaneously
port 6379

# Memory Management for Agent Workloads
# Set memory limit to 2GB for agent operations
maxmemory 2gb

# Use LRU eviction policy for agent memory patterns
# This works well with session-based agent memory access
maxmemory-policy allkeys-lru

# Enable lazy freeing for better performance with large agent datasets
lazyfree-lazy-eviction yes
lazyfree-lazy-expire yes
lazyfree-lazy-server-del yes

# Active defragmentation for vector storage optimization
# Helps with memory fragmentation from embedding vectors
activedefrag yes
active-defrag-ignore-bytes 100mb
active-defrag-threshold-lower 10
active-defrag-threshold-upper 100
active-defrag-cycle-min 1
active-defrag-cycle-max 25

# Persistence optimized for agent memory
# More frequent saves for agent session data
save 900 1 300 10 60 1000

# Enable AOF for agent session persistence
appendonly yes
appendfsync everysec
no-appendfsync-on-rewrite no
auto-aof-rewrite-percentage 100
auto-aof-rewrite-min-size 64mb

# Keyspace notifications for agent memory events
# Enable expiration events for session cleanup
notify-keyspace-events "Ex"

# Data structure optimizations for agent memory patterns
# Optimized for small hash tables (agent session data)
hash-max-listpack-entries 512
hash-max-listpack-value 64

# Optimized for sets (agent entity tracking)
set-max-intset-entries 512

# Optimized for sorted sets (agent memory ranking)
zset-max-listpack-entries 128
zset-max-listpack-value 64

# Client connection optimization for agent workloads
# Higher timeout for agent processing
timeout 300

# More aggressive TCP keepalive for agent connections
tcp-keepalive 60

# Higher client output buffer for large agent responses
client-output-buffer-limit normal 256mb 128mb 60
client-output-buffer-limit replica 256mb 64mb 60
client-output-buffer-limit pubsub 64mb 16mb 60

# Performance tuning for agent vector operations
# Disable slow log for development (agent operations may be slow)
slowlog-log-slower-than -1

# Increase max clients for concurrent agent sessions
maxclients 1000

# Threading configuration for agent workloads
# Use more I/O threads for better agent concurrency
io-threads 4
io-threads-do-reads yes

# Memory optimization for agent embedding storage
# Faster hash function for agent data
hash-function fnv
EOF

echo "✅ Added agent optimization settings"

# Test the configuration
echo "🧪 Testing Redis configuration..."
if redis-server --test-config -c /etc/redis/redis.conf; then
    echo "✅ Configuration is valid"
    
    # Restart Redis service
    echo "🔄 Restarting Redis service..."
    sudo systemctl restart redis-server
    
    # Check status
    if sudo systemctl is-active --quiet redis-server; then
        echo "✅ Redis service restarted successfully"
        
        # Test connection
        if redis-cli ping > /dev/null 2>&1; then
            echo "✅ Redis is responding to connections"
            
            # Show memory info
            echo ""
            echo "📊 Current Redis Memory Configuration:"
            redis-cli config get maxmemory
            redis-cli config get maxmemory-policy
            redis-cli info memory | grep used_memory_human
        else
            echo "❌ Redis is not responding to connections"
        fi
    else
        echo "❌ Failed to start Redis service"
        echo "📋 Service status:"
        sudo systemctl status redis-server
    fi
else
    echo "❌ Configuration test failed"
    echo "🔄 Restoring backup..."
    sudo cp /etc/redis/redis.conf.backup.$(date +%Y%m%d)* /etc/redis/redis.conf
fi

echo ""
echo "🎯 Next Steps:"
echo "1. Test agent-memory-server connection: python -c \"import redis; r = redis.Redis(); print('Redis:', r.ping())\""
echo "2. Start agent-memory-server: python -m agent_memory_server.cli api --port 8000"
echo "3. Monitor Redis performance: redis-cli info stats"
