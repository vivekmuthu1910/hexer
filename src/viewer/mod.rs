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
mod grid;

pub use common_dt::{DataType, DisplayType, Endianness, ViewMode};
use common_dt::stride_help_text;

#[derive(Debug, Default)]
pub struct ViewerContainer {
    file: PathBuf,
    action_mode: ActionMode,
    file_viewer: FileViewer,
    file_viewer_state: FileViewerState,
    data_type: DataType,
    display_type: DisplayType,
    endianness: Endianness,
    view_mode: ViewMode,
    /// `None` → Binary auto-fit Width to the terminal.
    pinned_width: Option<usize>,
    /// `None` → Stride follows Width.
    pinned_stride: Option<usize>,
    show_padding: bool,
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
    Help,
}

fn render_button(name: String, btn_color: Color, text_color: Color) -> impl Widget {
    Paragraph::new(name).fg(text_color).bg(btn_color).centered()
}

impl ViewerContainer {
    pub fn with_file(mut self, file: PathBuf) -> Self {
        self.file = file;
        self
    }

    /// Apply CLI launch options as the initial viewer session state (still editable in the TUI).
    pub fn with_launch_options(mut self, options: &crate::launch::ViewerLaunchOptions) -> Self {
        self.data_type = options.data_type;
        self.display_type = options.display_type;
        self.endianness = options.endianness;
        self.view_mode = options.view_mode;
        self.pinned_width = options.width;
        self.pinned_stride = options.stride;
        self.show_padding = options.show_padding;

        self.file_viewer.set_data_type(options.data_type);
        self.file_viewer.set_display_type(options.display_type);
        self.file_viewer.set_endianness(options.endianness);
        self.file_viewer.set_view_mode(options.view_mode);
        self.sync_layout_to_viewer();
        self
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> ViewerContainerEvent {
        match self.action_mode {
            ActionMode::Normal => self.handle_normal_keys(key),
            ActionMode::SelectDataType(_) => self.handle_dt_keys(key),
            ActionMode::Help => {
                match key.code {
                    KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q') => {
                        self.action_mode = ActionMode::Normal;
                    }
                    _ => {}
                }
                ViewerContainerEvent::Poll
            }
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
            // Width: ]/[ pin & adjust; a = auto (unpin)
            (_, KeyCode::Char(']')) => self.bump_width(1),
            (_, KeyCode::Char('[')) => self.bump_width(-1),
            (_, KeyCode::Char('a')) => {
                // Auto Width is Binary-only; Image requires an explicit Width.
                if !self.view_mode.requires_explicit_width() {
                    self.pinned_width = None;
                    self.sync_layout_to_viewer();
                }
            }
            // Stride: }/{ adjust; = resets Stride to follow Width
            (_, KeyCode::Char('}')) => self.bump_stride(1),
            (_, KeyCode::Char('{')) => self.bump_stride(-1),
            (_, KeyCode::Char('=')) => {
                self.pinned_stride = None;
                self.sync_layout_to_viewer();
            }
            // Padding visibility
            (_, KeyCode::Char('p')) => {
                self.show_padding = !self.show_padding;
                self.sync_layout_to_viewer();
            }
            (_, KeyCode::Char('v')) => self.toggle_view_mode(),
            (_, KeyCode::Char('?')) => self.action_mode = ActionMode::Help,
            _ => {}
        }
        ViewerContainerEvent::Poll
    }

    fn toggle_view_mode(&mut self) {
        let next = self.view_mode.toggle();
        self.view_mode = next;
        if next.requires_explicit_width() && self.pinned_width.is_none() {
            self.pinned_width = Some(self.current_width_for_edit());
        }
        self.file_viewer.set_view_mode(next);
        self.sync_layout_to_viewer();
    }

    fn sync_layout_to_viewer(&mut self) {
        self.file_viewer.set_pinned_width(self.pinned_width);
        self.file_viewer.set_pinned_stride(self.pinned_stride);
        self.file_viewer.set_show_padding(self.show_padding);
    }

    fn current_width_for_edit(&self) -> usize {
        self.pinned_width
            .or_else(|| {
                let w = self.file_viewer_state.viewport_width();
                if w > 0 {
                    Some(w)
                } else {
                    None
                }
            })
            .unwrap_or(1)
            .max(1)
    }

