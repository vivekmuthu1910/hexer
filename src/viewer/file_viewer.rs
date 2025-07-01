use super::common_dt::{DataType, DisplayType, Endianness};
use crate::utils::previous_power_of_two;
use bytemuck::{AnyBitPattern, cast_slice};
use num_traits::Float;
use ratatui::prelude::{Buffer, Rect};
use ratatui::style::{Color, Style, Stylize};
use ratatui::widgets::{Block, Borders, Paragraph, StatefulWidget, Widget};
use std::fmt::{Display, LowerExp, UpperHex};

#[cfg(debug_assertions)]
use tracing::{info, instrument};

const SUPER_NUMS: [char; 10] = [
    '\u{2070}', '\u{00B9}', '\u{00B2}', '\u{00B3}', '\u{2074}', '\u{2075}', '\u{2076}', '\u{2077}',
    '\u{2078}', '\u{2079}',
];
const SUPER_MINUS: char = '\u{207B}';
const SUB_10: &'static str = "\u{2081}\u{2080}";
const SUB_16: &'static str = "\u{2081}\u{2086}";

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
}

impl FileViewerState {
    pub fn move_down(&mut self) {
        if self.total_rows > self.rows {
            if self.row_offset < (self.total_rows - self.rows) {
                self.row_offset += 1;
            }
        }
    }

    pub fn move_up(&mut self) {
        if self.row_offset > 0 {
            self.row_offset -= 1;
        }
    }

    pub fn move_right(&mut self) {
        match self.set_cols {
            Some(col) => {
                if col > self.cols {
                    if self.col_offset < (col - self.cols) {
                        self.col_offset += 1;
                    }
                }
            }
            None => {}
        }
    }

    pub fn move_left(&mut self) {
        match self.set_cols {
            Some(_) => {
                if self.col_offset > 0 {
                    self.col_offset -= 1;
                }
            }
            None => {}
        }
    }

    pub fn goto_top(&mut self) {
        self.row_offset = 0;
    }
    pub fn goto_bottom(&mut self) {
        if self.total_rows > self.rows {
            self.row_offset = self.total_rows - self.rows;
        }
    }
    pub fn goto_start(&mut self) {
        self.col_offset = 0;
    }
    pub fn goto_end(&mut self) {
        match self.set_cols {
            Some(col) => {
                if col > self.cols {
                    self.col_offset = col - self.cols;
                }
            }
            None => {}
        }
    }

    pub fn scroll_down(&mut self) {
        if self.total_rows > self.rows * 3 / 2 {
            if self.row_offset < (self.total_rows - self.rows - self.rows / 2) {
                self.row_offset += self.rows / 2;
            } else {
                self.row_offset = self.total_rows - self.rows;
            }
        }
    }
    pub fn scroll_up(&mut self) {
        if self.row_offset > self.rows / 2 {
            self.row_offset -= self.rows / 2;
        } else {
            self.row_offset = 0;
        }
    }
}

