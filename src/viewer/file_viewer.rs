use super::common_dt::{DataType, DisplayType, Endianness};
use super::grid::{Grid, Value};
use crate::utils::previous_power_of_two;
use num_traits::Float;
use ratatui::prelude::{Buffer, Rect};
use ratatui::style::{Color, Style, Stylize};
use ratatui::widgets::{
    Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget,
    Widget,
};
use std::fmt::{Display, LowerExp, UpperHex};

#[cfg(debug_assertions)]
use tracing::{info, instrument};

const SUPER_NUMS: [char; 10] = [
    '\u{2070}', '\u{00B9}', '\u{00B2}', '\u{00B3}', '\u{2074}', '\u{2075}', '\u{2076}', '\u{2077}',
    '\u{2078}', '\u{2079}',
];
const SUPER_MINUS: char = '\u{207B}';
const SUB_10: &str = "\u{2081}\u{2080}";
const SUB_16: &str = "\u{2081}\u{2086}";

#[derive(Debug, Default)]
pub struct FileViewer {
    data_type: DataType,
    display_type: DisplayType,
    endianness: Endianness,
    content: Vec<u8>,
}

#[derive(Debug, Default)]
pub struct FileViewerState {
    row_offset: usize,
    col_offset: usize,
    cols: usize,
    rows: usize,
    total_rows: usize,
    set_cols: Option<usize>,
    /// Focused Grid Value (Row, Column).
    cursor_row: usize,
    cursor_col: usize,
    /// Byte-offset Address of the Cursor, refreshed on render.
    cursor_address: Option<usize>,
    scrollbar: Option<ScrollbarState>,
}

impl FileViewerState {
    pub fn cursor_address(&self) -> Option<usize> {
        self.cursor_address
    }

    pub fn cursor(&self) -> (usize, usize) {
        (self.cursor_row, self.cursor_col)
    }

    fn grid_width(&self) -> usize {
        self.set_cols.unwrap_or(self.cols).max(1)
    }

    fn sync_scrollbar(&mut self) {
        if let Some(scroll) = self.scrollbar {
            self.scrollbar = Some(scroll.position(self.row_offset));
        }
    }

    fn ensure_cursor_visible(&mut self) {
        if self.rows == 0 || self.cols == 0 {
            return;
        }
        if self.cursor_row < self.row_offset {
            self.row_offset = self.cursor_row;
        } else if self.cursor_row >= self.row_offset + self.rows {
            self.row_offset = self.cursor_row + 1 - self.rows;
        }
        if self.cursor_col < self.col_offset {
            self.col_offset = self.cursor_col;
        } else if self.cursor_col >= self.col_offset + self.cols {
            self.col_offset = self.cursor_col + 1 - self.cols;
        }
        self.sync_scrollbar();
    }

    fn clamp_cursor(&mut self) {
        if self.total_rows == 0 {
            self.cursor_row = 0;
            self.cursor_col = 0;
            return;
        }
        self.cursor_row = self.cursor_row.min(self.total_rows - 1);
        let width = self.grid_width();
        self.cursor_col = self.cursor_col.min(width.saturating_sub(1));
    }

    pub fn move_down(&mut self) {
        if self.cursor_row + 1 < self.total_rows {
            self.cursor_row += 1;
            self.ensure_cursor_visible();
        }
    }

    pub fn move_up(&mut self) {
        if self.cursor_row > 0 {
            self.cursor_row -= 1;
            self.ensure_cursor_visible();
        }
    }

    pub fn move_right(&mut self) {
        let width = self.grid_width();
        if self.cursor_col + 1 < width {
            self.cursor_col += 1;
            self.ensure_cursor_visible();
        }
    }

    pub fn move_left(&mut self) {
        if self.cursor_col > 0 {
            self.cursor_col -= 1;
            self.ensure_cursor_visible();
        }
    }

    pub fn goto_top(&mut self) {
        self.cursor_row = 0;
        self.ensure_cursor_visible();
    }

    pub fn goto_bottom(&mut self) {
        if self.total_rows > 0 {
            self.cursor_row = self.total_rows - 1;
            self.ensure_cursor_visible();
        }
    }

