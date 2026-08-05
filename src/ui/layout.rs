use ratatui::layout::{Constraint, Flex, Layout, Rect};

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
            Constraint::Length(4),
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

        Self {
            header,
            columns,
            details,
            footer,
        }
    }
}

pub fn center(area: Rect, horizontal: Constraint, vertical: Constraint) -> Rect {
    let [area] = Layout::horizontal([horizontal])
        .flex(Flex::Center)
        .areas(area);
    let [area] = Layout::vertical([vertical]).flex(Flex::Center).areas(area);
    area
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_and_footer_are_fixed_height() {
        let layout = AppLayout::compute(Rect::new(0, 0, 120, 40));

        assert_eq!(layout.header.height, 4);
        assert_eq!(layout.footer.height, 1);
    }

    #[test]
    fn body_absorbs_the_remaining_height() {
        let layout = AppLayout::compute(Rect::new(0, 0, 120, 40));
        let body_height = layout.columns[0].height;

        assert_eq!(
            layout.header.height + body_height + layout.footer.height,
            40
        );
    }

    #[test]
    fn details_pane_appears_at_the_breakpoint() {
        assert!(
            AppLayout::compute(Rect::new(0, 0, 99, 40))
                .details
                .is_none()
        );
        assert!(
            AppLayout::compute(Rect::new(0, 0, 100, 40))
                .details
                .is_some()
        );
    }

    #[test]
    fn details_pane_has_a_fixed_width() {
        let layout = AppLayout::compute(Rect::new(0, 0, 200, 40));

        assert_eq!(layout.details.map(|d| d.width), Some(DETAILS_WIDTH));
    }

    #[test]
    fn columns_do_not_overlap() {
        let layout = AppLayout::compute(Rect::new(0, 0, 120, 40));

        for pair in layout.columns.windows(2) {
            assert!(
                pair[0].right() <= pair[1].left(),
                "{:?} overlaps {:?}",
                pair[0],
                pair[1]
            );
        }
    }

    #[test]
    fn columns_stay_inside_the_board() {
        let layout = AppLayout::compute(Rect::new(0, 0, 120, 40));
        let last = layout.columns[2];

        match layout.details {
            Some(details) => assert!(last.right() <= details.left()),
            None => assert!(last.right() <= 120),
        }
    }

    #[test]
    fn no_panic_at_any_plausible_size() {
        for width in 0..=200 {
            for height in [0, 1, 2, 3, 4, 5, 40] {
                let _ = AppLayout::compute(Rect::new(0, 0, width, height));
            }
        }
    }

    #[test]
    fn centering_is_symmetric() {
        let area = Rect::new(0, 0, 100, 50);
        let centred = center(area, Constraint::Length(40), Constraint::Length(10));

        assert_eq!(centred.left(), area.width - centred.right());
        assert_eq!(centred.top(), area.height - centred.bottom());
    }
}