impl StatefulWidget for &FileViewer {
    type State = FileViewerState;
    #[cfg_attr(
        debug_assertions,
        instrument(skip(self, buf, state), name = "FileViewer::render")
    )]
    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let (cols, data_width, data_size) = self.calc_cols(area);
        let areas = simple_layout_solver(area, cols, 1, data_width);

        #[cfg(debug_assertions)]
        info!(?areas);

        state.rows = area.height as usize - 2;

        state.cols = cols as usize;
        state.total_rows = match state.set_cols {
            Some(col) => self.content.len() / (data_size as usize * col),
            None => self.content.len() / (data_size as usize * state.cols),
        };

        self.render_header(cols, data_width, &areas[..], buf);
        self.render_data(
            state.row_offset,
            state.col_offset,
            state.rows,
            state.cols,
            &areas[..],
            buf,
        );
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
    fn render_header(&self, cols: u16, data_size: u16, area: &[Rect], buf: &mut Buffer) {
        let fg = Color::LightCyan;
        let b = Block::default().borders(Borders::RIGHT | Borders::LEFT);
        Paragraph::new(" Address ")
            .style(Style::default().fg(fg).bold())
            .block(b.bg(Color::Reset).fg(fg))
            .render(area[0], buf);

        for i in 0..cols {
            Paragraph::new(format!("{i}"))
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
        instrument(skip(self, buf, areas), name = "FileViewer::render_data")
    )]
    fn render_data(
        &self,
        row_offset: usize,
        col_offset: usize,
        rows: usize,
        cols: usize,
        areas: &[Rect],
        buf: &mut Buffer,
    ) {
        match self.data_type {
            DataType::U8 => {
                self.render_int_data::<u8>(row_offset, col_offset, rows, cols, areas, buf)
            }
            DataType::I8 => {
                self.render_int_data::<i8>(row_offset, col_offset, rows, cols, areas, buf)
            }
            DataType::U16 => {
                self.render_int_data::<u16>(row_offset, col_offset, rows, cols, areas, buf)
            }
            DataType::I16 => {
                self.render_int_data::<i16>(row_offset, col_offset, rows, cols, areas, buf)
            }
            DataType::U32 => {
                self.render_int_data::<u32>(row_offset, col_offset, rows, cols, areas, buf)
            }
            DataType::I32 => {
                self.render_int_data::<i32>(row_offset, col_offset, rows, cols, areas, buf)
            }
            DataType::U64 => {
                self.render_int_data::<u64>(row_offset, col_offset, rows, cols, areas, buf)
            }
            DataType::I64 => {
                self.render_int_data::<i64>(row_offset, col_offset, rows, cols, areas, buf)
            }
            DataType::F32 => {
                self.render_float_data::<f32, 5>(row_offset, col_offset, rows, cols, areas, buf)
            }
            DataType::F64 => {
                self.render_float_data::<f64, 10>(row_offset, col_offset, rows, cols, areas, buf)
            }
        }
    }

    fn render_int_data<T>(
        &self,
        row_offset: usize,
        col_offset: usize,
        rows: usize,
        cols: usize,
        areas: &[Rect],
        buf: &mut Buffer,
    ) where
        T: AnyBitPattern + Display + UpperHex,
    {
        let fg = Color::LightCyan;
        let mut y = areas[0].y;
        // let content = &self.content;
        let content: &[T] = cast_slice(&self.content);
        let content_len = content.len();
        'outer_loop: for row in row_offset..(rows + row_offset) {
            y += 1;
            let mut area = areas[0];
            area.y = y;
            Paragraph::new(format!(" {:08X} ", row * cols))
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

                if (row * cols + col) >= content_len {
                    break 'outer_loop;
                }
                match self.display_type {
                    DisplayType::Decimal => {
                        Paragraph::new(format!("{}{SUB_10}", content[row * cols + col]))
                            .right_aligned()
                            .style(Style::default().fg(Color::Yellow))
                            .render(area, buf)
                    }
                    DisplayType::HexaDecimal => {
                        Paragraph::new(format!("{:X}{SUB_16}", content[row * cols + col]))
                            .right_aligned()
                            .style(Style::default().fg(Color::Yellow))
                            .render(area, buf)
                    }
                }
            }
        }
    }

    fn render_float_data<T, const PREC: usize>(
        &self,
        row_offset: usize,
        col_offset: usize,
        rows: usize,
        cols: usize,
        areas: &[Rect],
        buf: &mut Buffer,
    ) where
        T: AnyBitPattern + Display + Float + LowerExp,
    {
        let fg = Color::LightCyan;
        let mut y = areas[0].y;
        let content: &[T] = cast_slice(&self.content);
        let content_len = content.len();
        'outer_loop: for row in row_offset..(rows + row_offset) {
            y += 1;
            let mut area = areas[0];
            area.y = y;
            Paragraph::new(format!(" {:08X} ", row * cols))
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

                if (row * cols + col) >= content_len {
                    break 'outer_loop;
                }

                Paragraph::new(format_scientific_unicode(content[row * cols + col], PREC))
                    .right_aligned()
                    .style(Style::default().fg(Color::Yellow))
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

#[cfg_attr(debug_assertions, instrument)]
fn simple_layout_solver(area: Rect, cols: u16, rows: u16, data_size: u16) -> Vec<Rect> {
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
    let right_border = 1;

    let spacing = (width - (total_address_size + cols * data_size + right_border)) / (cols + 1);
    #[cfg(debug_assertions)]
    info!(spacing);

    let remaining_space =
        width - (total_address_size + cols * data_size + right_border) - (cols + 1) * spacing;
    let front_margin = remaining_space / 2;
    #[cfg(debug_assertions)]
    info!(front_margin);
    x += front_margin;

    #[cfg(debug_assertions)]
    info!(x);

    rects.push(Rect {
        x: x,
        y,
        width: total_address_size,
        height: 1,
    });

    x += total_address_size + spacing;

    #[cfg(debug_assertions)]
    info!(x);

    for _ in 0..cols {
        rects.push(Rect {
            x,
            y,
            width: data_size,
            height: 1,
        });
        x += data_size + spacing;
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
