use color_eyre::owo_colors::OwoColorize;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::{Color, Style, Stylize},
    text::Line,
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Padding, Paragraph},
};
use std::{env, fs, io::Result, path::PathBuf};

#[derive(Debug)]
pub enum FileType {
    File(String),
    Dir(String),
}

#[derive(Debug, Default)]
pub struct FilePickerState {
    state: ListState,
    cwd: PathBuf,
    files: Vec<FileType>,
    reload_dir: bool,
    cwd_selected: bool,
}

pub fn render_file_picker(frame: &mut Frame, picker_state: &mut FilePickerState) -> Result<String> {
    if !picker_state.cwd_selected {
        picker_state.reload_dir = true;
        picker_state.cwd = env::current_dir()?;
        picker_state.cwd_selected = true;
    }

    let layout =
        Layout::vertical([Constraint::Length(3), Constraint::Fill(1)]).areas::<2>(frame.area());

    let b = Block::default()
        .title(Line::from(" Directory ").centered())
        .border_style(Style::default().fg(Color::Cyan))
        .border_type(BorderType::Rounded)
        .borders(Borders::ALL);
    frame.render_widget(
        Paragraph::new(picker_state.cwd.to_str().unwrap()).block(b),
        layout[0],
    );

    if picker_state.reload_dir {
        picker_state.files = fs::read_dir(&picker_state.cwd)?
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
        picker_state.files.sort_by(|a, b| match (a, b) {
            (FileType::Dir(a_name), FileType::Dir(b_name)) => a_name.cmp(b_name),
            (FileType::File(a_name), FileType::File(b_name)) => a_name.cmp(b_name),
            (FileType::Dir(_), FileType::File(_)) => std::cmp::Ordering::Less,
            (FileType::File(_), FileType::Dir(_)) => std::cmp::Ordering::Greater,
        });
        picker_state.reload_dir = false;
        picker_state.state.select_first();
    }

    let list_view = List::new(picker_state.files.iter().map(|f| match f {
        FileType::Dir(fname) => ListItem::new(fname.as_str()).blue(),
        FileType::File(fname) => ListItem::new(fname.as_str()).green(),
    }))
    .block(
        Block::bordered()
            .border_style(Style::default().fg(Color::Magenta))
            .border_type(BorderType::Rounded)
            .title_bottom(" Files ")
            .padding(Padding::uniform(1)),
    )
    .highlight_style(Style::new().reversed())
    .highlight_symbol("  ")
    .repeat_highlight_symbol(true);
    if picker_state.state.selected().is_none() {
        picker_state.state.select_first();
    }
    frame.render_stateful_widget(list_view, layout[1], &mut picker_state.state);
    Ok(String::from(""))
}

pub fn handle_key(key: KeyEvent, picker_state: &mut FilePickerState) -> bool {
    match (key.modifiers, key.code) {
        (_, KeyCode::Esc | KeyCode::Char('q'))
        | (KeyModifiers::CONTROL, KeyCode::Char('c') | KeyCode::Char('C')) => return true,
        (_, KeyCode::Char('j') | KeyCode::Down) => picker_state.state.select_next(),
        (_, KeyCode::Char('k') | KeyCode::Up) => picker_state.state.select_previous(),
        (_, KeyCode::Char('h') | KeyCode::Left) => {
            if let Some(parent) = picker_state.cwd.parent() {
                picker_state.cwd = parent.to_path_buf();
                picker_state.reload_dir = true;
            }
        }
        (_, KeyCode::Enter) => {
            let selected = &picker_state.files[picker_state.state.selected().unwrap()];
            match selected {
                FileType::Dir(d) => {
                    picker_state.cwd = picker_state.cwd.join(d.as_str());
                    // eprintln!("picker_state.cwd: {:?}", picker_state.cwd);
                    picker_state.reload_dir = true;
                }
                FileType::File(_) => {}
            }
        }
        _ => {}
    }
    false
}
