use color_eyre::Result;
use crossterm::event::{self, Event, KeyEvent, KeyEventKind};
use file_picker::{FilePickerEvent, FilePickerState};
use ratatui::{DefaultTerminal, Frame};
use viewer::{ViewerEvent, ViewerState};

mod file_picker;
mod viewer;

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let terminal = ratatui::init();
    let result = App::new().run(terminal);
    ratatui::restore();
    result
}

#[derive(Debug)]
enum Window {
    FilePicker(FilePickerState),
    HexViewer(ViewerState),
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

    fn render(&mut self, frame: &mut Frame) {
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
    fn on_key_event(&mut self, key: KeyEvent) {
        match self.window {
            Window::FilePicker(ref mut state) => match state.handle_key(key) {
                FilePickerEvent::Quit => self.quit(),
                FilePickerEvent::Poll => {}
                FilePickerEvent::SelectedFile(f) => {
                    self.window = Window::HexViewer(ViewerState::default().with_file(f))
                }
            },
            Window::HexViewer(ref mut state) => match state.handle_key(key) {
                ViewerEvent::Quit => self.quit(),
                ViewerEvent::Poll => {}
            },
        };
    }

    fn quit(&mut self) {
        self.running = false;
    }
}
