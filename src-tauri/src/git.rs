//! Git integration for file diffs.
//!
//! Provides functionality to get file contents from HEAD and working directory
//! for comparison in the diff viewer.

use git2::Repository;
use std::fs;
use std::path::Path;

/// Result of getting a git file diff - original (HEAD) and current content.
#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitFileDiff {
    /// Content of the file at HEAD (original)
    pub original: String,
    /// Current content of the file in working directory
    pub current: String,
    /// Whether the file exists at HEAD
    pub exists_at_head: bool,
    /// Whether the file exists in working directory
    pub exists_in_workdir: bool,
}

/// Get the original (HEAD) and current content of a file for diff comparison.
///
/// # Arguments
/// * `project_path` - Path to the project/repository root (used as fallback)
/// * `file_path` - Path to the file (can be absolute or relative to project)
///
/// This function discovers the git repository that actually contains the file,
/// which may be different from project_path when editing files outside the project.
pub fn get_git_file_diff(project_path: &str, file_path: &str) -> Result<GitFileDiff, String> {
    // Determine the actual file path on disk
    let actual_file_path = if Path::new(file_path).is_absolute() {
        Path::new(file_path).to_path_buf()
    } else {
        Path::new(project_path).join(file_path)
    };

    // Try to discover the git repository that contains this file
    // This handles the case where the file is in a different repo than project_path
    let (repo, relative_path) = if actual_file_path.exists() {
        // Discover repo from the file's parent directory
        let parent_dir = actual_file_path.parent().unwrap_or(Path::new("."));
        match Repository::discover(parent_dir) {
            Ok(repo) => {
                // Get the repo's workdir to compute relative path
                let workdir = repo
                    .workdir()
                    .ok_or_else(|| "Repository has no working directory".to_string())?;
                let rel = actual_file_path
                    .strip_prefix(workdir)
                    .map(|p| p.to_path_buf())
                    .unwrap_or_else(|_| Path::new(file_path).to_path_buf());
                (repo, rel)
            }
            Err(_) => {
                // No git repo found, fall back to project_path
                let repo = Repository::open(project_path)
                    .map_err(|e| format!("Failed to open repository: {}", e))?;
                (repo, Path::new(file_path).to_path_buf())
            }
        }
    } else {
        // File doesn't exist, try project_path repo
        let repo = Repository::open(project_path)
            .map_err(|e| format!("Failed to open repository: {}", e))?;
        (repo, Path::new(file_path).to_path_buf())
    };

    // Get HEAD commit
    let head = repo.head().map_err(|e| format!("Failed to get HEAD: {}", e))?;
    let head_commit = head
        .peel_to_commit()
        .map_err(|e| format!("Failed to get HEAD commit: {}", e))?;
    let head_tree = head_commit
        .tree()
        .map_err(|e| format!("Failed to get HEAD tree: {}", e))?;

    // Try to get file content from HEAD using the relative path
    let (original, exists_at_head) = match head_tree.get_path(&relative_path) {
        Ok(entry) => {
            let obj = entry
                .to_object(&repo)
                .map_err(|e| format!("Failed to get object: {}", e))?;
            let blob = obj
                .as_blob()
                .ok_or_else(|| "Entry is not a blob".to_string())?;
            let content = String::from_utf8_lossy(blob.content()).to_string();
            (content, true)
        }
        Err(_) => {
            // File doesn't exist at HEAD (new file)
            (String::new(), false)
        }
    };

    // Get current file content from working directory
    let (current, exists_in_workdir) = if actual_file_path.exists() {
        let content = fs::read_to_string(&actual_file_path)
            .map_err(|e| format!("Failed to read current file: {}", e))?;
        (content, true)
    } else {
        // File was deleted
        (String::new(), false)
    };

    Ok(GitFileDiff {
        original,
        current,
        exists_at_head,
        exists_in_workdir,
    })
}
