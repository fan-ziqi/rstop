//! Translate visible content rows into Ratatui's scroll-position range.
use ratatui::widgets::ScrollbarState;

pub(super) fn state(content: usize, viewport: usize, position: usize) -> ScrollbarState {
    if viewport == 0 || content <= viewport {
        return ScrollbarState::new(0);
    }
    // Ratatui uses content_length - 1 as the maximum offset, adding the
    // viewport separately to compute the thumb's size and position.
    let maximum = content.saturating_sub(viewport);
    ScrollbarState::new(maximum.saturating_add(1))
        .position(position.min(maximum))
        .viewport_content_length(viewport)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{
        buffer::Buffer,
        layout::Rect,
        widgets::{Scrollbar, StatefulWidget},
    };

    #[test]
    fn thumb_touches_both_ends_for_short_and_long_content_and_varied_tracks() {
        for (content, viewport) in [(2, 1), (10, 9), (50, 3), (50, 24), (1000, 20)] {
            for track in [1, 2, 7, 20] {
                let area = Rect::new(0, 0, 1, track);
                for position in [0, content - viewport, usize::MAX] {
                    let mut buffer = Buffer::empty(area);
                    Scrollbar::default()
                        .begin_symbol(None)
                        .end_symbol(None)
                        .thumb_symbol("#")
                        .track_symbol(Some("."))
                        .render(area, &mut buffer, &mut state(content, viewport, position));
                    let edge = if position == 0 { 0 } else { track - 1 };
                    assert_eq!(
                        buffer[(0, edge)].symbol(),
                        "#",
                        "{content}/{viewport} track {track} position {position}"
                    );
                }
            }
        }
    }

    #[test]
    fn content_that_fits_or_has_no_viewport_does_not_show_a_scrollbar() {
        for (content, viewport) in [(0, 0), (0, 10), (5, 5), (5, 10), (10, 0)] {
            let area = Rect::new(0, 0, 1, 5);
            let mut buffer = Buffer::empty(area);
            Scrollbar::default().render(area, &mut buffer, &mut state(content, viewport, 0));
            assert!(buffer.content.iter().all(|cell| cell.symbol() == " "));
        }
    }
}
