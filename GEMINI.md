# redis-windows - Project Overview

This project provides a native Redis 8.4.0 server for Windows, wrapped in a .NET 8 Windows Service. It is optimized for LLM Agent workloads with the `agent-sync` Rust library providing high-performance distributed coordination.

## Core Components
- **Redis Server**: Native port of Redis 8.4.0 (upstream merged).
- **RedisService (.NET 8)**: Background worker service managing Redis lifecycle, logging, and graceful shutdown.
- **agent-sync (Rust)**: High-performance coordination library - task queues, shared context, pub/sub notifications, agent registry.

## Redis Configuration Standards
- **Port**: `6380` (primary for Windows agents), `6379` (WSL dev).
- **Memory**: `2GB` limit with `allkeys-lru` eviction policy.
- **Persistence**: `AOF` (appendonly) enabled for agent memory durability.
- **Optimization**: Large payload support, active defragmentation, keyspace events.

## Agent Integration
The `agent-sync` Rust library provides:
- **Distributed Task Queue**: Priority-based task distribution with dependencies.
- **Shared Context**: Type-safe key-value storage with TTL and semantic search.
- **Notification Bus**: Pub/sub messaging with topic patterns and direct messaging.
- **Agent Registry**: Liveness tracking with heartbeats and capability discovery.

## Key Dependencies
- **Rust**: redis 0.32, tokio 1.35, bb8 0.9, bb8-redis 0.24, thiserror 2.0
- **.NET**: 8.0, Microsoft.Extensions.Hosting.WindowsServices 8.0.1

## Development Workflows
- **Build**: `build.ps1` for .NET service, `cargo build` for agent-sync.
- **Deployment**: `install-redis-service.ps1` to register the Windows Service.
- **Dual Instance**: `manage-dual-redis.ps1` for dev (6379) + agent (6380) instances.

## Known Deficiencies
- **Port Inconsistency**: Some docs still reference `6379`. Standardize on `6380` for all Windows agent tools.
- **Vector Search**: Current implementation uses local Rust-side approaches. Future work should evaluate Redis 8 native vector sets.
