use std::path::{Path, PathBuf};

use clap::{CommandFactory, Parser};
use color_eyre::eyre::{Result, eyre};

use crate::viewer::{DataType, DisplayType, Endianness, ViewMode};

/// Initial window chosen from an optional CLI path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LaunchWindow {
    FilePicker { cwd: PathBuf },
    Viewer { file: PathBuf },
}

/// Viewer options from CLI flags — seed session state; TUI can still change them.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ViewerLaunchOptions {
    pub data_type: DataType,
    pub display_type: DisplayType,
    pub endianness: Endianness,
    pub width: Option<usize>,
    pub stride: Option<usize>,
    pub view_mode: ViewMode,
    pub show_padding: bool,
}

/// Path resolution plus initial viewer options.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchConfig {
    pub window: LaunchWindow,
    pub options: ViewerLaunchOptions,
}

#[derive(Debug, Parser)]
#[command(
    name = "hexer",
    about = "View binary Buffers as a typed Grid",
    long_about = "Hexer visualizes a binary Buffer as a typed Grid.\n\n\
Stride is counted in Values (not bytes)."
)]
struct Cli {
    /// File or directory path (absent → File Picker at cwd; directory → File Picker there; file → viewer)
    path: Option<PathBuf>,

    /// Data Type (u8, i8, u16, i16, u32, i32, u64, i64, f32, f64)
    #[arg(long = "data-type", value_name = "TYPE")]
    data_type: Option<String>,

    /// Display Type for integers: decimal or hex
    #[arg(long = "display")]
    display: Option<String>,

    /// Endianness for multi-byte Values: little or big
    #[arg(long = "endianness", value_name = "ORDER")]
    endianness: Option<String>,

    /// Width in Values per row (pins Width; omit for Binary auto-fit)
    #[arg(long = "width", value_name = "N")]
    width: Option<usize>,

    /// Stride in Values from one row start to the next (default: Width). Counted in Values, not bytes.
    #[arg(long = "stride", value_name = "N")]
    stride: Option<usize>,

    /// View Mode: binary (default) or image
    #[arg(long = "view-mode", value_name = "MODE")]
    view_mode: Option<String>,

    /// Show Padding Values (Stride−Width) in the Grid
    #[arg(long = "show-padding", default_value_t = false)]
    show_padding: bool,
}

/// Parse process args into a LaunchConfig (path + viewer option flags).
/// Prefer [`parse_launch_config_from_env`] in `main` so `--help` / `--version` exit cleanly.
pub fn parse_launch_config<I, T>(args: I) -> Result<LaunchConfig>
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    let cli = Cli::try_parse_from(args).map_err(|e| eyre!("{e}"))?;
    launch_config_from_cli(cli)
}

/// Parse OS args for the binary entrypoint; clap handles `--help` / `--version` exits.
pub fn parse_launch_config_from_env() -> Result<LaunchConfig> {
    launch_config_from_cli(Cli::parse())
}

fn launch_config_from_cli(cli: Cli) -> Result<LaunchConfig> {
    let options = viewer_options_from_cli(&cli)?;
    let window = resolve_launch(cli.path.as_deref())?;
    Ok(LaunchConfig { window, options })
}

