use std::path::{Path, PathBuf, Component};

#[derive(Debug)]
pub enum NormalizeError {
    EmptyInput,
}

pub fn normalize_path(
    _root: &Path,
    origin: &Path,
    input: &Path,
) -> Result<PathBuf, NormalizeError> {
    if input.as_os_str().is_empty() {
        return Err(NormalizeError::EmptyInput);
    }
    let mut stack = Vec::new();
    let origin_joint_input = origin.join(input);
    for component in origin_joint_input.components() {
        match component {
            Component::CurDir => (),
            Component::ParentDir => { stack.pop(); },
            Component::Prefix(_) => { stack.push(component); },
            Component::Normal(_) => { stack.push(component); },
            Component::RootDir => { stack.push(component) },
        }
    }
    Ok(PathBuf::from_iter(stack))
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{NormalizeError, normalize_path};

    #[test]
    fn empty_path_returns_error() {
        let root = Path::new("");
        let origin_dir = Path::new("");
        let input = Path::new("");
        let result = normalize_path(root, origin_dir, input);
        assert!(matches!(result, Err(NormalizeError::EmptyInput)));
    }

    #[test]
    fn plain_filename_with_root_at_cwd_returns_filename() {
        let root = Path::new("");
        let origin_dir = Path::new("");
        let input = Path::new("main.rs");
        let result = normalize_path(root, origin_dir, input);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Path::new("main.rs"));
    }

    #[test]
    fn relative_path_from_origin_is_resolved() {
        let root = Path::new("");
        let origin_dir = Path::new("src");
        let input = Path::new("../main.rs");
        let result = normalize_path(root, origin_dir, input);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Path::new("main.rs"));
    }
}
