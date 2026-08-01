use ratatui::layout::{Constraint, Layout, Rect, Flex};

/// Terminals narrower than this don't get a details pane.
const DETAILS_BREAKPOINT: u16 = 100;
/// Width of the details pane when it is shown.
const DETAILS_WIDTH: u16 = 32;

/// Every region the UI needs, computed once per frame from the terminal size.
#[derive(Debug, Clone, Copy)]
pub struct AppLayout {
    pub header: Rect,
    pub columns: [Rect; 3],
    pub details: Option<Rect>,
    pub footer: Rect,
}

impl AppLayout {
    pub fn compute(area: Rect) -> Self {
        let [header, body, footer] = Layout::vertical([
            Constraint::Length(3),
            Constraint::Fill(1),
            Constraint::Length(1),
        ])
        .areas(area);

        let (board, details) = if area.width >= DETAILS_BREAKPOINT {
            let [board, details] =
                Layout::horizontal([Constraint::Fill(1), Constraint::Length(DETAILS_WIDTH)])
                    .areas(body);
            (board, Some(details))
        } else {
            (body, None)
        };

        let columns = Layout::horizontal([Constraint::Fill(1); 3])
            .spacing(1)
            .areas(board);

        Self { header, columns, details, footer }
    }    
}

pub fn center(area: Rect, horizontal: Constraint, vertical: Constraint) -> Rect {
    let [area] = Layout::horizontal([horizontal]).flex(Flex::Center).areas(area);
    let [area] = Layout::vertical([vertical]).flex(Flex::Center).areas(area);
    area
}