//! Path conversion utilities for Windows/MSYS2/Cygwin compatibility.
//!
//! Redis for Windows (MSYS2-based) requires Unix-style paths for configuration.
//! This module converts Windows paths to the appropriate format.

/// Convert a Windows path to cygdrive format (`C:\foo` -> `/cygdrive/C/foo`).
///
/// This matches the behavior of the original C# service wrapper, which the
/// MSYS2/Cygwin-compiled Redis executable requires for config file paths.
///
/// Handles Windows verbatim path prefixes (`\\?\`, `//?/`) that can be
/// produced by `dunce::canonicalize()` or the OS when running as a service.
pub fn to_cygdrive(path: &str) -> String {
    // Strip Windows verbatim path prefixes before any conversion.
    // \\?\UNC\server\share -> \\server\share (UNC)
    // \\?\C:\... -> C:\... (verbatim disk)
    // //?/C:/... -> C:/... (forward-slash variant)
    let path = path
        .strip_prefix(r"\\?\UNC\")
        .map(|rest| format!(r"\\{rest}"))
        .unwrap_or_else(|| {
            path.strip_prefix(r"\\?\")
                .or_else(|| path.strip_prefix("//?/"))
                .unwrap_or(path)
                .to_string()
        });

    let path = path.replace('\\', "/");
    // Match drive letter pattern like "C:/"
    if path.len() >= 2 && path.as_bytes()[1] == b':' {
        let drive = path.as_bytes()[0];
        let rest = &path[2..]; // includes the leading /
        format!("/cygdrive/{}{}", drive as char, rest)
    } else {
        path
    }
}

/// Convert backslashes to forward slashes only, preserving drive letter.
#[allow(dead_code)]
pub fn to_forward_slash(path: &str) -> String {
    path.replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cygdrive_basic() {
        assert_eq!(
            to_cygdrive(r"C:\redis\redis-agent.conf"),
            "/cygdrive/C/redis/redis-agent.conf"
        );
    }

    #[test]
    fn test_cygdrive_different_drive() {
        assert_eq!(
            to_cygdrive(r"T:\projects\redis-windows\redis.conf"),
            "/cygdrive/T/projects/redis-windows/redis.conf"
        );
    }

    #[test]
    fn test_cygdrive_forward_slashes_input() {
        assert_eq!(
            to_cygdrive("C:/Program Files/Redis/redis.conf"),
            "/cygdrive/C/Program Files/Redis/redis.conf"
        );
    }

    #[test]
    fn test_cygdrive_relative_path_unchanged() {
        assert_eq!(to_cygdrive("redis.conf"), "redis.conf");
    }

    #[test]
    fn test_cygdrive_unix_path_unchanged() {
        assert_eq!(
            to_cygdrive("/etc/redis/redis.conf"),
            "/etc/redis/redis.conf"
        );
    }

    #[test]
    fn test_cygdrive_verbatim_backslash_prefix() {
        assert_eq!(
            to_cygdrive(r"\\?\C:\Program Files\Redis\redis-agent.conf"),
            "/cygdrive/C/Program Files/Redis/redis-agent.conf"
        );
    }

    #[test]
    fn test_cygdrive_verbatim_forward_slash_prefix() {
        assert_eq!(
            to_cygdrive("//?/C:/Program Files/Redis/redis-agent.conf"),
            "/cygdrive/C/Program Files/Redis/redis-agent.conf"
        );
    }

    #[test]
    fn test_cygdrive_verbatim_unc_prefix() {
        assert_eq!(
            to_cygdrive(r"\\?\UNC\server\share\file.conf"),
            "//server/share/file.conf"
        );
    }

    #[test]
    fn test_forward_slash_basic() {
        assert_eq!(
            to_forward_slash(r"C:\redis\redis.conf"),
            "C:/redis/redis.conf"
        );
    }

    #[test]
    fn test_forward_slash_no_change() {
        assert_eq!(
            to_forward_slash("already/forward/slashes"),
            "already/forward/slashes"
        );
    }
}


