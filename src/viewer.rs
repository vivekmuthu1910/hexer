use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Flex, Layout, Rect},
    style::{Color, Style, Stylize},
    text::Line,
    widgets::{Block, BorderType, Borders, Paragraph, Widget},
};
use std::{fmt, io::Result};

#[derive(Debug, Default, PartialEq, Eq)]
pub enum DataType {
    #[default]
    U8,
    I8,
    U16,
    I16,
    U32,
    I32,
    F16,
    F32,
}
impl DataType {
    const ALL: [DataType; 8] = [
        DataType::U8,
        DataType::I8,
        DataType::U16,
        DataType::I16,
        DataType::U32,
        DataType::I32,
        DataType::F16,
        DataType::F32,
    ];
}

impl fmt::Display for DataType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DataType::U8 => write!(f, "U8"),
            DataType::I8 => write!(f, "I8"),
            DataType::U16 => write!(f, "U16"),
            DataType::I16 => write!(f, "I16"),
            DataType::U32 => write!(f, "U32"),
            DataType::I32 => write!(f, "I32"),
            DataType::F16 => write!(f, "F16"),
            DataType::F32 => write!(f, "F32"),
        }
    }
}

#[derive(Debug, Default)]
pub struct ViewerState {
    data_type: DataType,
    file: String,
    search_field: String,
}

pub enum ViewerEvent {
    Quit,
    Poll,
}

pub enum ActionMode {
    SelectDataType,
    EditSearch,
}

fn render_button(name: String, btn_color: Color, text_color: Color) -> impl Widget {
    Paragraph::new(name).fg(text_color).bg(btn_color).centered()
}

impl ViewerState {
    pub fn with_file(mut self, file: String) -> Self {
        self.file = file;
        self
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> ViewerEvent {
        match (key.modifiers, key.code) {
            (_, KeyCode::Esc | KeyCode::Char('q'))
            | (KeyModifiers::CONTROL, KeyCode::Char('c') | KeyCode::Char('C')) => {
                return ViewerEvent::Quit;
            }
            _ => {}
        }
        ViewerEvent::Poll
    }

    pub fn render_viewer(&mut self, frame: &mut Frame) -> Result<()> {
        let page_layout = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Fill(1),
        ])
        .areas::<3>(frame.area());

        let file_name = Paragraph::new(self.file.as_str()).bold().blue();
        frame.render_widget(file_name, page_layout[0]);

        let layout = Layout::horizontal([Constraint::Fill(2), Constraint::Fill(1)])
            .areas::<2>(page_layout[1]);

        let b = Block::default()
            .border_style(Style::default().fg(Color::Cyan))
            .border_type(BorderType::Rounded)
            .borders(Borders::ALL)
            .title(Line::from(" Data Type ").centered());

        frame.render_widget(b, layout[0]);

        self.render_buttons(layout[0], frame);

        let b = Block::default()
            .title(" Search ")
            .border_style(Style::default().fg(Color::Cyan))
            .border_type(BorderType::Rounded)
            .borders(Borders::ALL);
        frame.render_widget(b, layout[1]);

        Ok(())
    }

    fn render_buttons(&self, rect: Rect, frame: &mut Frame) {
        let btn_layout = Layout::horizontal([Constraint::Max(8); 8])
            .flex(Flex::SpaceBetween)
            .vertical_margin(1)
            .horizontal_margin(10)
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