    pub fn goto_start(&mut self) {
        self.cursor_col = 0;
        self.ensure_cursor_visible();
    }

    pub fn goto_end(&mut self) {
        let width = self.grid_width();
        if width > 0 {
            self.cursor_col = width - 1;
            self.ensure_cursor_visible();
        }
    }

    pub fn scroll_down(&mut self) {
        if self.rows == 0 || self.total_rows == 0 {
            return;
        }
        let step = (self.rows / 2).max(1);
        self.cursor_row = (self.cursor_row + step).min(self.total_rows - 1);
        self.ensure_cursor_visible();
    }

    pub fn scroll_up(&mut self) {
        if self.rows == 0 {
            return;
        }
        let step = (self.rows / 2).max(1);
        self.cursor_row = self.cursor_row.saturating_sub(step);
        self.ensure_cursor_visible();
    }
}

impl StatefulWidget for &FileViewer {
    type State = FileViewerState;
    #[cfg_attr(
        debug_assertions,
        instrument(skip(self, buf, state), name = "FileViewer::render")
    )]
    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let (cols, data_width, _data_size) = self.calc_cols(area);
        let areas = simple_layout_solver(area, cols, data_width);

        #[cfg(debug_assertions)]
        info!(?areas);

        state.rows = area.height as usize - 2;

        state.cols = cols as usize;
        // Logical Width: pinned set_cols, else auto-fit viewport columns.
        let width = state.set_cols.filter(|&w| w > 0).unwrap_or(state.cols).max(1);
        let grid = Grid::from_buffer(&self.content, self.data_type, self.endianness, width);
        state.total_rows = grid.height();
        state.clamp_cursor();
        state.ensure_cursor_visible();
        state.cursor_address = grid.address_at(state.cursor_row, state.cursor_col);

        self.render_header(cols, &areas[..], buf);
        self.render_data(
            &grid,
            state.row_offset,
            state.col_offset,
            state.rows,
            state.cols,
            state.cursor_row,
            state.cursor_col,
            &areas[..],
            buf,
        );

        state.scrollbar = Some(match state.scrollbar {
            Some(scroll) => scroll.content_length(state.total_rows),
            None => ScrollbarState::new(state.total_rows),
        });

        let mut scrollbar_area = areas[state.cols + 1];
        scrollbar_area.height = state.rows as u16 + 1;

        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓"));
        scrollbar.render(scrollbar_area, buf, &mut state.scrollbar.unwrap());
    }
}

impl FileViewer {
    pub fn set_content(&mut self, content: Vec<u8>) {
        self.content = content;
    }
    pub fn set_display_type(&mut self, display_type: DisplayType) {
        self.display_type = display_type;
    }
    pub fn set_data_type(&mut self, data_type: DataType) {
        self.data_type = data_type;
    }
    pub fn set_endianness(&mut self, endianness: Endianness) {
        self.endianness = endianness;
    }

