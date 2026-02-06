# Redis for Windows - LLM Agent Ready

**A production-ready Redis server for Windows with native LLM agent integration and memory management capabilities.**

## 🤖 Built for LLM Agents

This project provides a high-performance Redis setup specifically optimized for:

- **Agent Memory Management** - Working and long-term memory for AI agents
- **Vector Storage & Retrieval** - Optimized for embedding and semantic search
- **Session Management** - Isolated contexts for multiple agent sessions  
- **Real-time Performance** - Low-latency responses for agent workflows
- **Production Deployment** - Windows service with enterprise features

## 🔗 Agent-Memory-Server Integration

Seamlessly integrates with [agent-memory-server](https://github.com/david-t-martel/agent-memory-server) to provide:

- **Conversational Memory** - Automatic summarization and context management
- **Semantic Search** - Vector-based memory retrieval 
- **Multi-Session Support** - Isolated memory per agent session
- **REST & MCP APIs** - Multiple integration options for different agent frameworks
- **Background Processing** - Async memory compaction and optimization

## 🚀 Quick Start for Agents

```powershell
# Clone and setup
git clone https://github.com/david-t-martel/redis-windows.git
cd redis-windows

# Setup for LLM agents (one command)
.\build.ps1 -PrepareForAgents

# Start Redis service
net start Redis

# Your agents can now connect to:
# Redis: redis://localhost:6379
# Agent Memory API: http://localhost:8000
# Agent Memory MCP: ws://localhost:8001
```

## 📋 What You Get

### Core Components
- **Redis Server** (`redis-server.exe`) - High-performance data store
- **Redis CLI** (`redis-cli.exe`) - Command-line interface
- **Windows Service** (`RedisService.exe`) - Reliable background operation
- **Agent Configuration** (`redis-agent.conf`) - Optimized for agent workloads

### Agent Integration
- **Memory Server Integration** - Automatic setup of agent-memory-server
- **Optimized Configurations** - Pre-tuned for LLM agent access patterns
- **Development Tools** - VS Code tasks, debugging, and monitoring
- **Production Features** - Logging, security, and performance monitoring

## 🛠️ Build Options

### Interactive Setup
```powershell
.\build.ps1
# Choose from menu options including agent preparation
```

### Direct Commands
```powershell
# Prepare for agents (includes Redis + agent-memory-server)
.\build.ps1 -PrepareForAgents

# Build Redis service only
.\build.ps1 -BuildOnly

# Download latest release
.\build.ps1 -CheckLatestRelease

# Setup agent-memory-server integration
.\build.ps1 -ConfigureMemoryServer

# Trigger GitHub workflow to build Redis
.\build.ps1 -GitHubWorkflow
```

## 🔧 Agent Configuration

### Environment Variables
```bash
REDIS_URL=redis://localhost:6379
AGENT_MEMORY_API_URL=http://localhost:8000
AGENT_MEMORY_MCP_URL=ws://localhost:8001
```

### Python Agent Example
```python
from agent_memory_client import MemoryClient
import redis

# Redis direct access
redis_client = redis.Redis(host='localhost', port=6379)

# Agent memory server
memory_client = MemoryClient(api_url='http://localhost:8000')

# Store conversation memory
memory_client.store_working_memory(
    session_id="agent_session_1",
    messages=[{"role": "user", "content": "Hello"}]
)
```

### MCP Integration
```json
{
  "mcpServers": {
    "agent-memory": {
      "command": "agent-memory-server",
      "args": ["run-mcp"],
      "env": {
        "REDIS_URL": "redis://localhost:6379"
      }
    }
  }
}
```

## 📖 Documentation

- **[Agent Integration Guide](AGENT_INTEGRATION_GUIDE.md)** - Complete guide for LLM agent developers
- **[Quick Start Guide](QUICKSTART.md)** - Get running in 5 minutes
- **[Setup Guide](SETUP_GUIDE.md)** - Detailed installation and configuration
- **[Project Status](PROJECT_STATUS.md)** - Current features and roadmap

## 🏗️ Architecture

```text
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   LLM Agent     │───▶│ Agent Memory    │───▶│ Redis Server    │
│   (Claude,      │    │ Server          │    │ (This Project)  │
│   GPT, etc.)    │    │ (Python/FastAPI)│    │ (C#/.NET)       │
└─────────────────┘    └─────────────────┘    └─────────────────┘
                              │
                              ▼
                       ┌─────────────────┐
                       │ Vector Store    │
                       │ (Redis/Other)   │
                       └─────────────────┘
```

## ⚡ Performance Features

- **Memory Optimization** - Configured for high-throughput agent workloads
- **Lazy Freeing** - Non-blocking memory cleanup
- **Active Defragmentation** - Automatic memory optimization
- **Connection Pooling** - Efficient multi-agent connection handling
- **Background Persistence** - Non-blocking data durability

## 🔒 Security Features

- **Authentication** - Redis password protection
- **Network Security** - Configurable bind addresses
- **Windows Integration** - Event log monitoring
- **Agent Isolation** - Session-based memory separation
- **OAuth2 Support** - Via agent-memory-server integration

## 🏢 Production Ready

### Windows Service
- **Automatic Startup** - Starts with Windows
- **Process Management** - Automatic restart on failure  
- **Event Logging** - Integrated with Windows Event Log
- **Performance Monitoring** - Built-in metrics and alerts

### Monitoring & Debugging
- **VS Code Integration** - Full debugging support
- **Performance Metrics** - Redis INFO and slow log
- **Health Checks** - Service status monitoring
- **Log Management** - Structured logging for troubleshooting

## 🤝 Contributing

We welcome contributions, especially for:
- Agent framework integrations
- Performance optimizations
- Documentation improvements
- Testing and validation

## 📄 License

This project is licensed under the same terms as Redis - see [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- **Redis Team** - For the amazing Redis database
- **Microsoft** - For .NET hosting services
- **Agent Memory Server** - For the Python memory management framework

---

**Ready to power your AI agents with high-performance memory? Get started with the [Agent Integration Guide](AGENT_INTEGRATION_GUIDE.md)!**


## Disclaimer

This GitHub organization and its repositories are not affiliated with, endorsed by, or sponsored by Redis Ltd. The license provided here applies only to the content of this repository, and does not apply to the official [Redis](https://github.com/redis/redis) project or its software.

We suggest that you use it for local development and follow Redis official guidance to deploy it on Linux for production environment. This project doesn't bear any responsibility for any losses caused by using it and is only for learning and exchange purposes.


