use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Flex, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
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

#[derive(Debug, Default, PartialEq, Eq)]
pub enum DisplayType {
    #[default]
    Decimal,
    HexaDecimal,
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
    display_type: DisplayType,
    file: String,
    // search_field: String,
    action_mode: ActionMode,
}

pub enum ViewerEvent {
    Quit,
    Poll,
}

#[derive(Debug, Default)]
pub enum ActionMode {
    #[default]
    Normal,
    // EditSearch,
    SelectDataType(Option<KeyCode>),
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
            (_, KeyCode::Char('d')) => self.display_type = DisplayType::Decimal,
            (_, KeyCode::Char('x')) => self.display_type = DisplayType::HexaDecimal,
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
                (Char('u'), Char('2')) => self.data_type = DataType::U32,
                (Char('i'), Char('2')) => self.data_type = DataType::I32,
                (Char('u'), Char('6')) => self.data_type = DataType::U16,
                (Char('i'), Char('6')) => self.data_type = DataType::I16,
                (Char('u'), Char('8')) => self.data_type = DataType::U8,
                (Char('i'), Char('8')) => self.data_type = DataType::I8,
                (Char('f'), Char('2')) => self.data_type = DataType::F32,
                (Char('f'), Char('6')) => self.data_type = DataType::F16,
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
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Fill(1),
        ])
        .areas::<3>(frame.area());

        let file_name = Line::from(vec![
            Span::styled("File: ", Style::default().fg(Color::Yellow)),
            Span::styled(" ", Style::default().bg(Color::Gray)),
            Span::styled(
                self.file.as_str(),
                Style::default()
                    .fg(Color::LightBlue)
                    .bg(Color::Gray)
                    .bold()
                    .italic(),
            ),
            Span::styled(" ", Style::default().bg(Color::Gray)),
        ]);
        //Paragraph::new(self.file.as_str()).bold().blue();
        frame.render_widget(file_name, page_layout[0]);

        let layout = Layout::horizontal([
            Constraint::Length(72),
            Constraint::Length(32),
            Constraint::Fill(1),
        ])
        .areas::<3>(page_layout[1]);

        self.render_dt_buttons(layout[0], frame);
        self.render_display_buttons(layout[1], frame);

        let b = Block::default()
            .title(" Search ")
            .border_style(Style::default().fg(Color::Rgb(70, 70, 70)))
            .border_type(BorderType::Rounded)
            .borders(Borders::ALL);
        frame.render_widget(b, layout[2]);

        Ok(())
    }

    fn render_display_buttons(&self, rect: Rect, frame: &mut Frame) {
        let b = Block::default()
            .border_style(Style::default().fg(Color::Cyan))
            .border_type(BorderType::Rounded)
            .borders(Borders::ALL)
            .title(Line::from(" Display Type ").centered());
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

    fn render_dt_buttons(&self, rect: Rect, frame: &mut Frame) {
        let b = Block::default()
            .border_style(Style::default().fg(Color::Cyan))
            .border_type(BorderType::Rounded)
            .borders(Borders::ALL)
            .title(Line::from(" Data Type ").centered());
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
        ])
        .flex(Flex::SpaceBetween)
        .vertical_margin(1)
        .horizontal_margin(2)
        // .spacing(2)
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