    fn bump_width(&mut self, delta: i32) {
        let current = self.current_width_for_edit() as i32;
        let next = (current + delta).max(1) as usize;
        self.pinned_width = Some(next);
        if let Some(stride) = self.pinned_stride {
            if stride < next {
                self.pinned_stride = Some(next);
            }
        }
        self.sync_layout_to_viewer();
    }

    fn bump_stride(&mut self, delta: i32) {
        let width = self.current_width_for_edit();
        let current = self.pinned_stride.unwrap_or(width) as i32;
        let next = (current + delta).max(width as i32) as usize;
        self.pinned_stride = Some(next);
        self.sync_layout_to_viewer();
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
            Constraint::Length(1),
        ])
        .areas::<4>(frame.area());

        let layout = Layout::horizontal([Constraint::Length(70), Constraint::Fill(1)])
            .areas::<2>(page_layout[0]);
        self.render_file_name(layout[0], frame);
        self.render_search_bar(layout[1], frame);

        let layout = Layout::horizontal([
            Constraint::Length(70),
            Constraint::Length(22),
            Constraint::Length(23),
            Constraint::Length(24),
        ])
        .flex(Flex::SpaceAround)
        .areas::<4>(page_layout[1]);

        self.render_dt_buttons(layout[0], frame);
        self.render_display_buttons(layout[1], frame);
        self.render_endianness_buttons(layout[2], frame);
        self.render_view_mode_buttons(layout[3], frame);

        let content = fs::read(&self.file)?;

        #[cfg(debug_assertions)]
        info!("Content len: {}", content.len());

        self.file_viewer.set_content(content);
        self.file_viewer.set_view_mode(self.view_mode);
        self.sync_layout_to_viewer();
        frame.render_stateful_widget(
            &self.file_viewer,
            page_layout[2],
            &mut self.file_viewer_state,
        );
        self.render_status(page_layout[3], frame);

        if matches!(self.action_mode, ActionMode::Help) {
            self.render_help(frame.area(), frame);
        }
        Ok(())
    }

    fn render_status(&self, rect: Rect, frame: &mut Frame) {
        let width_label = match self.pinned_width {
            Some(w) => w.to_string(),
            None => format!("auto({})", self.file_viewer_state.viewport_width().max(1)),
        };
        let stride_base = self
            .pinned_width
            .unwrap_or_else(|| self.file_viewer_state.viewport_width().max(1));
        let stride_label = self
            .pinned_stride
            .unwrap_or(stride_base)
            .max(stride_base)
            .to_string();
        let padding_label = if self.show_padding { "show" } else { "omit" };

        let mut spans = Vec::new();
        if self.view_mode.uses_address_gutter() {
            let address = self
                .file_viewer_state
                .cursor_address()
                .map(|a| format!("{a:08X}"))
                .unwrap_or_else(|| "--------".to_string());
            spans.extend([
                Span::styled(" Address: ", Style::default().fg(Color::LightCyan).bold()),
                Span::styled(address, Style::default().fg(Color::Yellow).bold()),
            ]);
        } else {
            let (row, col) = self.file_viewer_state.cursor();
            spans.extend([
                Span::styled(" Cursor: ", Style::default().fg(Color::LightCyan).bold()),
                Span::styled(
                    format!("({row}, {col})"),
                    Style::default().fg(Color::Yellow).bold(),
                ),
            ]);
        }
        spans.extend([
            Span::styled("  Width: ", Style::default().fg(Color::LightCyan).bold()),
            Span::styled(width_label, Style::default().fg(Color::Yellow).bold()),
            Span::styled("  Stride: ", Style::default().fg(Color::LightCyan).bold()),
            Span::styled(stride_label, Style::default().fg(Color::Yellow).bold()),
            Span::styled(" (Values)", Style::default().fg(Color::DarkGray)),
            Span::styled("  Padding: ", Style::default().fg(Color::LightCyan).bold()),
            Span::styled(padding_label, Style::default().fg(Color::Yellow).bold()),
            Span::styled("  ?:help", Style::default().fg(Color::DarkGray)),
        ]);
        frame.render_widget(Paragraph::new(Line::from(spans)), rect);
    }

    fn render_help(&self, area: Rect, frame: &mut Frame) {
        let help = format!(
            " Help \n\n\
             View Mode (v): Binary ↔ Image — same Grid engine\n\
             {}\n\
             Width [/]  auto(a, Binary)  Stride {{/}}  reset(=)  Padding(p)\n\
             Esc/?/q close help",
            stride_help_text()
        );
        let block = Block::default()
            .title(" Help ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .border_type(BorderType::Rounded);
        let inner = area.inner(Margin {
            horizontal: area.width.saturating_sub(60) / 2,
            vertical: area.height.saturating_sub(10) / 2,
        });
        frame.render_widget(Paragraph::new(help).block(block), inner);
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
        let btn_layout = Layout::horizontal([Length(11), Length(7)])
            .flex(Flex::SpaceBetween)
            .vertical_margin(1)
            .horizontal_margin(2)
            .split(rect);
        let (btn1, btn2) = match self.display_type {
            DisplayType::Decimal => (
                render_button(
                    DisplayType::Decimal.label().to_string(),
                    Color::Green,
                    Color::Black,
                ),
                render_button(
                    DisplayType::HexaDecimal.label().to_string(),
                    Color::Yellow,
                    Color::Black,
                ),
            ),
            DisplayType::HexaDecimal => (
                render_button(
                    DisplayType::Decimal.label().to_string(),
                    Color::Yellow,
                    Color::Black,
                ),
                render_button(
                    DisplayType::HexaDecimal.label().to_string(),
                    Color::Green,
                    Color::Black,
                ),
            ),
        };
        frame.render_widget(btn1, btn_layout[0]);
        frame.render_widget(btn2, btn_layout[1]);
    }

    fn render_view_mode_buttons(&self, rect: Rect, frame: &mut Frame) {
        let b = Block::default()
            .border_style(Style::default().fg(Color::Cyan))
            .border_type(BorderType::Rounded)
            .borders(Borders::ALL)
            .title(Line::from(" View Mode "));
        frame.render_widget(b, rect);

        use Constraint::Length;
        let btn_layout = Layout::horizontal([Length(10), Length(9)])
            .flex(Flex::SpaceBetween)
            .vertical_margin(1)
            .horizontal_margin(2)
            .split(rect);
        let (btn1, btn2) = match self.view_mode {
            ViewMode::Binary => (
                render_button("Binary".to_string(), Color::Green, Color::Black),
                render_button("Image".to_string(), Color::Yellow, Color::Black),
            ),
            ViewMode::Image => (
                render_button("Binary".to_string(), Color::Yellow, Color::Black),
                render_button("Image".to_string(), Color::Green, Color::Black),
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

#[cfg(test)]
mod launch_options_tests {
    use super::*;
    use crate::launch::ViewerLaunchOptions;
    use std::path::PathBuf;

    #[test]
    fn launch_options_seed_viewer_state() {
        let options = ViewerLaunchOptions {
            data_type: DataType::U32,
            display_type: DisplayType::HexaDecimal,
            endianness: Endianness::Big,
            width: Some(16),
            stride: Some(20),
            view_mode: ViewMode::Image,
            show_padding: true,
        };
        let viewer = ViewerContainer::default()
            .with_file(PathBuf::from("sample.bin"))
            .with_launch_options(&options);

        assert_eq!(viewer.data_type, DataType::U32);
        assert_eq!(viewer.display_type, DisplayType::HexaDecimal);
        assert_eq!(viewer.endianness, Endianness::Big);
        assert_eq!(viewer.pinned_width, Some(16));
        assert_eq!(viewer.pinned_stride, Some(20));
        assert_eq!(viewer.view_mode, ViewMode::Image);
        assert!(viewer.show_padding);
    }

    #[test]
    fn launch_options_do_not_freeze_tui_changes() {
        let options = ViewerLaunchOptions {
            data_type: DataType::U16,
            ..ViewerLaunchOptions::default()
        };
        let mut viewer = ViewerContainer::default()
            .with_file(PathBuf::from("sample.bin"))
            .with_launch_options(&options);

        assert_eq!(viewer.data_type, DataType::U16);
        // Same mutations the TUI key handlers perform after launch.
        viewer.data_type = DataType::U8;
        viewer.file_viewer.set_data_type(DataType::U8);
        viewer.display_type = DisplayType::HexaDecimal;
        viewer.pinned_width = Some(4);
        assert_eq!(viewer.data_type, DataType::U8);
        assert_eq!(viewer.display_type, DisplayType::HexaDecimal);
        assert_eq!(viewer.pinned_width, Some(4));
    }
}

