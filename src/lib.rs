use std::path::{Component, Path, PathBuf};

#[derive(Debug)]
pub enum NormalizeError {
    EmptyInput,
}

pub fn normalize_path(root: &Path, origin: &Path, input: &Path) -> Result<PathBuf, NormalizeError> {
    if input.as_os_str().is_empty() {
        return Err(NormalizeError::EmptyInput);
    }
    let mut stack = Vec::new();
    let origin_joint_input = origin.join(input);
    for component in origin_joint_input.components() {
        match component {
            Component::CurDir => (),
            Component::ParentDir => {
                stack.pop();
            }
            Component::Prefix(_) => stack.push(component),
            Component::Normal(_) => stack.push(component),
            Component::RootDir => stack.push(component),
        }
    }
    let normalized_origin_join_input = PathBuf::from_iter(stack);
    Ok(normalize_to_root(&normalized_origin_join_input, root))
}

fn normalize_to_root(target_path: &Path, root: &Path,) -> PathBuf {
    match target_path.strip_prefix(root) {
        Ok(normalized_path) => normalized_path.to_path_buf(),
        Err(_) => {
            let root_parent = root.parent().expect("failed cuz target_path is not absolute");
            PathBuf::from("..").join(normalize_to_root(target_path, root_parent))
        },
    }
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

    #[test]
    fn path_is_made_relative_to_root() {
        let root = Path::new("/project");
        let origin_dir = Path::new("/project/src");
        let input = Path::new("main.rs");
        let result = normalize_path(root, origin_dir, input);
        assert_eq!(result.unwrap(), Path::new("src/main.rs"));
    }

    #[test]
    fn path_is_made_relative_to_root_from_outside() {
        let root = Path::new("/project");
        let origin_dir = Path::new("/outside");
        let input = Path::new("main.rs");
        let result = normalize_path(root, origin_dir, input);
        assert_eq!(result.unwrap(), Path::new("../outside/main.rs"));
    }
}