    #[cfg_attr(
        debug_assertions,
        instrument(skip(self, buf), name = "FileViewer::render_header")
    )]
    fn render_header(&self, cols: u16, area: &[Rect], buf: &mut Buffer) {
        let fg = Color::LightCyan;
        let b = Block::default().borders(Borders::RIGHT | Borders::LEFT);
        Paragraph::new(" Address ")
            .style(Style::default().fg(fg).bold())
            .block(b.bg(Color::Reset).fg(fg))
            .render(area[0], buf);

        for i in 0..cols {
            Paragraph::new(format!("{i:X}"))
                .centered()
                .style(Style::default().fg(fg).bold())
                .render(area[i as usize + 1], buf);
        }
        let b = Block::default()
            .borders(Borders::RIGHT)
            .bg(Color::Reset)
            .fg(fg);
        b.render(area[cols as usize + 1], buf);
    }

    #[cfg_attr(
        debug_assertions,
        instrument(skip(self, buf, areas, grid), name = "FileViewer::render_data")
    )]
    fn render_data(
        &self,
        grid: &Grid,
        row_offset: usize,
        col_offset: usize,
        rows: usize,
        cols: usize,
        cursor_row: usize,
        cursor_col: usize,
        areas: &[Rect],
        buf: &mut Buffer,
    ) {
        let fg = Color::LightCyan;
        let mut y = areas[0].y;
        'outer_loop: for row in row_offset..(rows + row_offset) {
            y += 1;
            let mut area = areas[0];
            area.y = y;
            let Some(row_addr) = grid.row_address(row) else {
                break 'outer_loop;
            };
            Paragraph::new(format!(" {row_addr:08X} "))
                .block(
                    Block::default()
                        .borders(Borders::RIGHT | Borders::LEFT)
                        .bg(Color::Reset)
                        .fg(fg),
                )
                .style(Style::default().fg(fg).bold())
                .render(area, buf);
            area = areas[cols + 1];
            area.y = y;
            Block::default()
                .borders(Borders::RIGHT)
                .bg(Color::Reset)
                .fg(fg)
                .render(area, buf);
            for col in col_offset..(cols + col_offset) {
                area = areas[(col - col_offset) + 1];
                area.y = y;

                let Some(value) = grid.value_at(row, col) else {
                    break 'outer_loop;
                };
                let text = format_value(value, self.display_type);
                let style = if row == cursor_row && col == cursor_col {
                    Style::default().fg(Color::Black).bg(Color::Yellow)
                } else {
                    Style::default().fg(Color::Yellow)
                };
                Paragraph::new(text)
                    .right_aligned()
                    .style(style)
                    .render(area, buf);
            }
        }
    }

    #[cfg_attr(
        debug_assertions,
        instrument(skip(self), name = "FileViewer::calc_cols")
    )]
    fn calc_cols(&self, area: Rect) -> (u16, u16, u8) {
        let address_size = 8 + 3 + 1; // 32bit address + 1 margin + 1 sep + 1 space
        use DataType::*;
        use DisplayType::*;
        let (mut data_width, data_size) = match (&self.data_type, &self.display_type) {
            (U8, Decimal) => (3u16, 1u8),
            (I8, Decimal) => (4, 1),
            (U16, Decimal) => (5, 2),
            (I16, Decimal) => (6, 2),
            (U32, Decimal) => (10, 4),
            (I32, Decimal) => (11, 4),
            (U64, Decimal) => (20, 8),
            (I64, Decimal) => (20, 8),
            (U8, HexaDecimal) => (2, 1),
            (I8, HexaDecimal) => (2, 1),
            (U16, HexaDecimal) => (4, 2),
            (I16, HexaDecimal) => (4, 2),
            (U32, HexaDecimal) => (8, 4),
            (I32, HexaDecimal) => (8, 4),
            (U64, HexaDecimal) => (16, 8),
            (I64, HexaDecimal) => (16, 8),
            (F32, Decimal) => (14, 4),
            (F64, Decimal) => (23, 8),
            (F32, HexaDecimal) => (4, 4),
            (F64, HexaDecimal) => (8, 8),
        };
        data_width += 2 + 1; // 2 is for base + 1 for spacing
        let num_cols = (area.width - address_size - 2) / data_width;
        #[cfg(debug_assertions)]
        info!(num_cols, data_width);

        (previous_power_of_two(num_cols), data_width, data_size)
    }
}

fn format_value(value: &Value, display_type: DisplayType) -> String {
    match value {
        Value::U8(v) => format_int(*v, display_type),
        Value::I8(v) => format_int(*v, display_type),
        Value::U16(v) => format_int(*v, display_type),
        Value::I16(v) => format_int(*v, display_type),
        Value::U32(v) => format_int(*v, display_type),
        Value::I32(v) => format_int(*v, display_type),
        Value::U64(v) => format_int(*v, display_type),
        Value::I64(v) => format_int(*v, display_type),
        Value::F32(v) => format_scientific_unicode(*v, 5),
        Value::F64(v) => format_scientific_unicode(*v, 10),
    }
}

fn format_int<T>(val: T, display_type: DisplayType) -> String
where
    T: Display + UpperHex,
{
    match display_type {
        DisplayType::Decimal => format!("{val}{SUB_10}"),
        DisplayType::HexaDecimal => format!("{val:X}{SUB_16}"),
    }
}

