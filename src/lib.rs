use std::{env, path::{Component, Path, PathBuf}};

#[derive(Debug)]
pub enum NormalizeError {
    EmptyInput,
    CwdNotAbsolute,
    InputOutsideFileSystemRoot,
}

pub struct Normalizer {
    root: PathBuf,
    origin: PathBuf,
}

impl Normalizer {
    pub fn new(root: &Path, origin: &Path,) -> std::io::Result<Self> {
        let cwd = env::current_dir()?;
        Ok(Self::new_with_cwd(root, origin, &cwd))
    }

    fn new_with_cwd(root: &Path, origin: &Path, cwd: &Path,) -> Self {
        Self { root: absolutize(root, cwd), origin: absolutize(origin, cwd) }
    }

    pub fn normalize_path(&self, input: &Path,) -> Result<PathBuf, NormalizeError> {
        if input.as_os_str().is_empty() {
            return Err(NormalizeError::EmptyInput);
        }
        let input = absolutize(input, &self.origin);
        let normalized_input = normalize_lexically(&input)?;
        Ok(normalize_to_root(normalized_input, &self.root))
    }
}

fn absolutize(path: &Path, absolute_prefix: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        absolute_prefix.join(path)
    }
}

fn normalize_lexically(path: &Path) -> Result<PathBuf, NormalizeError> {
    let mut lexical = PathBuf::new();
    let mut iter = path.components().peekable();

    // Find the root, if any, and add it to the lexical path.
    // Here we treat the Windows path "C:\" as a single "root" even though
    // `components` splits it into two: (Prefix, RootDir).
    let root = match iter.peek() {
        Some(Component::ParentDir) => return Err(NormalizeError::InputOutsideFileSystemRoot),
        Some(p @ Component::RootDir) | Some(p @ Component::CurDir) => {
            lexical.push(p);
            iter.next();
            lexical.as_os_str().len()
        }
        Some(Component::Prefix(prefix)) => {
            lexical.push(prefix.as_os_str());
            iter.next();
            if let Some(p @ Component::RootDir) = iter.peek() {
                lexical.push(p);
                iter.next();
            }
            lexical.as_os_str().len()
        }
        None => return Ok(PathBuf::new()),
        Some(Component::Normal(_)) => 0,
    };

    for component in iter {
        match component {
            Component::RootDir => unreachable!(),
            Component::Prefix(_) => return Err(NormalizeError::InputOutsideFileSystemRoot),
            Component::CurDir => continue,
            Component::ParentDir => {
                // It's an error if ParentDir causes us to go above the "root".
                if lexical.as_os_str().len() == root {
                    return Err(NormalizeError::InputOutsideFileSystemRoot);
                } else {
                    lexical.pop();
                }
            }
            Component::Normal(path) => lexical.push(path),
        }
    }
    Ok(lexical)
}

fn normalize_to_root(target: PathBuf, mut root: &Path) -> PathBuf {
    let mut prefix = PathBuf::new();
    loop {
        if let Ok(suffix) = target.strip_prefix(root) {
            return prefix.join(suffix);
        }
        if let Some(new_root) = root.parent() {
            prefix.push("..");
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
        let result = normalizer.normalize_path(Path::new(""));
        assert!(matches!(result, Err(NormalizeError::EmptyInput)));
    }

    #[test]
    fn plain_filename_with_root_at_cwd_returns_filename() {
        let fake_cwd = Path::new("/sandbox");
        let normalizer = Normalizer::new_with_cwd(Path::new(""), Path::new(""), fake_cwd);
        let result = normalizer.normalize_path(Path::new("main.rs"));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Path::new("main.rs"));
    }

    #[test]
    fn relative_path_from_origin_is_resolved() {
        let fake_cwd = Path::new("/sandbox");
        let root = Path::new("");
        let origin_dir = Path::new("src");
        let normalizer = Normalizer::new_with_cwd(root, origin_dir, fake_cwd);
        let result = normalizer.normalize_path(Path::new("../main.rs"));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Path::new("main.rs"));
    }

    #[test]
    fn path_is_made_relative_to_root() {
        let fake_cwd = Path::new("/sandbox");
        let root = Path::new("/project");
        let origin_dir = Path::new("/project/src");
        let normalizer = Normalizer::new_with_cwd(root, origin_dir, fake_cwd);
        let result = normalizer.normalize_path(Path::new("main.rs"));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Path::new("src/main.rs"));
    }

    #[test]
    fn path_is_made_relative_to_root_from_outside() {
        let fake_cwd = Path::new("/sandbox");
        let root = Path::new("/project");
        let origin_dir = Path::new("/outside");
        let normalizer = Normalizer::new_with_cwd(root, origin_dir, fake_cwd);
        let result = normalizer.normalize_path(Path::new("main.rs"));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Path::new("../outside/main.rs"));
    }

    #[test]
    fn path_is_made_relative_to_root_even_if_root_dir_is_relative() {
        let fake_cwd = Path::new("/sandbox");
        let root = Path::new("project");
        let origin_dir = Path::new("/sandbox/outside");
        let normalizer = Normalizer::new_with_cwd(root, origin_dir, fake_cwd);
        let result = normalizer.normalize_path(Path::new("main.rs"));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Path::new("../outside/main.rs"));
    }

    #[test]
    fn absolute_inputs_work() {
        let fake_cwd = Path::new("/sandbox");
        let root = Path::new("project");
        let origin_dir = Path::new("/sandbox/outside");
        let normalizer = Normalizer::new_with_cwd(root, origin_dir, fake_cwd);
        let result = normalizer.normalize_path(Path::new("/sandbox/main.rs"));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Path::new("../main.rs"));
    }

    #[test]
    fn path_is_made_relative_to_root_even_if_origin_dir_is_relative() {
        let fake_cwd = Path::new("/sandbox");
        let root = Path::new("/sandbox/project");
        let origin_dir = Path::new("outside");
        let normalizer = Normalizer::new_with_cwd(root, origin_dir, fake_cwd);
        let result = normalizer.normalize_path(Path::new("main.rs"));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Path::new("../outside/main.rs"));
    }

    #[test]
    fn input_cannot_go_above_root() {
        let fake_cwd = Path::new("/sandbox");
        let root = Path::new("project");
        let origin_dir = Path::new("outside");
        let normalizer = Normalizer::new_with_cwd(root, origin_dir, fake_cwd);
        let result = normalizer.normalize_path(Path::new("../../../main.rs"));
        assert!(matches!(result, Err(NormalizeError::InputOutsideFileSystemRoot)));
    }
}
