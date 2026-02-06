# Redis-Windows Context - 2026-02-06 Session D

## Context ID: ctx-redis-windows-20260206d

## State Summary

Redis 8.4.0 (MSYS2 build) is fully operational as a Windows Service on port 6380.
The custom Rust service wrapper (`redis-service.exe`) has been fixed and modernized:
- Path resolution handles Windows verbatim prefixes and service CWD
- Uses cygdrive paths for MSYS2 Redis compatibility
- Dependencies modernized (windows 0.58, windows-service 0.8, thiserror 2.0)
- Config updated for Redis 8.x MSYS2 constraints

## Recent Changes

| File | Change |
|------|--------|
| `process_manager.rs` | Use `to_cygdrive()` for MSYS2 path handling (not forward-slash) |
| `path_convert.rs` | Strip verbatim prefixes; `to_cygdrive()` active, `to_forward_slash()` dead_code |
| `main.rs` | `resolve_config_path()` falls back to exe dir when CWD is system32 |
| `error.rs` | Migrated to thiserror derive |
| `event_log.rs` | Migrated from winapi 0.3 to windows 0.58 crate |
| `redis-agent.conf` | Disabled activedefrag, active-rehashing; lowered maxclients to 3000 |
| `install-redis-service.ps1` | Full rewrite with MSYS2 DLL copying, verification steps |
| `install-redis-service.bat` | New batch wrapper with admin check |

## Key Decisions

1. **Cygdrive over forward-slash paths**: MSYS2 Redis treats `C:/...` as relative paths.
   Cygdrive `/cygdrive/C/...` is the only format MSYS2 recognizes as absolute.
   Spaces in paths work because Rust's `Command::arg()` doesn't do shell splitting.

2. **Redis 8.4.0 MSYS2 build**: Uses libc not jemalloc, limited to ~3200 file descriptors,
   `active-rehashing` removed in 8.x, `activedefrag` needs jemalloc.

3. **windows crate 0.58 over winapi 0.3**: Modern Microsoft-maintained FFI. HANDLE is
   `*mut c_void` not `isize`, needs feature gating for `Win32_Security`.

## Verification Results

- Service: Running (Redis 8.4.0, port 6380)
- PING: PONG
- 80 tests pass (68 lib + 12 service)
- Clippy: clean
- rag-redis-mcp-server.exe: connects successfully
