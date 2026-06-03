use std::{env, path::{Component, Path, PathBuf}};

#[derive(Debug)]
pub enum NormalizeError {
    EmptyInput,
    CwdNotAbsolute,
    InputOutsideFileSystemRoot,
}

pub fn normalize_path(root: &Path, origin: &Path, input: &Path) -> Result<PathBuf, NormalizeError> {
    let cwd = env::current_dir().map_err(|_| NormalizeError::CwdNotAbsolute)?;
    normalize_path_with_preset_cwd(root, origin, input, &cwd)
}

fn normalize_path_with_preset_cwd(
    root: &Path,
    origin: &Path,
    input: &Path,
    cwd: &Path,
) -> Result<PathBuf, NormalizeError> {
    if input.as_os_str().is_empty() {
        return Err(NormalizeError::EmptyInput);
    }
    if !cwd.is_absolute() {
        return Err(NormalizeError::CwdNotAbsolute);
    }
    let input = if input.is_absolute() {
        input.to_path_buf()
    } else if origin.is_absolute() {
        origin.join(input)
    } else {
        cwd.join(origin).join(input)
    };
    let root = if root.is_absolute() {
        root.to_path_buf()
    } else {
        cwd.join(root)
    };
    let normalized_input = normalize_lexically(&input)?;
    Ok(normalize_to_root(normalized_input, &root))
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

    use super::{NormalizeError, normalize_path_with_preset_cwd};

    #[test]
    fn empty_path_returns_error() {
        let fake_cwd = Path::new("/sandbox");
        let root = Path::new("");
        let origin_dir = Path::new("");
        let input = Path::new("");
        let result = normalize_path_with_preset_cwd(root, origin_dir, input, fake_cwd);
        assert!(matches!(result, Err(NormalizeError::EmptyInput)));
    }

    #[test]
    fn plain_filename_with_root_at_cwd_returns_filename() {
        let fake_cwd = Path::new("/sandbox");
        let root = Path::new("");
        let origin_dir = Path::new("");
        let input = Path::new("main.rs");
        let result = normalize_path_with_preset_cwd(root, origin_dir, input, fake_cwd);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Path::new("main.rs"));
    }

    #[test]
    fn relative_path_from_origin_is_resolved() {
        let fake_cwd = Path::new("/sandbox");
        let root = Path::new("");
        let origin_dir = Path::new("src");
        let input = Path::new("../main.rs");
        let result = normalize_path_with_preset_cwd(root, origin_dir, input, fake_cwd);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Path::new("main.rs"));
    }

    #[test]
    fn path_is_made_relative_to_root() {
        let fake_cwd = Path::new("/sandbox");
        let root = Path::new("/project");
        let origin_dir = Path::new("/project/src");
        let input = Path::new("main.rs");
        let result = normalize_path_with_preset_cwd(root, origin_dir, input, fake_cwd);
        assert_eq!(result.unwrap(), Path::new("src/main.rs"));
    }

    #[test]
    fn path_is_made_relative_to_root_from_outside() {
        let fake_cwd = Path::new("/sandbox");
        let root = Path::new("/project");
        let origin_dir = Path::new("/outside");
        let input = Path::new("main.rs");
        let result = normalize_path_with_preset_cwd(root, origin_dir, input, fake_cwd);
        assert_eq!(result.unwrap(), Path::new("../outside/main.rs"));
    }

    #[test]
    fn path_is_made_relative_to_root_even_if_root_dir_is_relative() {
        let fake_cwd = Path::new("/sandbox");
        let root = Path::new("project");
        let origin_dir = Path::new("/sandbox/outside");
        let input = Path::new("main.rs");
        let result = normalize_path_with_preset_cwd(root, origin_dir, input, fake_cwd);
        assert_eq!(result.unwrap(), Path::new("../outside/main.rs"));
    }

    #[test]
    fn absolute_inputs_work() {
        let fake_cwd = Path::new("/sandbox");
        let root = Path::new("project");
        let origin_dir = Path::new("/sandbox/outside");
        let input = Path::new("/sandbox/main.rs");
        let result = normalize_path_with_preset_cwd(root, origin_dir, input, fake_cwd);
        assert_eq!(result.unwrap(), Path::new("../main.rs"));
    }

    #[test]
    fn path_is_made_relative_to_root_even_if_origin_dir_is_relative() {
        let fake_cwd = Path::new("/sandbox");
        let root = Path::new("/sandbox/project");
        let origin_dir = Path::new("outside");
        let input = Path::new("main.rs");
        let result = normalize_path_with_preset_cwd(root, origin_dir, input, fake_cwd);
        assert_eq!(result.unwrap(), Path::new("../outside/main.rs"));
    }

    #[test]
    fn input_cannot_go_above_root() {
        let fake_cwd = Path::new("/sandbox");
        let root = Path::new("project");
        let origin_dir = Path::new("outside");
        let input = Path::new("../../../main.rs");
        let result = normalize_path_with_preset_cwd(root, origin_dir, input, fake_cwd);
        assert!(matches!(result, Err(NormalizeError::InputOutsideFileSystemRoot)));
    }
}
