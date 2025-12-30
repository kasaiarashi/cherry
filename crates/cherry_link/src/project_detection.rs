use std::path::Path;

/// Detects if a directory contains an Unreal Engine project
pub fn is_unreal_project(path: &Path) -> bool {
    if !path.is_dir() {
        return false;
    }

    // Check for .uproject file in the directory
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            if let Some(ext) = entry.path().extension() {
                if ext == "uproject" {
                    return true;
                }
            }
        }
    }

    false
}

/// Finds the .uproject file in a directory, returning its name (without extension)
pub fn find_uproject_name(path: &Path) -> Option<String> {
    if !path.is_dir() {
        return None;
    }

    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let entry_path = entry.path();
            if let Some(ext) = entry_path.extension() {
                if ext == "uproject" {
                    return entry_path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .map(|s| s.to_string());
                }
            }
        }
    }

    None
}

/// Finds the .uproject file path in a directory
pub fn find_uproject_path(path: &Path) -> Option<std::path::PathBuf> {
    if !path.is_dir() {
        return None;
    }

    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let entry_path = entry.path();
            if let Some(ext) = entry_path.extension() {
                if ext == "uproject" {
                    return Some(entry_path);
                }
            }
        }
    }

    None
}
