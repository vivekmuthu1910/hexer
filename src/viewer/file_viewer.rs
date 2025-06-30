use ratatui::layout::{Constraint, Flex, Layout};
use ratatui::prelude::{Buffer, Rect};
use ratatui::style::{Color, Style, Stylize};
use ratatui::widgets::{Block, Borders, Paragraph, Widget};
#[cfg(debug_assertions)]
use tracing::{info, instrument};

use super::common_dt::{DataType, DisplayType, Endianness};
use crate::utils::previous_power_of_two;

const SUPER_NUMS: [char; 10] = [
    '\u{2070}', '\u{00B9}', '\u{00B2}', '\u{00B3}', '\u{2074}', '\u{2075}', '\u{2076}', '\u{2077}',
    '\u{2078}', '\u{2079}',
];
const SUPER_MINUS: char = '\u{207B}';
const SUB_10: &'static str = "\u{2081}\u{2080}";
const SUB_16: &'static str = "\u{2081}\u{2086}";

#[derive(Debug, Default)]
pub struct FileViewer {
    cols: Option<u16>,
    rows: Option<u16>,
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
        self.render_data(0, 0, rows, cols, data_size, &areas[..], buf);
    }
}

impl FileViewer {
    pub(super) fn set_content(&mut self, content: Vec<u8>) {
        self.content = content;
    }

    #[cfg_attr(
        debug_assertions,
        instrument(skip(self), name = "FileViewer::calc_cols")
    )]
    fn calc_cols(&self, area: Rect) -> (u16, u16) {
        let address_size = 8 + 3 + 1; // 32bit address + 1 margin + 1 sep + 1 space
        let data_size = self.calc_data_size() + 2 + 1; // 2 is for base + 1 for spacing
        let num_cols = (area.width - address_size - 2) / data_size as u16;
        #[cfg(debug_assertions)]
        info!(num_cols, data_size);

        (previous_power_of_two(num_cols), data_size as u16)
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
        data_size: u16,
        areas: &[Rect],
        buf: &mut Buffer,
    ) {
        let fg = Color::LightCyan;
        let mut y = areas[0].y;
        for row in 0..rows {
            y += 1;
            let mut area = areas[0];
            area.y = y;
            Paragraph::new(format!(" {:08X} ", (row + row_offset) * cols))
                .block(
                    Block::default()
                        .borders(Borders::RIGHT | Borders::LEFT)
                        .bg(Color::Reset)
                        .fg(fg),
                )
                .style(Style::default().fg(fg).bold())
                .render(area, buf);
            for col in 0..cols {
                area = areas[col as usize + 1];
                area.y = y;
                Paragraph::new(format!("127{SUB_10}"))
                    .centered()
                    .style(Style::default().fg(Color::Yellow))
                    // .block(
                    //     Block::default()
                    //         .borders(Borders::ALL)
                    //         .bg(Color::Reset)
                    //         .fg(fg),
                    // )
                    .render(area, buf);
            }

            area = areas[cols as usize + 1];
            area.y = y;
            Block::default()
                .borders(Borders::RIGHT)
                .bg(Color::Reset)
                .fg(fg)
                .render(area, buf);
        }
    }

    fn calc_data_size(&self) -> u8 {
        use DataType::*;
        use DisplayType::*;
        match (&self.data_type, &self.display_type) {
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
            (F32, Decimal) => 4,
            (F64, Decimal) => 8,
            (F32, HexaDecimal) => 4,
            (F64, HexaDecimal) => 8,
        }
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
