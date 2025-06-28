use ratatui::prelude::{Buffer, Rect};
use ratatui::widgets::Widget;

#[derive(Debug, Default)]
pub struct FileViewer {
    // row: usize,
    // file_content: Vec<u8>,
}

impl Widget for &FileViewer {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let size = area.as_size();
        format!("{size:?}").render(area, buf);
    }
}
