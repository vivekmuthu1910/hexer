use super::utils::last_n_components;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use file_viewer::FileViewer;
use ratatui::{
    Frame,
    layout::{Constraint, Flex, Layout, Margin, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, Widget},
};
use std::{fs, io::Result, path::PathBuf};

mod common_dt;
mod file_viewer;

use common_dt::{DataType, DisplayType, Endianness};

#[derive(Debug, Default)]
pub struct ViewerState {
    file: PathBuf,
    action_mode: ActionMode,
    file_viewer: FileViewer,
    // search_field: String,
}

pub enum ViewerEvent {
    Quit,
    Poll,
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

impl ViewerState {
    pub fn with_file(mut self, file: PathBuf) -> Self {
        self.file = file;
        self
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> ViewerEvent {
        match self.action_mode {
            ActionMode::Normal => self.handle_normal_keys(key),
            ActionMode::SelectDataType(_) => self.handle_dt_keys(key),
        }
    }

    fn handle_normal_keys(&mut self, key: KeyEvent) -> ViewerEvent {
        match (key.modifiers, key.code) {
            (_, KeyCode::Esc | KeyCode::Char('q'))
            | (KeyModifiers::CONTROL, KeyCode::Char('c') | KeyCode::Char('C')) => {
                return ViewerEvent::Quit;
            }
            (_, KeyCode::Char('d')) => self.file_viewer.display_type = DisplayType::Decimal,
            (_, KeyCode::Char('x')) => self.file_viewer.display_type = DisplayType::HexaDecimal,
            (_, KeyCode::Char('l')) => self.file_viewer.endianness = Endianness::Little,
            (_, KeyCode::Char('b')) => self.file_viewer.endianness = Endianness::Big,
            (KeyModifiers::CONTROL, KeyCode::Char('t')) => {
                self.action_mode = ActionMode::SelectDataType(None)
            }
            _ => {}
        }
        ViewerEvent::Poll
    }

    fn handle_dt_keys(&mut self, key: KeyEvent) -> ViewerEvent {
        use KeyCode::Char;
        if let ActionMode::SelectDataType(Some(x)) = self.action_mode {
            match (x, key.code) {
                (Char('u'), Char('1')) => self.file_viewer.data_type = DataType::U8,
                (Char('i'), Char('1')) => self.file_viewer.data_type = DataType::I8,
                (Char('u'), Char('2')) => self.file_viewer.data_type = DataType::U16,
                (Char('i'), Char('2')) => self.file_viewer.data_type = DataType::I16,
                (Char('u'), Char('3')) => self.file_viewer.data_type = DataType::U32,
                (Char('i'), Char('3')) => self.file_viewer.data_type = DataType::I32,
                (Char('u'), Char('4')) => self.file_viewer.data_type = DataType::U64,
                (Char('i'), Char('4')) => self.file_viewer.data_type = DataType::I64,
                (Char('f'), Char('1')) => self.file_viewer.data_type = DataType::F32,
                (Char('f'), Char('2')) => self.file_viewer.data_type = DataType::F64,
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
        ViewerEvent::Poll
    }

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

        frame.render_widget(&self.file_viewer, page_layout[2]);

        let content = fs::read(&self.file)?;
        self.file_viewer.set_content(content);
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
        let (btn1, btn2) = match self.file_viewer.display_type {
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
        let (btn1, btn2) = match self.file_viewer.endianness {
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
            let btn = if &self.file_viewer.data_type == val {
                render_button(val.to_string(), Color::Green, Color::Black)
            } else {
                render_button(val.to_string(), Color::Yellow, Color::Black)
            };
            frame.render_widget(btn, btn_layout[i]);
        }
    }
}
