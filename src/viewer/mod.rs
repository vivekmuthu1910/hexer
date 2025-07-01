use super::utils::last_n_components;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use file_viewer::{FileViewer, FileViewerState};
use ratatui::{
    Frame,
    layout::{Constraint, Flex, Layout, Margin, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, Widget},
};
use std::{fs, io::Result, path::PathBuf};
#[cfg(debug_assertions)]
use tracing::{info, instrument};

mod common_dt;
mod file_viewer;

use common_dt::{DataType, DisplayType, Endianness};

#[derive(Debug, Default)]
pub struct ViewerContainer {
    file: PathBuf,
    action_mode: ActionMode,
    file_viewer: FileViewer,
    file_viewer_state: FileViewerState,
    data_type: DataType,
    display_type: DisplayType,
    endianness: Endianness,
    // search_field: String,
}

pub enum ViewerContainerEvent {
    Quit,
    Poll,
    SelectFile(PathBuf),
}

#[derive(Debug, Default)]
pub enum ActionMode {
    #[default]
    Normal,
    SelectDataType(Option<KeyCode>),
    // EditSearch,
}

fn render_button(name: String, btn_color: Color, text_color: Color) -> impl Widget {
    Paragraph::new(name).fg(text_color).bg(btn_color).centered()
}

impl ViewerContainer {
    pub fn with_file(mut self, file: PathBuf) -> Self {
        self.file = file;
        self
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> ViewerContainerEvent {
        match self.action_mode {
            ActionMode::Normal => self.handle_normal_keys(key),
            ActionMode::SelectDataType(_) => self.handle_dt_keys(key),
        }
    }

    fn handle_normal_keys(&mut self, key: KeyEvent) -> ViewerContainerEvent {
        match (key.modifiers, key.code) {
            (_, KeyCode::Esc | KeyCode::Char('q'))
            | (KeyModifiers::CONTROL, KeyCode::Char('c') | KeyCode::Char('C')) => {
                return ViewerContainerEvent::Quit;
            }
            (_, KeyCode::Char('d')) => {
                self.display_type = DisplayType::Decimal;
                self.file_viewer.set_display_type(DisplayType::Decimal);
            }
            (_, KeyCode::Char('x')) => {
                self.display_type = DisplayType::HexaDecimal;
                self.file_viewer.set_display_type(DisplayType::HexaDecimal);
            }
            (KeyModifiers::SHIFT, KeyCode::Char('l')) => {
                self.endianness = Endianness::Little;
                self.file_viewer.set_endianness(Endianness::Little);
            }
            (KeyModifiers::SHIFT, KeyCode::Char('b')) => {
                self.endianness = Endianness::Big;
                self.file_viewer.set_endianness(Endianness::Big);
            }
            (KeyModifiers::CONTROL, KeyCode::Char('t')) => {
                self.action_mode = ActionMode::SelectDataType(None)
            }
            (KeyModifiers::CONTROL, KeyCode::Char('f')) => {
                return ViewerContainerEvent::SelectFile(self.file.parent().unwrap().to_owned());
            }
            (_, KeyCode::Char('j') | KeyCode::Down) => self.file_viewer_state.move_down(),
            (_, KeyCode::Char('k') | KeyCode::Up) => self.file_viewer_state.move_up(),
            (_, KeyCode::Char('h') | KeyCode::Left) => self.file_viewer_state.move_left(),
            (_, KeyCode::Char('l') | KeyCode::Right) => self.file_viewer_state.move_right(),
            (KeyModifiers::CONTROL, KeyCode::Home) => self.file_viewer_state.goto_top(),
            (KeyModifiers::CONTROL, KeyCode::End) => self.file_viewer_state.goto_bottom(),
            (_, KeyCode::Home) => self.file_viewer_state.goto_start(),
            (_, KeyCode::End) => self.file_viewer_state.goto_end(),
            (_, KeyCode::PageUp) => self.file_viewer_state.scroll_up(),
            (_, KeyCode::PageDown) => self.file_viewer_state.scroll_down(),
            _ => {}
        }
        ViewerContainerEvent::Poll
    }

    fn handle_dt_keys(&mut self, key: KeyEvent) -> ViewerContainerEvent {
        use KeyCode::Char;
        if let ActionMode::SelectDataType(Some(x)) = self.action_mode {
            match (x, key.code) {
                (Char('u'), Char('1')) => {
                    self.data_type = DataType::U8;
                    self.file_viewer.set_data_type(DataType::U8);
                }
                (Char('i'), Char('1')) => {
                    self.data_type = DataType::I8;
                    self.file_viewer.set_data_type(DataType::I8);
                }
                (Char('u'), Char('2')) => {
                    self.data_type = DataType::U16;
                    self.file_viewer.set_data_type(DataType::U16);
                }
                (Char('i'), Char('2')) => {
                    self.data_type = DataType::I16;
                    self.file_viewer.set_data_type(DataType::I16);
                }
                (Char('u'), Char('3')) => {
                    self.data_type = DataType::U32;
                    self.file_viewer.set_data_type(DataType::U32);
                }
                (Char('i'), Char('3')) => {
                    self.data_type = DataType::I32;
                    self.file_viewer.set_data_type(DataType::I32);
                }
                (Char('u'), Char('4')) => {
                    self.data_type = DataType::U64;
                    self.file_viewer.set_data_type(DataType::U64);
                }
                (Char('i'), Char('4')) => {
                    self.data_type = DataType::I64;
                    self.file_viewer.set_data_type(DataType::I64);
                }
                (Char('f'), Char('1')) => {
                    self.data_type = DataType::F32;
                    self.file_viewer.set_data_type(DataType::F32);
                }
                (Char('f'), Char('2')) => {
                    self.data_type = DataType::F64;
                    self.file_viewer.set_data_type(DataType::F64);
                }
                _ => self.action_mode = ActionMode::Normal,
            }
            self.action_mode = ActionMode::Normal;
        } else {
            match key.code {
                Char('u') | Char('i') | Char('f') => {
                    self.action_mode = ActionMode::SelectDataType(Some(key.code))
                }
                _ => self.action_mode = ActionMode::Normal,
            }
        }
        ViewerContainerEvent::Poll
    }

    #[cfg_attr(debug_assertions, instrument(skip_all, name = "Viewer::render_viewer"))]
    pub fn render_viewer(&mut self, frame: &mut Frame) -> Result<()> {
        let page_layout = Layout::vertical([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Fill(1),
        ])
        .areas::<3>(frame.area());

        let layout = Layout::horizontal([Constraint::Length(70), Constraint::Fill(1)])
            .areas::<2>(page_layout[0]);
        self.render_file_name(layout[0], frame);
        self.render_search_bar(layout[1], frame);

        let layout = Layout::horizontal([
            Constraint::Length(88),
            Constraint::Length(32),
            Constraint::Length(23),
        ])
        .flex(Flex::SpaceAround)
        .areas::<3>(page_layout[1]);

        self.render_dt_buttons(layout[0], frame);
        self.render_display_buttons(layout[1], frame);
        self.render_endianness_buttons(layout[2], frame);

        let content = fs::read(&self.file)?;

        #[cfg(debug_assertions)]
        info!("Content len: {}", content.len());

        self.file_viewer.set_content(content);
        frame.render_stateful_widget(
            &self.file_viewer,
            page_layout[2],
            &mut self.file_viewer_state,
        );
        Ok(())
    }

    fn render_file_name(&mut self, rect: Rect, frame: &mut Frame) {
        let b = Block::default()
            .border_style(Style::default().fg(Color::Cyan))
            .border_type(BorderType::Rounded)
            .borders(Borders::ALL)
            .title(Line::from(" File "));

        let fg = Color::LightYellow;
        let bg = Color::Blue;
        frame.render_widget(b, rect);
        let (file_comp_n, file_name_3comp) = last_n_components(&self.file, 3);
        let mut file_name = Line::from(Span::styled(" ", Style::default().bg(bg)));
        if file_comp_n > 3 {
            file_name.push_span(Span::styled(
                ".../",
                Style::default().fg(fg).bg(bg).bold().italic(),
            ));
        }

        file_name.extend(vec![
            Span::styled(
                file_name_3comp.to_str().unwrap(),
                Style::default().fg(fg).bg(bg).bold().italic(),
            ),
            Span::styled(" ", Style::default().bg(bg)),
        ]);
        frame.render_widget(
            file_name,
            rect.inner(Margin {
                horizontal: 2,
                vertical: 1,
            }),
        );
    }

    fn render_search_bar(&mut self, rect: Rect, frame: &mut Frame) {
        let b = Block::default()
            .title(" Search ")
            .border_style(Style::default().fg(Color::Rgb(70, 70, 70)))
            .border_type(BorderType::Rounded)
            .borders(Borders::ALL);
        frame.render_widget(b, rect);
    }

    fn render_display_buttons(&self, rect: Rect, frame: &mut Frame) {
        let b = Block::default()
            .border_style(Style::default().fg(Color::Cyan))
            .border_type(BorderType::Rounded)
            .borders(Borders::ALL)
            .title(Line::from(" Display Type "));
        frame.render_widget(b, rect);

        use Constraint::Length;
        let btn_layout = Layout::horizontal([Length(11), Length(15)])
            .flex(Flex::SpaceBetween)
            .vertical_margin(1)
            .horizontal_margin(2)
            .split(rect);
        let (btn1, btn2) = match self.display_type {
            DisplayType::Decimal => (
                render_button("Decimal".to_string(), Color::Green, Color::Black),
                render_button("HexaDecimal".to_string(), Color::Yellow, Color::Black),
            ),
            DisplayType::HexaDecimal => (
                render_button("Decimal".to_string(), Color::Yellow, Color::Black),
                render_button("HexaDecimal".to_string(), Color::Green, Color::Black),
            ),
        };
        frame.render_widget(btn1, btn_layout[0]);
        frame.render_widget(btn2, btn_layout[1]);
    }

    fn render_endianness_buttons(&self, rect: Rect, frame: &mut Frame) {
        let b = Block::default()
            .border_style(Style::default().fg(Color::Cyan))
            .border_type(BorderType::Rounded)
            .borders(Borders::ALL)
            .title(Line::from(" Endianness "));
        frame.render_widget(b, rect);

        use Constraint::Length;
        let btn_layout = Layout::horizontal([Length(10), Length(7)])
            .flex(Flex::SpaceBetween)
            .vertical_margin(1)
            .horizontal_margin(2)
            .split(rect);
        let (btn1, btn2) = match self.endianness {
            Endianness::Little => (
                render_button("Little".to_string(), Color::Green, Color::Black),
                render_button("Big".to_string(), Color::Yellow, Color::Black),
            ),
            Endianness::Big => (
                render_button("Little".to_string(), Color::Yellow, Color::Black),
                render_button("Big".to_string(), Color::Green, Color::Black),
            ),
        };
        frame.render_widget(btn1, btn_layout[0]);
        frame.render_widget(btn2, btn_layout[1]);
    }

    fn render_dt_buttons(&self, rect: Rect, frame: &mut Frame) {
        let b = Block::default()
            .border_style(Style::default().fg(Color::Cyan))
            .border_type(BorderType::Rounded)
            .borders(Borders::ALL)
            .title(Line::from(" Data Type "));
        frame.render_widget(b, rect);

        use Constraint::Length;
        let btn_layout = Layout::horizontal([
            Length(6),
            Length(6),
            Length(7),
            Length(7),
            Length(7),
            Length(7),
            Length(7),
            Length(7),
            Length(7),
            Length(7),
        ])
        .flex(Flex::SpaceBetween)
        .vertical_margin(1)
        .horizontal_margin(2)
        .split(rect);

        for (i, val) in DataType::ALL.iter().enumerate() {
            let btn = if &self.data_type == val {
                render_button(val.to_string(), Color::Green, Color::Black)
            } else {
                render_button(val.to_string(), Color::Yellow, Color::Black)
            };
            frame.render_widget(btn, btn_layout[i]);
        }
    }
}
