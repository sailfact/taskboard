use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};

const EIGHTHS: [&str; 9] = [" ", "▏", "▎", "▍", "▌", "▋", "▊", "▉", "█"];

pub fn draw(frame: &mut Frame, area: Rect, ratio: f64, filled: Color, track: Color) {
    if area.is_empty() {
        return;
    }

    let style = Style::new().fg(filled).bg(track);
    let total = (f64::from(area.width) * 8.0 * ratio.clamp(0.0, 1.0)).round() as u16;
    let buffer = frame.buffer_mut();

    for offset in 0..area.width {
        let eighths = total.saturating_sub(offset * 8).min(8) as usize;

        if let Some(cell) = buffer.cell_mut((area.x + offset, area.y)) {
            cell.set_symbol(EIGHTHS[eighths]).set_style(style);
        }
    }
}
