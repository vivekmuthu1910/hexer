use std::path::{Path, PathBuf};

/// Initial window chosen from an optional CLI path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LaunchWindow {
    FilePicker { cwd: PathBuf },
    Viewer { file: PathBuf },
}

/// Map an optional path argument to the initial launch window.
///
/// - `None` → File Picker at the current working directory
/// - directory → File Picker rooted at that directory
/// - file → viewer on that File
pub fn resolve_launch(path: Option<&Path>) -> std::io::Result<LaunchWindow> {
    match path {
        None => Ok(LaunchWindow::FilePicker {
            cwd: std::env::current_dir()?,
        }),
        Some(path) => {
            let meta = std::fs::metadata(path)?;
            if meta.is_dir() {
                Ok(LaunchWindow::FilePicker {
                    cwd: path.to_path_buf(),
                })
            } else if meta.is_file() {
                Ok(LaunchWindow::Viewer {
                    file: path.to_path_buf(),
                })
            } else {
                Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("{} is neither a file nor a directory", path.display()),
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn no_path_opens_file_picker_at_cwd() {
        let cwd = env::current_dir().unwrap();
        let launch = resolve_launch(None).unwrap();
        assert_eq!(launch, LaunchWindow::FilePicker { cwd });
    }

    #[test]
    fn directory_path_opens_file_picker_there() {
        let dir = tempdir().unwrap();
        let launch = resolve_launch(Some(dir.path())).unwrap();
        assert_eq!(
            launch,
            LaunchWindow::FilePicker {
                cwd: dir.path().to_path_buf()
            }
        );
    }

    #[test]
    fn file_path_opens_viewer_on_that_file() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("sample.bin");
        fs::write(&file, [0u8, 1, 2, 3]).unwrap();

        let launch = resolve_launch(Some(&file)).unwrap();
        assert_eq!(launch, LaunchWindow::Viewer { file });
    }

    #[test]
    fn missing_path_returns_error() {
        let dir = tempdir().unwrap();
        let missing = dir.path().join("does-not-exist.bin");
        let err = resolve_launch(Some(&missing)).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
    }
}
