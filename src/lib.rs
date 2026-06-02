use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum NormalizeError {
    EmptyInput,
}

pub fn normalize_path(
    _root: &Path,
    _origin: &Path,
    input: &Path,
) -> Result<PathBuf, NormalizeError> {
    if input.as_os_str().is_empty() {
        return Err(NormalizeError::EmptyInput);
    }
    Ok(input.to_path_buf())
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{NormalizeError, normalize_path};

    #[test]
    fn empty_path_returns_error() {
        let root = Path::new("/project");
        let origin_dir = Path::new("/project");
        let input = Path::new("");
        let result = normalize_path(root, origin_dir, input);
        assert!(matches!(result, Err(NormalizeError::EmptyInput)));
    }

    #[test]
    fn plain_filename_with_root_at_cwd_returns_filename() {
        let root = Path::new("/project");
        let origin_dir = Path::new("/project");
        let input = Path::new("main.rs");
        let result = normalize_path(root, origin_dir, input);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Path::new("main.rs"));
    }
}
