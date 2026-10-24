use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;

pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const GITHUB_REPO_URL: &str = "https://github.com/mzia/PostureFlow";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateCheckReport {
    pub current_version: String,
    pub repo_path: Option<String>,
    pub branch: String,
    pub local_commit: Option<String>,
    pub remote_commit: Option<String>,
    pub commits_behind: usize,
    pub commits_ahead: usize,
    pub dirty_files_count: usize,
    pub incoming_commits: Vec<String>,
    pub update_available: bool,
    pub summary: String,
}

/// Resolves the local PostureFlow git repository path if available.
pub fn locate_git_repo() -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();

    if let Ok(env_repo) = std::env::var("POSTUREFLOW_REPO") {
        candidates.push(PathBuf::from(env_repo));
    }

    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.clone());
        if let Some(parent) = cwd.parent() {
            candidates.push(parent.to_path_buf());
        }
    }

    if let Ok(exe) = std::env::current_exe() {
        let mut cur = exe.parent();
        while let Some(dir) = cur {
            candidates.push(dir.to_path_buf());
            cur = dir.parent();
        }
    }

    if let Ok(home) = std::env::var("HOME") {
        let home_path = Path::new(&home);
        candidates.push(home_path.join("workspace/PostureFlow"));
        candidates.push(home_path.join("PostureFlow"));
        candidates.push(home_path.join("Projects/PostureFlow"));
        candidates.push(home_path.join("src/PostureFlow"));
    }

    if let Ok(sudo_user) = std::env::var("SUDO_USER") {
        if !sudo_user.is_empty() && sudo_user != "root" {
            for base in ["/home", "/Users"] {
                let user_home = Path::new(base).join(&sudo_user);
                candidates.push(user_home.join("workspace/PostureFlow"));
                candidates.push(user_home.join("PostureFlow"));
            }
        }
    }

    for candidate in candidates {
        if candidate.join(".git").exists() && candidate.join("Cargo.toml").exists() {
            return Some(candidate);
        }
    }

    None
}

fn run_git(repo: &Path, args: &[&str]) -> Option<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .ok()?;
    if out.status.success() {
        Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        None
    }
}

/// Checks if updates are available on `origin/<branch>` or GitHub.
pub fn check_for_updates(fetch_remote: bool) -> UpdateCheckReport {
    let mut report = UpdateCheckReport {
        current_version: APP_VERSION.to_string(),
        branch: "main".to_string(),
        ..Default::default()
    };

    if let Some(repo) = locate_git_repo() {
        report.repo_path = Some(repo.display().to_string());

        if let Some(branch) = run_git(&repo, &["rev-parse", "--abbrev-ref", "HEAD"]) {
            if !branch.is_empty() && branch != "HEAD" {
                report.branch = branch;
            }
        }

        if fetch_remote {
            let _ = Command::new("git")
                .arg("-C")
                .arg(&repo)
                .args(["fetch", "origin", "--prune", "--quiet"])
                .output();
        }

        let remote_ref = format!("origin/{}", report.branch);

        report.local_commit = run_git(&repo, &["log", "-1", "--format=%h (%cr) %s", "HEAD"]);
        report.remote_commit = run_git(&repo, &["log", "-1", "--format=%h (%cr) %s", &remote_ref]);

        if let Some(behind_str) = run_git(
            &repo,
            &["rev-list", "--count", &format!("HEAD..{}", remote_ref)],
        ) {
            report.commits_behind = behind_str.parse::<usize>().unwrap_or(0);
        }

        if let Some(ahead_str) = run_git(
            &repo,
            &["rev-list", "--count", &format!("{}..HEAD", remote_ref)],
        ) {
            report.commits_ahead = ahead_str.parse::<usize>().unwrap_or(0);
        }

        if let Some(status_out) = run_git(&repo, &["status", "--porcelain"]) {
            report.dirty_files_count = status_out
                .lines()
                .filter(|line| !line.trim().is_empty())
                .count();
        }

        if report.commits_behind > 0 {
            if let Some(log_out) = run_git(
                &repo,
                &[
                    "log",
                    &format!("HEAD..{}", remote_ref),
                    "--oneline",
                    "-n",
                    "10",
                ],
            ) {
                report.incoming_commits = log_out
                    .lines()
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            }
        }

        report.update_available = report.commits_behind > 0;

        if report.update_available {
            report.summary = format!(
                "Update available! {} new commit(s) on origin/{}{}",
                report.commits_behind,
                report.branch,
                if report.dirty_files_count > 0 {
                    format!(" ({} local uncommitted file(s))", report.dirty_files_count)
                } else {
                    String::new()
                }
            );
        } else if report.commits_ahead > 0 {
            report.summary = format!(
                "Up to date with origin/{} ({} local commit(s) ahead{})",
                report.branch,
                report.commits_ahead,
                if report.dirty_files_count > 0 {
                    format!(", {} uncommitted file(s)", report.dirty_files_count)
                } else {
                    String::new()
                }
            );
        } else {
            let short_sha = run_git(&repo, &["rev-parse", "--short", "HEAD"])
                .unwrap_or_else(|| "HEAD".to_string());
            report.summary = format!(
                "Up to date (v{}, {} on {}{})",
                report.current_version,
                short_sha,
                report.branch,
                if report.dirty_files_count > 0 {
                    format!(", {} uncommitted file(s)", report.dirty_files_count)
                } else {
                    String::new()
                }
            );
        }

        return report;
    }

    // Fallback: Query GitHub API via curl if no git checkout is present
    if fetch_remote {
        if let Ok(out) = Command::new("curl")
            .args([
                "-fsSL",
                "--max-time",
                "4",
                "https://api.github.com/repos/mzia/PostureFlow/commits/main",
            ])
            .output()
        {
            if out.status.success() {
                if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&out.stdout) {
                    let sha = json
                        .get("sha")
                        .and_then(|v| v.as_str())
                        .map(|s| s.chars().take(7).collect::<String>());
                    let msg = json
                        .get("commit")
                        .and_then(|c| c.get("message"))
                        .and_then(|m| m.as_str())
                        .and_then(|m| m.lines().next())
                        .unwrap_or("Latest main commit");
                    if let Some(short_sha) = sha {
                        report.remote_commit = Some(format!("{} {}", short_sha, msg));
                        report.summary = format!(
                            "Installed v{} • Latest remote on main: {} ({})",
                            report.current_version, short_sha, msg
                        );
                        return report;
                    }
                }
            }
        }
    }

    report.summary = format!(
        "PostureFlow v{} (No local git repository detected)",
        report.current_version
    );
    report
}

