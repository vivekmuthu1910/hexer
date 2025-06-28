use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    style::{Color, Style, Stylize},
    text::Line,
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Padding},
};
use std::{env, fs, io::Result, path::PathBuf};

#[derive(Debug)]
pub enum FileType {
    File(String),
    Dir(String),
}

#[derive(Debug, Default)]
pub struct FilePickerState {
    list_state: ListState,
    cwd: PathBuf,
    files: Vec<FileType>,
    reload_dir: bool,
    cwd_selected: bool,
}

pub enum FilePickerEvent {
    Quit,
    Poll,
    SelectedFile(PathBuf),
}

impl FilePickerState {
    pub fn render_file_picker(&mut self, frame: &mut Frame) -> Result<()> {
        if !self.cwd_selected {
            self.reload_dir = true;
            self.cwd = env::current_dir()?;
            self.cwd_selected = true;
        }

        let b = Block::default()
            .title(
                Line::from(format!(" {} ", self.cwd.to_str().unwrap()))
                    .bold()
                    .blue()
                    .centered(),
            )
            .border_style(Style::default().fg(Color::Cyan))
            .border_type(BorderType::Rounded)
            .borders(Borders::ALL);

        if self.reload_dir {
            self.files = fs::read_dir(&self.cwd)?
                .filter_map(Result::ok)
                .filter_map(|f| {
                    let ft = f.file_type().ok()?;
                    let file_name = f.file_name().into_string().ok()?;

                    if ft.is_dir() {
                        Some(FileType::Dir(file_name))
                    } else if ft.is_file() {
                        Some(FileType::File(file_name))
                    } else {
                        None
                    }
                })
                .collect::<Vec<FileType>>();
            self.files.sort_by(|a, b| match (a, b) {
                (FileType::Dir(a_name), FileType::Dir(b_name)) => a_name.cmp(b_name),
                (FileType::File(a_name), FileType::File(b_name)) => a_name.cmp(b_name),
                (FileType::Dir(_), FileType::File(_)) => std::cmp::Ordering::Less,
                (FileType::File(_), FileType::Dir(_)) => std::cmp::Ordering::Greater,
            });
            self.reload_dir = false;
            self.list_state.select_first();
        }

        let list_view = List::new(self.files.iter().map(|f| match f {
            FileType::Dir(fname) => ListItem::new(fname.as_str()).blue(),
            FileType::File(fname) => ListItem::new(fname.as_str()).green(),
        }))
        .block(b.title_bottom(" Files ").padding(Padding::uniform(1)))
        .highlight_style(Style::new().reversed())
        .highlight_symbol("  ")
        .repeat_highlight_symbol(true);
        if self.list_state.selected().is_none() {
            self.list_state.select_first();
        }
        frame.render_stateful_widget(list_view, frame.area(), &mut self.list_state);
        Ok(())
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> FilePickerEvent {
        match (key.modifiers, key.code) {
            (_, KeyCode::Esc | KeyCode::Char('q'))
            | (KeyModifiers::CONTROL, KeyCode::Char('c') | KeyCode::Char('C')) => {
                return FilePickerEvent::Quit;
            }
            (_, KeyCode::Char('j') | KeyCode::Down) => self.list_state.select_next(),
            (_, KeyCode::Char('k') | KeyCode::Up) => self.list_state.select_previous(),
            (_, KeyCode::Char('h') | KeyCode::Left) => {
                if let Some(parent) = self.cwd.parent() {
                    self.cwd = parent.to_path_buf();
                    self.reload_dir = true;
                }
            }
            (_, KeyCode::Enter) => {
                let selected = &mut self.files[self.list_state.selected().unwrap()];
                match selected {
                    FileType::Dir(d) => {
                        self.cwd = self.cwd.join(d.as_str());
                        self.reload_dir = true;
                    }
                    FileType::File(f) => {
                        let result = FilePickerEvent::SelectedFile(self.cwd.join(f));
                        self.files.clear();
                        return result;
                    }
                }
            }
            _ => {}
        }
        FilePickerEvent::Poll
    }
}
