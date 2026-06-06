use std::path::Path;

pub fn display_path(path: &Path) -> String {
    let s = format!("{:?}", path);
    s.strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .unwrap_or(&s)
        .to_string()
}
