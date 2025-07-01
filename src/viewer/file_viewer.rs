use super::common_dt::{DataType, DisplayType, Endianness};
use crate::utils::previous_power_of_two;
use bytemuck::{AnyBitPattern, cast_slice};
use num_traits::Float;
use ratatui::prelude::{Buffer, Rect};
use ratatui::style::{Color, Style, Stylize};
use ratatui::widgets::{Block, Borders, Paragraph, Widget};
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
    // cols: Option<u16>,
    // rows: Option<u16>,
    content: Vec<u8>,
    pub(super) data_type: DataType,
    pub(super) display_type: DisplayType,
    pub(super) endianness: Endianness,
}

impl Widget for &FileViewer {
    #[cfg_attr(
        debug_assertions,
        instrument(skip(self, buf), name = "FileViewer::render")
    )]
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let (cols, data_size) = self.calc_cols(area);
        let areas = simple_layout_solver(area, cols, 1, data_size);

        #[cfg(debug_assertions)]
        info!(?areas);

        let rows = area.height - 2;

        self.render_header(cols, data_size, &areas[..], buf);
        self.render_data(0, 0, rows, cols, &areas[..], buf);
    }
}

impl FileViewer {
    pub(super) fn set_content(&mut self, content: Vec<u8>) {
        self.content = content;
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
        row_offset: u16,
        col_offset: u16,
        rows: u16,
        cols: u16,
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
        row_offset: u16,
        col_offset: u16,
        rows: u16,
        cols: u16,
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
            Paragraph::new(format!(" {:08X} ", row as u32 * cols as u32))
                .block(
                    Block::default()
                        .borders(Borders::RIGHT | Borders::LEFT)
                        .bg(Color::Reset)
                        .fg(fg),
                )
                .style(Style::default().fg(fg).bold())
                .render(area, buf);
            area = areas[cols as usize + 1];
            area.y = y;
            Block::default()
                .borders(Borders::RIGHT)
                .bg(Color::Reset)
                .fg(fg)
                .render(area, buf);
            for col in col_offset..(cols + col_offset) {
                area = areas[(col - col_offset) as usize + 1];
                area.y = y;

                if (row as u32 * cols as u32 + col as u32) as usize >= content_len {
                    break 'outer_loop;
                }
                match self.display_type {
                    DisplayType::Decimal => {
                        Paragraph::new(format!("{}{SUB_10}", content[(row * cols + col) as usize]))
                            .right_aligned()
                            .style(Style::default().fg(Color::Yellow))
                            .render(area, buf)
                    }
                    DisplayType::HexaDecimal => Paragraph::new(format!(
                        "{:X}{SUB_16}",
                        content[(row * cols + col) as usize]
                    ))
                    .right_aligned()
                    .style(Style::default().fg(Color::Yellow))
                    .render(area, buf),
                }
            }
        }
    }
    fn render_float_data<T, const PREC: usize>(
        &self,
        row_offset: u16,
        col_offset: u16,
        rows: u16,
        cols: u16,
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
            Paragraph::new(format!(" {:08X} ", row as u32 * cols as u32))
                .block(
                    Block::default()
                        .borders(Borders::RIGHT | Borders::LEFT)
                        .bg(Color::Reset)
                        .fg(fg),
                )
                .style(Style::default().fg(fg).bold())
                .render(area, buf);
            area = areas[cols as usize + 1];
            area.y = y;
            Block::default()
                .borders(Borders::RIGHT)
                .bg(Color::Reset)
                .fg(fg)
                .render(area, buf);
            for col in col_offset..(cols + col_offset) {
                area = areas[(col - col_offset) as usize + 1];
                area.y = y;

                if (row as u32 * cols as u32 + col as u32) as usize >= content_len {
                    break 'outer_loop;
                }

                Paragraph::new(format_scientific_unicode(
                    content[(row * cols + col) as usize],
                    PREC,
                ))
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
    fn calc_cols(&self, area: Rect) -> (u16, u16) {
        let address_size = 8 + 3 + 1; // 32bit address + 1 margin + 1 sep + 1 space
        use DataType::*;
        use DisplayType::*;
        let mut data_size = match (&self.data_type, &self.display_type) {
            (U8, Decimal) => 3,
            (I8, Decimal) => 4,
            (U16, Decimal) => 5,
            (I16, Decimal) => 6,
            (U32, Decimal) => 10,
            (I32, Decimal) => 11,
            (U64, Decimal) => 20,
            (I64, Decimal) => 20,
            (U8, HexaDecimal) => 2,
            (I8, HexaDecimal) => 2,
            (U16, HexaDecimal) => 4,
            (I16, HexaDecimal) => 4,
            (U32, HexaDecimal) => 8,
            (I32, HexaDecimal) => 8,
            (U64, HexaDecimal) => 16,
            (I64, HexaDecimal) => 16,
            (F32, Decimal) => 14,
            (F64, Decimal) => 23,
            (F32, HexaDecimal) => 4,
            (F64, HexaDecimal) => 8,
        };
        data_size += 2 + 1; // 2 is for base + 1 for spacing
        let num_cols = (area.width - address_size - 2) / data_size as u16;
        #[cfg(debug_assertions)]
        info!(num_cols, data_size);

        (previous_power_of_two(num_cols), data_size as u16)
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