#[cfg_attr(debug_assertions, instrument)]
fn simple_layout_solver(area: Rect, cols: u16, data_size: u16) -> Vec<Rect> {
    let mut rects = vec![];
    let Rect {
        mut x,
        y,
        width,
        height: _,
    } = area;

    let address_size = 8;
    let address_padding = 2;
    let address_border = 2;
    let total_address_size = address_size + address_padding + address_border;
    let scrollbar = 1;

    let spacing = (width - (total_address_size + cols * data_size + scrollbar)) / (cols + 1);
    #[cfg(debug_assertions)]
    info!(spacing);

    let remaining_space =
        width - (total_address_size + cols * data_size + scrollbar) - (cols + 1) * spacing;

    rects.push(Rect {
        x,
        y,
        width: total_address_size,
        height: 1,
    });

    x += total_address_size + spacing;

    #[cfg(debug_assertions)]
    info!(x);

    let mut sub = 0;
    for i in 0..cols {
        rects.push(Rect {
            x,
            y,
            width: data_size,
            height: 1,
        });
        x += data_size + spacing;
        if remaining_space * i / cols > sub {
            x += 1;
            sub += 1;
        }
        #[cfg(debug_assertions)]
        info!(x);
    }
    rects.push(Rect {
        x,
        y,
        width: 1,
        height: 1,
    });
    rects
}

fn format_scientific_unicode<T>(val: T, precision: usize) -> String
where
    T: Float + Display + LowerExp,
{
    if val == T::zero() {
        return format!("{:.*}×10{}", precision, val, '⁰');
    }

    if val.is_nan() {
        return String::from("NAN");
    }

    if val.is_infinite() {
        if val.is_sign_negative() {
            return String::from("-∞");
        } else {
            return String::from("∞");
        }
    }

    let sci = format!("{:.*e}", precision, val); // e.g. "1.23e+4"
    let mut parts = sci.split('e');
    let mantissa = parts.next().unwrap();
    let exponent: i32 = parts.next().unwrap().parse().unwrap();

    let superscript = to_superscript(exponent);
    format!("{mantissa}×10{superscript}")
}

fn to_superscript(mut exp: i32) -> String {
    if exp == 0 {
        return SUPER_NUMS[0].to_string();
    }

    let mut s = String::new();
    if exp < 0 {
        s.push(SUPER_MINUS);
        exp = -exp;
    }

    for d in exp.to_string().chars() {
        s.push(SUPER_NUMS[d.to_digit(10).unwrap() as usize]);
    }

    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state_with_grid(total_rows: usize, width: usize, viewport_rows: usize, viewport_cols: usize) -> FileViewerState {
        FileViewerState {
            total_rows,
            cols: viewport_cols,
            rows: viewport_rows,
            set_cols: Some(width),
            ..FileViewerState::default()
        }
    }

    #[test]
    fn cursor_moves_across_grid_values() {
        let mut state = state_with_grid(4, 4, 4, 4);

        assert_eq!(state.cursor(), (0, 0));
        state.move_right();
        assert_eq!(state.cursor(), (0, 1));
        state.move_down();
        assert_eq!(state.cursor(), (1, 1));
        state.move_left();
        assert_eq!(state.cursor(), (1, 0));
        state.move_up();
        assert_eq!(state.cursor(), (0, 0));
    }

    #[test]
    fn cursor_navigation_scrolls_viewport_to_keep_cursor_visible() {
        let mut state = state_with_grid(10, 4, 3, 2);

        for _ in 0..3 {
            state.move_down();
        }
        assert_eq!(state.cursor(), (3, 0));
        assert_eq!(state.row_offset, 1);

        state.goto_end();
        assert_eq!(state.cursor(), (3, 3));
        assert_eq!(state.col_offset, 2);
    }

    #[test]
    fn cursor_stays_within_grid_bounds() {
        let mut state = state_with_grid(2, 2, 2, 2);

        state.move_up();
        state.move_left();
        assert_eq!(state.cursor(), (0, 0));

        state.goto_bottom();
        state.goto_end();
        assert_eq!(state.cursor(), (1, 1));

        state.move_down();
        state.move_right();
        assert_eq!(state.cursor(), (1, 1));
    }
}
