//! Save an export (CSV/JSON built by the frontend) into the user's Downloads folder.

use std::fs;
use std::path::PathBuf;

/// Keep only a plain file name: no separators, no leading dots, limited charset.
fn sanitize(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.') { c } else { '_' })
        .collect();
    let cleaned = cleaned.trim_start_matches('.').to_string();
    if cleaned.is_empty() { "export.txt".into() } else { cleaned }
}

/// Write `content` to ~/Downloads/<name> (never overwriting: appends -1, -2, ... if it exists).
/// Returns the final path.
#[tauri::command]
pub fn save_export(file_name: String, content: String) -> Result<String, String> {
    let dir = dirs::download_dir().ok_or("Could not find Downloads folder")?;
    let name = sanitize(&file_name);
    let (stem, ext) = match name.rsplit_once('.') {
        Some((s, e)) => (s.to_string(), format!(".{e}")),
        None => (name.clone(), String::new()),
    };
    let mut path: PathBuf = dir.join(&name);
    let mut n = 1;
    while path.exists() {
        path = dir.join(format!("{stem}-{n}{ext}"));
        n += 1;
    }
    fs::write(&path, content).map_err(|e| e.to_string())?;
    Ok(path.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::sanitize;

    #[test]
    fn strips_path_tricks() {
        // "../../etc/passwd" -> ".._.._etc_passwd" -> leading dots trimmed
        assert_eq!(sanitize("../../etc/passwd"), "_.._etc_passwd");
        assert!(!sanitize("../../etc/passwd").contains('/'));
        assert!(!sanitize(".hidden").starts_with('.'));
        assert_eq!(sanitize(""), "export.txt");
    }
}
