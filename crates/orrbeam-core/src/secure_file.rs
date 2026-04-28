//! Cross-platform helper for restricting file access to the current user only.
//!
//! Sensitive on-disk artifacts (the Ed25519 signing key, the TLS private key,
//! and the trusted-peer YAML store) must not be world-readable.
//!
//! - **Unix** (Linux, macOS): chmod 0o600 — owner read/write only.
//! - **Windows**: invoke `icacls` to remove inherited ACEs and grant only the
//!   current user (`%USERNAME%`) read/write. `%APPDATA%` and `%LOCALAPPDATA%`
//!   are already per-user paths, so this is defence-in-depth rather than
//!   the primary access boundary. If `icacls` is unavailable or fails the
//!   call returns Ok — we log a warning but do NOT fail the surrounding
//!   write, since the data has already been persisted to a per-user dir.
//!
//! Tests live alongside each call site (identity, tls, peers).

use std::path::Path;

/// Restrict a file to owner read/write only.
///
/// On Unix this calls `chmod 0o600`. On Windows this invokes `icacls /inheritance:r`
/// followed by `icacls /grant:r %USERNAME%:RW`. On any other target it is a no-op.
///
/// On Windows the result is best-effort: if `icacls` fails (e.g. because the
/// path is on a filesystem without ACL support) the warning is logged via
/// `tracing` and `Ok(())` is returned. The caller decided that the file is
/// allowed to exist with default permissions in that situation.
pub fn restrict_to_owner(path: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
        Ok(())
    }

    #[cfg(target_os = "windows")]
    {
        windows_impl::restrict_to_owner(path)
    }

    #[cfg(not(any(unix, target_os = "windows")))]
    {
        let _ = path;
        Ok(())
    }
}

#[cfg(target_os = "windows")]
mod windows_impl {
    use std::path::Path;
    use std::process::Command;

    /// Best-effort ACL restriction via `icacls`.
    ///
    /// Order matters: we GRANT first, then strip inheritance. The reverse order
    /// (strip then grant) leaves the file inaccessible if the grant fails — and
    /// the grant is the brittle step because principal-name resolution can fail
    /// in workgroup setups where `%USERDOMAIN%` is the literal string
    /// "WORKGROUP" rather than a real SID-resolvable domain.
    ///
    /// Principal resolution tries, in order:
    /// 1. `%USERNAME%` alone — works on local-account workgroup machines where
    ///    `WORKGROUP\<user>` does NOT resolve.
    /// 2. `%COMPUTERNAME%\%USERNAME%` — works on most workgroup machines as the
    ///    canonical local-account form.
    /// 3. `%USERDOMAIN%\%USERNAME%` — works on domain-joined machines.
    ///
    /// We accept the first form that icacls accepts. If all three fail we log
    /// and bail out WITHOUT touching inheritance, so the file stays accessible
    /// under its default ACLs (less secure but never inaccessible).
    pub(super) fn restrict_to_owner(path: &Path) -> std::io::Result<()> {
        let username = match std::env::var("USERNAME") {
            Ok(u) if !u.is_empty() => u,
            _ => {
                tracing::warn!(
                    path = %path.display(),
                    "secure_file: USERNAME unset; skipping icacls — file retains inherited ACLs"
                );
                return Ok(());
            }
        };

        let mut candidates: Vec<String> = vec![username.clone()];
        if let Ok(c) = std::env::var("COMPUTERNAME") {
            if !c.is_empty() {
                candidates.push(format!(r"{c}\{username}"));
            }
        }
        if let Ok(d) = std::env::var("USERDOMAIN") {
            if !d.is_empty() && d != "WORKGROUP" {
                candidates.push(format!(r"{d}\{username}"));
            }
        }

        let path_str = match path.to_str() {
            Some(s) => s,
            None => {
                tracing::warn!(
                    path = %path.display(),
                    "secure_file: path is not valid UTF-8; skipping icacls"
                );
                return Ok(());
            }
        };

        // Step 1: try each candidate principal until icacls accepts one.
        // (M) = Modify = R + W + D (delete) + write-attributes. Without D, a
        // subsequent `std::fs::write` to the same path would fail with
        // PermissionDenied because Windows treats file replacement as a
        // delete-then-create. (R,W) alone is too restrictive in practice.
        let mut applied_principal: Option<String> = None;
        for candidate in &candidates {
            match Command::new("icacls")
                .args([path_str, "/grant:r", &format!("{candidate}:(M)")])
                .output()
            {
                Ok(out) if out.status.success() => {
                    applied_principal = Some(candidate.clone());
                    break;
                }
                Ok(out) => {
                    tracing::debug!(
                        path = %path.display(),
                        candidate = %candidate,
                        stderr = %String::from_utf8_lossy(&out.stderr).trim(),
                        "secure_file: icacls /grant:r rejected this principal; trying next"
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        path = %path.display(),
                        error = %e,
                        "secure_file: icacls not available; file retains default ACLs"
                    );
                    return Ok(());
                }
            }
        }

        let Some(principal) = applied_principal else {
            tracing::warn!(
                path = %path.display(),
                tried = ?candidates,
                "secure_file: no principal resolved; file retains default ACLs"
            );
            return Ok(());
        };

        // Step 2: now that the user has an explicit ACE, drop inherited ACEs.
        // If this fails, the file is still accessible (just less restricted).
        match Command::new("icacls")
            .args([path_str, "/inheritance:r"])
            .output()
        {
            Ok(out) if out.status.success() => {
                tracing::debug!(
                    path = %path.display(),
                    principal = %principal,
                    "secure_file: icacls applied"
                );
            }
            Ok(out) => {
                tracing::warn!(
                    path = %path.display(),
                    stderr = %String::from_utf8_lossy(&out.stderr).trim(),
                    "secure_file: icacls /inheritance:r failed; file is accessible but inheritance not stripped"
                );
            }
            Err(e) => {
                tracing::warn!(
                    path = %path.display(),
                    error = %e,
                    "secure_file: icacls /inheritance:r spawn failed"
                );
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn restrict_to_owner_on_existing_file_succeeds() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("secret.bin");
        std::fs::write(&path, b"test").unwrap();
        restrict_to_owner(&path).expect("restrict succeeds");
        assert!(path.exists());
    }

    #[cfg(unix)]
    #[test]
    fn restrict_to_owner_chmods_to_0600_on_unix() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempdir().unwrap();
        let path = dir.path().join("secret.bin");
        std::fs::write(&path, b"test").unwrap();
        // Set permissive mode first so we know restrict actually changes it.
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
        restrict_to_owner(&path).expect("restrict succeeds");
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "expected 0o600, got {mode:o}");
    }
}
