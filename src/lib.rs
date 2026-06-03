use std::{
    env,
    path::{Component, Path, PathBuf},
};

#[derive(Debug)]
pub enum NormalizeError {
    EmptyInput,
    EscapesFilesystemRoot,
    InvalidMultiplePrefix,
}

pub struct Normalizer {
    root: PathBuf,
    origin_base: PathBuf,
}

impl Normalizer {
    pub fn new(root: &Path, origin_base: &Path) -> std::io::Result<Self> {
        let cwd = env::current_dir()?;
        Ok(Self::new_with_cwd(root, origin_base, &cwd))
    }

    fn new_with_cwd(root: &Path, origin_base: &Path, cwd: &Path) -> Self {
        Self {
            root: absolutize(root, cwd),
            origin_base: absolutize(origin_base, cwd),
        }
    }

    pub fn normalize(&self, input: &Path) -> Result<PathBuf, NormalizeError> {
        if input.as_os_str().is_empty() {
            return Err(NormalizeError::EmptyInput);
        }
        let input = absolutize(input, &self.origin_base);
        let normalized_input = normalize_components(&input)?;
        Ok(make_relative_to_root(normalized_input, &self.root))
    }
}

fn absolutize(path: &Path, base: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base.join(path)
    }
}

/// # Invariant
/// `path` must be an absolute path.
/// Violations indicate a bug in the caller.
fn normalize_components(path: &Path) -> Result<PathBuf, NormalizeError> {
    debug_assert!(
        path.is_absolute(),
        "Input must be an absolute path: {:?}",
        path
    );
    let mut normalized = PathBuf::new();
    let mut iter = path.components().peekable();

    // Find the root, if any, and add it to the lexical path.
    // Here we treat the Windows path "C:\" as a single "root" even though
    // `components` splits it into two: (Prefix, RootDir).
    let root = match iter.peek() {
        Some(p @ Component::RootDir) => {
            normalized.push(p);
            iter.next();
            normalized.as_os_str().len()
        }
        Some(Component::Prefix(prefix)) => {
            normalized.push(prefix.as_os_str());
            iter.next();
            if let Some(p @ Component::RootDir) = iter.peek() {
                normalized.push(p);
                iter.next();
            }
            normalized.as_os_str().len()
        }
        _ => unreachable!(
            "normalize_components received a non-absolute path: {:?}",
            path
        ),
    };

    for component in iter {
        match component {
            Component::RootDir => unreachable!(),
            Component::Prefix(_) => return Err(NormalizeError::InvalidMultiplePrefix),
            Component::CurDir => continue,
            Component::ParentDir => {
                // It's an error if ParentDir causes us to go above the "root".
                if normalized.as_os_str().len() == root {
                    return Err(NormalizeError::EscapesFilesystemRoot);
                } else {
                    normalized.pop();
                }
            }
            Component::Normal(path) => normalized.push(path),
        }
    }
    Ok(normalized)
}

/// # Invariant
/// `target` and `root` must be an absolute paths.
/// Violations indicate a bug in the caller.
fn make_relative_to_root(target: PathBuf, mut root: &Path) -> PathBuf {
    debug_assert!(
        target.is_absolute(),
        "Target must be an absolute path: {:?}",
        target
    );
    debug_assert!(
        root.is_absolute(),
        "Root must be an absolute path: {:?}",
        root
    );
    let mut upward = PathBuf::new();
    loop {
        if let Ok(suffix) = target.strip_prefix(root) {
            return upward.join(suffix);
        }
        if let Some(new_root) = root.parent() {
            upward.push("..");
            root = new_root;
        } else {
            return target;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{NormalizeError, Normalizer};

    #[test]
    fn empty_path_returns_error() {
        let fake_cwd = Path::new("/sandbox");
        let normalizer = Normalizer::new_with_cwd(Path::new(""), Path::new(""), fake_cwd);
        let result = normalizer.normalize(Path::new(""));
        assert!(matches!(result, Err(NormalizeError::EmptyInput)));
    }

    #[test]
    fn plain_filename_with_root_at_cwd_returns_filename() {
        let fake_cwd = Path::new("/sandbox");
        let normalizer = Normalizer::new_with_cwd(Path::new(""), Path::new(""), fake_cwd);
        let result = normalizer.normalize(Path::new("main.rs"));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Path::new("main.rs"));
    }

    #[test]
    fn relative_path_from_origin_is_resolved() {
        let fake_cwd = Path::new("/sandbox");
        let root = Path::new("");
        let origin_base = Path::new("src");
        let normalizer = Normalizer::new_with_cwd(root, origin_base, fake_cwd);
        let result = normalizer.normalize(Path::new("../main.rs"));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Path::new("main.rs"));
    }

    #[test]
    fn path_is_made_relative_to_root() {
        let fake_cwd = Path::new("/sandbox");
        let root = Path::new("/project");
        let origin_base = Path::new("/project/src");
        let normalizer = Normalizer::new_with_cwd(root, origin_base, fake_cwd);
        let result = normalizer.normalize(Path::new("main.rs"));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Path::new("src/main.rs"));
    }

    #[test]
    fn path_is_made_relative_to_root_from_outside() {
        let fake_cwd = Path::new("/sandbox");
        let root = Path::new("/project");
        let origin_base = Path::new("/outside");
        let normalizer = Normalizer::new_with_cwd(root, origin_base, fake_cwd);
        let result = normalizer.normalize(Path::new("main.rs"));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Path::new("../outside/main.rs"));
    }

    #[test]
    fn path_is_made_relative_to_root_even_if_root_dir_is_relative() {
        let fake_cwd = Path::new("/sandbox");
        let root = Path::new("project");
        let origin_base = Path::new("/sandbox/outside");
        let normalizer = Normalizer::new_with_cwd(root, origin_base, fake_cwd);
        let result = normalizer.normalize(Path::new("main.rs"));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Path::new("../outside/main.rs"));
    }

    #[test]
    fn absolute_inputs_work() {
        let fake_cwd = Path::new("/sandbox");
        let root = Path::new("project");
        let origin_base = Path::new("/sandbox/outside");
        let normalizer = Normalizer::new_with_cwd(root, origin_base, fake_cwd);
        let result = normalizer.normalize(Path::new("/sandbox/main.rs"));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Path::new("../main.rs"));
    }

    #[test]
    fn path_is_made_relative_to_root_even_if_origin_base_is_relative() {
        let fake_cwd = Path::new("/sandbox");
        let root = Path::new("/sandbox/project");
        let origin_base = Path::new("outside");
        let normalizer = Normalizer::new_with_cwd(root, origin_base, fake_cwd);
        let result = normalizer.normalize(Path::new("main.rs"));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Path::new("../outside/main.rs"));
    }

    #[test]
    fn input_cannot_go_above_root() {
        let fake_cwd = Path::new("/sandbox");
        let root = Path::new("project");
        let origin_base = Path::new("outside");
        let normalizer = Normalizer::new_with_cwd(root, origin_base, fake_cwd);
        let result = normalizer.normalize(Path::new("../../../main.rs"));
        assert!(matches!(result, Err(NormalizeError::EscapesFilesystemRoot)));
    }
}