fn viewer_options_from_cli(cli: &Cli) -> Result<ViewerLaunchOptions> {
    let mut options = ViewerLaunchOptions::default();
    if let Some(ref s) = cli.data_type {
        options.data_type = parse_data_type(s)?;
    }
    if let Some(ref s) = cli.display {
        options.display_type = parse_display_type(s)?;
    }
    if let Some(ref s) = cli.endianness {
        options.endianness = parse_endianness(s)?;
    }
    if let Some(w) = cli.width {
        if w == 0 {
            return Err(eyre!("--width must be >= 1"));
        }
        options.width = Some(w);
    }
    if let Some(s) = cli.stride {
        if s == 0 {
            return Err(eyre!("--stride must be >= 1"));
        }
        options.stride = Some(s);
    }
    if let Some(ref s) = cli.view_mode {
        options.view_mode = parse_view_mode(s)?;
    }
    options.show_padding = cli.show_padding;

    if options.view_mode.requires_explicit_width() && options.width.is_none() {
        return Err(eyre!(
            "Image View Mode requires --width (explicit Width in Values)"
        ));
    }
    if let (Some(width), Some(stride)) = (options.width, options.stride) {
        if stride < width {
            return Err(eyre!("--stride must be >= --width"));
        }
    }
    Ok(options)
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

/// Long help text for `--help` (includes Stride unit note).
pub fn help_text() -> String {
    let mut cmd = Cli::command();
    let mut buf = Vec::new();
    cmd.write_long_help(&mut buf).expect("help write");
    String::from_utf8(buf).expect("help utf8")
}

fn parse_data_type(s: &str) -> Result<DataType> {
    match s.to_ascii_lowercase().as_str() {
        "u8" => Ok(DataType::U8),
        "i8" => Ok(DataType::I8),
        "u16" => Ok(DataType::U16),
        "i16" => Ok(DataType::I16),
        "u32" => Ok(DataType::U32),
        "i32" => Ok(DataType::I32),
        "u64" => Ok(DataType::U64),
        "i64" => Ok(DataType::I64),
        "f32" => Ok(DataType::F32),
        "f64" => Ok(DataType::F64),
        other => Err(eyre!(
            "unknown --data-type '{other}' (expected u8|i8|u16|i16|u32|i32|u64|i64|f32|f64)"
        )),
    }
}

fn parse_display_type(s: &str) -> Result<DisplayType> {
    match s.to_ascii_lowercase().as_str() {
        "decimal" | "dec" => Ok(DisplayType::Decimal),
        "hex" => Ok(DisplayType::HexaDecimal),
        other => Err(eyre!(
            "unknown --display '{other}' (expected decimal|hex)"
        )),
    }
}

fn parse_endianness(s: &str) -> Result<Endianness> {
    match s.to_ascii_lowercase().as_str() {
        "little" | "le" => Ok(Endianness::Little),
        "big" | "be" => Ok(Endianness::Big),
        other => Err(eyre!(
            "unknown --endianness '{other}' (expected little|big)"
        )),
    }
}

fn parse_view_mode(s: &str) -> Result<ViewMode> {
    match s.to_ascii_lowercase().as_str() {
        "binary" => Ok(ViewMode::Binary),
        "image" => Ok(ViewMode::Image),
        other => Err(eyre!(
            "unknown --view-mode '{other}' (expected binary|image)"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;
    use tempfile::tempdir;

    fn parse(args: &[&str]) -> Result<LaunchConfig> {
        let mut full = vec!["hexer"];
        full.extend(args);
        parse_launch_config(&full)
    }

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

    #[test]
    fn viewer_launch_options_default_to_binary_u8() {
        let opts = ViewerLaunchOptions::default();
        assert_eq!(opts.data_type, DataType::U8);
        assert_eq!(opts.display_type, DisplayType::Decimal);
        assert_eq!(opts.endianness, Endianness::Little);
        assert_eq!(opts.view_mode, ViewMode::Binary);
        assert_eq!(opts.width, None);
        assert_eq!(opts.stride, None);
        assert!(!opts.show_padding);
    }

    #[test]
    fn flags_populate_launch_options_with_file_path() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("img.bin");
        fs::write(&file, [0u8; 16]).unwrap();

        let cfg = parse(&[
            file.to_str().unwrap(),
            "--data-type",
            "u16",
            "--display",
            "hex",
            "--endianness",
            "big",
            "--width",
            "8",
            "--stride",
            "10",
            "--view-mode",
            "image",
            "--show-padding",
        ])
        .unwrap();

        assert_eq!(
            cfg.window,
            LaunchWindow::Viewer {
                file: file.clone()
            }
        );
        assert_eq!(cfg.options.data_type, DataType::U16);
        assert_eq!(cfg.options.display_type, DisplayType::HexaDecimal);
        assert_eq!(cfg.options.endianness, Endianness::Big);
        assert_eq!(cfg.options.width, Some(8));
        assert_eq!(cfg.options.stride, Some(10));
        assert_eq!(cfg.options.view_mode, ViewMode::Image);
        assert!(cfg.options.show_padding);
    }

    #[test]
    fn flags_apply_when_path_opens_file_picker() {
        let dir = tempdir().unwrap();
        let cfg = parse(&[
            dir.path().to_str().unwrap(),
            "--data-type",
            "f32",
            "--endianness",
            "little",
            "--width",
            "4",
        ])
        .unwrap();

        assert_eq!(
            cfg.window,
            LaunchWindow::FilePicker {
                cwd: dir.path().to_path_buf()
            }
        );
        assert_eq!(cfg.options.data_type, DataType::F32);
        assert_eq!(cfg.options.width, Some(4));
        assert_eq!(cfg.options.view_mode, ViewMode::Binary);
    }

    #[test]
    fn image_view_mode_without_width_errors() {
        let err = parse(&["--view-mode", "image"]).unwrap_err();
        assert!(err.to_string().contains("--width"));
    }

    #[test]
    fn flags_apply_with_absent_path_file_picker_at_cwd() {
        let cwd = env::current_dir().unwrap();
        let cfg = parse(&["--data-type", "i8", "--display", "hex", "--show-padding"]).unwrap();
        assert_eq!(cfg.window, LaunchWindow::FilePicker { cwd });
        assert_eq!(cfg.options.data_type, DataType::I8);
        assert_eq!(cfg.options.display_type, DisplayType::HexaDecimal);
        assert!(cfg.options.show_padding);
    }

    #[test]
    fn help_documents_flags_and_stride_in_values() {
        let help = help_text();
        assert!(help.contains("--data-type"));
        assert!(help.contains("--display"));
        assert!(help.contains("--endianness"));
        assert!(help.contains("--width"));
        assert!(help.contains("--stride"));
        assert!(help.contains("--view-mode"));
        assert!(help.contains("--show-padding"));
        assert!(help.contains("Values"));
        assert!(
            help.to_lowercase().contains("stride") && help.contains("Values"),
            "help should state Stride is in Values: {help}"
        );
    }
}