/// Fast-forwards the local git checkout from `origin/<branch>`, safely stashing and restoring
/// any uncommitted tracked modifications.
pub fn apply_self_update() -> Result<String, String> {
    let repo = locate_git_repo().ok_or_else(|| {
        "Could not locate local PostureFlow git repository. Clone https://github.com/mzia/PostureFlow or set POSTUREFLOW_REPO.".to_string()
    })?;

    let report = check_for_updates(true);
    if !report.update_available {
        return Ok(format!("Already up to date: {}", report.summary));
    }

    let remote_ref = format!("origin/{}", report.branch);

    // Check if tracked files have local changes that need stashing before fast-forward
    let tracked_dirty = run_git(&repo, &["status", "--porcelain", "-uno"])
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false);

    if tracked_dirty {
        let stash_out = Command::new("git")
            .arg("-C")
            .arg(&repo)
            .args(["stash", "push", "-m", "postureflow-auto-update"])
            .output()
            .map_err(|e| format!("Failed to run git stash: {}", e))?;
        if !stash_out.status.success() {
            return Err(format!(
                "Failed to stash local changes before update: {}",
                String::from_utf8_lossy(&stash_out.stderr).trim()
            ));
        }
    }

    let merge_out = Command::new("git")
        .arg("-C")
        .arg(&repo)
        .args(["merge", "--ff-only", &remote_ref])
        .output()
        .map_err(|e| format!("Failed to run git merge: {}", e))?;

    if !merge_out.status.success() {
        if tracked_dirty {
            let _ = Command::new("git")
                .arg("-C")
                .arg(&repo)
                .args(["stash", "pop"])
                .output();
        }
        return Err(format!(
            "Fast-forward merge from {} failed: {}",
            remote_ref,
            String::from_utf8_lossy(&merge_out.stderr).trim()
        ));
    }

    if tracked_dirty {
        let pop_out = Command::new("git")
            .arg("-C")
            .arg(&repo)
            .args(["stash", "pop"])
            .output()
            .map_err(|e| format!("Updated, but failed to run git stash pop: {}", e))?;
        if !pop_out.status.success() {
            return Err(format!(
                "Fast-forwarded to {}, but 'git stash pop' reported conflicts. Resolve manually in {}.",
                remote_ref,
                repo.display()
            ));
        }
    }

    let new_head = run_git(&repo, &["log", "-1", "--format=%h (%s)", "HEAD"])
        .unwrap_or_else(|| "latest".to_string());

    Ok(format!(
        "Successfully pulled {} new commit(s) from {} -> {}.\nRun './install.sh' in {} to rebuild and deploy updated binaries.",
        report.commits_behind,
        remote_ref,
        new_head,
        repo.display()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_for_updates_local_only() {
        let rep = check_for_updates(false);
        assert_eq!(rep.current_version, APP_VERSION);
        assert!(!rep.summary.is_empty());
    }
}
