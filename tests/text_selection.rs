use ratatui::{
    buffer::Buffer,
    layout::{Position, Rect},
    style::Modifier,
};
use rstop::app::text_selection::{TextRegion, TextSelection};

fn region() -> TextRegion {
    let area = Rect::new(3, 2, 18, 3);
    let mut buffer = Buffer::empty(area);
    buffer.set_string(3, 2, "甲e\u{301}乙👩‍💻xyz", ratatui::style::Style::default());
    buffer.set_string(3, 3, "next line", ratatui::style::Style::default());
    TextRegion::capture("test".into(), (0, 0), area, &buffer)
}

#[test]
fn reverse_drag_and_partial_wide_graphemes_copy_without_padding() {
    let region = region();
    let mut selection = TextSelection::default();
    selection.begin(region.clone(), Position::new(4, 2)); // second cell of 甲
    selection.extend(Position::new(9, 2), true); // second cell of emoji
    assert_eq!(selection.selected_text().as_deref(), Some("甲e\u{301}乙👩‍💻"));
    let mut frame = region.buffer.clone();
    selection.render(&mut frame, std::slice::from_ref(&region));
    assert!(frame[(3, 2)].modifier.contains(Modifier::REVERSED));
    assert!(frame[(9, 2)].modifier.contains(Modifier::REVERSED));

    selection.begin(region, Position::new(6, 3));
    selection.extend(Position::new(3, 2), true);
    assert_eq!(
        selection.selected_text().as_deref(),
        Some("甲e\u{301}乙👩‍💻xyz\nnext")
    );
}

#[test]
fn refresh_preserves_selected_sample_and_geometry_changes_clear_it() {
    let region = region();
    let mut selection = TextSelection::default();
    selection.begin(region.clone(), Position::new(3, 3));
    selection.extend(Position::new(6, 3), true);
    let mut updated = region.clone();
    updated
        .buffer
        .set_string(3, 3, "changed", ratatui::style::Style::default());
    let mut frame = updated.buffer.clone();
    selection.render(&mut frame, &[updated.clone()]);
    assert_eq!(frame[(3, 3)].symbol(), "n");
    assert_eq!(selection.selected_text().as_deref(), Some("next"));
    updated.offset.0 = 1;
    selection.render(&mut frame, &[updated]);
    assert!(selection.selected_text().is_none());

    selection.begin(region.clone(), Position::new(3, 3));
    selection.extend(Position::new(6, 3), true);
    selection.render(&mut frame, &[]);
    assert!(selection.selected_text().is_none());
}
