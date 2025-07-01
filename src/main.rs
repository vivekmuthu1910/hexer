use color_eyre::Result;
use crossterm::event::{self, Event, KeyEvent, KeyEventKind};
use file_picker::{FilePickerEvent, FilePickerState};
use ratatui::{DefaultTerminal, Frame, layout::Layout};
use std::{fs::File, num::NonZeroUsize};

#[cfg(debug_assertions)]
use tracing::{Level, info, instrument};
#[cfg(debug_assertions)]
use tracing_appender::non_blocking::WorkerGuard;

use viewer::{ViewerContainer, ViewerContainerEvent};

mod file_picker;
mod utils;
mod viewer;

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    #[cfg(debug_assertions)]
    let _guard = init_tracing()?;

    #[cfg(debug_assertions)]
    info!("Starting hexer");

    let terminal = ratatui::init();
    let result = App::new().run(terminal);
    ratatui::restore();

    #[cfg(debug_assertions)]
    info!("Exiting hexer");
    result
}

#[derive(Debug)]
enum Window {
    FilePicker(FilePickerState),
    HexViewer(ViewerContainer),
}

impl Default for Window {
    fn default() -> Self {
        Window::FilePicker(FilePickerState::default())
    }
}

/// The main application which holds the state and logic of the application.
#[derive(Debug, Default)]
pub struct App {
    /// Is the application running?
    window: Window,
    running: bool,
}

impl App {
    /// Construct a new instance of [`App`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Run the application's main loop.
    pub fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        self.running = true;
        while self.running {
            terminal.draw(|frame| self.render(frame))?;
            self.handle_crossterm_events()?;
        }
        Ok(())
    }

    #[cfg_attr(debug_assertions, instrument(skip_all, name = "App::render"))]
    fn render(&mut self, frame: &mut Frame) {
        Layout::init_cache(NonZeroUsize::new(1000).unwrap());
        #[cfg(debug_assertions)]
        info!("Rendering app");

        match self.window {
            Window::FilePicker(ref mut state) => {
                if let Err(err) = state.render_file_picker(frame) {
                    eprintln!("Error occured while selecting file {:?}", err);
                    self.running = false;
                }
            }
            Window::HexViewer(ref mut state) => {
                if let Err(err) = state.render_viewer(frame) {
                    eprintln!("Error occured while selecting file {:?}", err);
                    self.running = false;
                }
            }
        }
    }

    fn handle_crossterm_events(&mut self) -> Result<()> {
        match event::read()? {
            // it's important to check KeyEventKind::Press to avoid handling key release events
            Event::Key(key) if key.kind == KeyEventKind::Press => self.on_key_event(key),
            Event::Mouse(_) => {}
            Event::Resize(_, _) => {}
            _ => {}
        }
        Ok(())
    }

    /// Handles the key events and updates the state of [`App`].
    #[cfg_attr(debug_assertions, instrument(skip_all, name = "App::on_key_event"))]
    fn on_key_event(&mut self, key: KeyEvent) {
        match self.window {
            Window::FilePicker(ref mut state) => match state.handle_key(key) {
                FilePickerEvent::Quit => self.quit(),
                FilePickerEvent::Poll => {}
                FilePickerEvent::SelectedFile(f) => {
                    self.window = Window::HexViewer(ViewerContainer::default().with_file(f))
                }
            },
            Window::HexViewer(ref mut viewer_container) => match viewer_container.handle_key(key) {
                ViewerContainerEvent::Quit => self.quit(),
                ViewerContainerEvent::Poll => {}
                ViewerContainerEvent::SelectFile(f) => {
                    #[cfg(debug_assertions)]
                    info!("Changing back to file picker mode: {f:?}");
                    self.window = Window::FilePicker(FilePickerState::default().with_cwd(f));
                }
            },
        };
    }

    fn quit(&mut self) {
        self.running = false;
    }
}

#[cfg(debug_assertions)]
fn init_tracing() -> Result<WorkerGuard> {
    use tracing_appender::non_blocking;
    use tracing_subscriber::EnvFilter;
    let file = File::create("tracing.log")?;
    let (non_blocking, guard) = non_blocking(file);

    // By default, the subscriber is configured to log all events with a level of `DEBUG` or higher,
    // but this can be changed by setting the `RUST_LOG` environment variable.
    let env_filter = EnvFilter::builder()
        .with_default_directive(Level::DEBUG.into())
        .from_env_lossy();

    tracing_subscriber::fmt()
        .with_writer(non_blocking)
        .with_env_filter(env_filter)
        .init();
    Ok(guard)
}
