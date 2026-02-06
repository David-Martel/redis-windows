# Redis-Windows Context: Service Path Fix & Dependency Modernization

**Context ID:** ctx-redis-windows-20260206c
**Created:** 2026-02-06T17:17:00Z
**Branch:** main @ 994cff8
**Session Focus:** Fix Redis Windows Service `//?/` path bug + modernize dependencies

## State Summary

Fixed the critical bug preventing the Redis Windows Service (`redis-service.exe`) from starting.
The root cause was `to_cygdrive()` not stripping Windows verbatim path prefixes (`\\?\` / `//?/`)
produced by `dunce::canonicalize()` when running as a Windows service (CWD = system32).

Additionally modernized: winapi→windows crate, manual Error→thiserror, windows-service 0.7→0.8.
Also fixed `.gitignore` which was preventing `agent-sync/src/bin/` from being tracked.

## Commits

| SHA | Message |
|-----|---------|
| c383297 | fix(.gitignore): Use root-relative paths for .NET build artifacts |
| 994cff8 | fix(agent-sync): Fix service path resolution and modernize dependencies |

## Files Changed

| File | Change |
|------|--------|
| `.gitignore` | `bin/` → `/bin/` (root-relative, stops masking src/bin/) |
| `agent-sync/src/bin/redis_service/path_convert.rs` | Strip `\\?\`, `//?/`, `\\?\UNC\` prefixes; 3 new tests |
| `agent-sync/src/bin/redis_service/main.rs` | `resolve_config_path()` falls back to exe dir |
| `agent-sync/src/bin/redis_service/error.rs` | thiserror derive (was manual Display+Error) |
| `agent-sync/src/bin/redis_service/event_log.rs` | winapi 0.3 → windows 0.58 crate |
| `agent-sync/src/bin/redis_service/process_manager.rs` | No changes (newly tracked) |
| `agent-sync/src/bin/redis_service/service_handler.rs` | No changes (newly tracked) |
| `agent-sync/Cargo.toml` | windows-service 0.8, winapi→windows 0.58 |
| `agent-sync/deny.toml` | winapi/widestring → windows/windows-core |

## Key Decisions

1. **windows crate v0.58** (not 0.61/0.62): Chosen because it's a known stable version with the `Param<PCWSTR>` generic API. The lockfile already had windows-core 0.62 from transitive deps, but 0.58 is correct for our direct dependency.

2. **HANDLE stored as isize**: windows 0.58 defines `HANDLE(*mut c_void)` which isn't `Send+Sync`. We cast to `isize` for storage in `EventLogLayer` (which must be `Send+Sync` as a tracing Layer).

3. **Win32_Security feature required**: `ReportEventW` takes an `Option<PSID>` parameter, which pulls in Win32_Security. Not obvious from the function name alone.

4. **Root-relative gitignore**: Changed `bin/` to `/bin/` because the wildcard pattern was masking the entire `agent-sync/src/bin/` directory, preventing Rust service source from being tracked.

## Verification Results

- **cargo clippy --features service --all-targets -- -D warnings**: PASS (0 errors)
- **cargo nextest run --features service**: 80/80 PASS
- **cargo build --features service --release**: PASS (1.7MB binary)
- **dotnet build RedisService.csproj -c Release**: PASS (0 errors, 0 warnings)

## Next Steps

1. **Deploy**: Copy release binary to `C:\Program Files\Redis\redis-service.exe`, restart Redis service
2. **Verify live**: `redis-cli -p 6380 PING`, check Event Log for clean startup
3. **Push**: `git push origin main` (2 commits ahead)

## Agent Registry

| Agent | Task | Status |
|-------|------|--------|
| claude-opus | Path fix + dependency modernization | Complete |
| context7 | windows-rs API docs lookup | Complete |
| context7 | windows-service 0.8 API verification | Complete |
