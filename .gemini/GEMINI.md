# Gemini CLI - redis-windows Configuration

## Project Context
- **Rust Standards**: Defined in `.claude/CLAUDE.md`. Focus on performance (async, connection pooling) and type safety.
- **Redis Port**: Always use `6380` for internal agent tools on Windows.
- **Python Usage**: Always use `uv run python`.

## Key Files
- `redis-agent.conf`: Primary Redis configuration (port 6380, 2GB, LRU).
- `agent-sync/src/`: Source for agent coordination logic (Rust).
- `Program.cs`: Windows Service entry point (.NET 8).

## Agent Guidelines
1. **Performance**: Prioritize `Arc<T>` and `Cow` in Rust code. Use async throughout.
2. **Persistence**: Ensure Redis keys used for agent state have appropriate TTLs.
3. **Cross-Platform**: Use `PathBuf` and `dunce` for Windows UNC path compatibility.
4. **Dependencies**: redis 0.32, tokio 1.35, bb8 0.9, bb8-redis 0.24, thiserror 2.0.
