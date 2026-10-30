use super::Palette;
use crate::model::CoreTime;
use ratatui::prelude::*;

pub(super) const CPU_GLYPHS: [&str; 5] = ["▒", "█", "▓", "▥", "░"];

pub(super) fn cpu_time_colors(p: Palette) -> [Color; 5] {
    [
        p.cpu_nice,
        p.cpu_user,
        p.cpu_system,
        p.cpu_steal,
        p.cpu_wait,
    ]
}

pub(super) fn valid(times: CoreTime) -> bool {
    [
        times.nice,
        times.user,
        times.system,
        times.steal,
        times.iowait,
    ]
    .iter()
    .all(|v| v.is_finite() && *v >= 0.)
}

pub(super) fn time_track(
    value: f32,
    width: usize,
    glyph: &str,
    color: Color,
    p: Palette,
) -> Vec<Span<'static>> {
    let filled = (value.clamp(0., 100.) as f64 * width as f64 / 100.).round() as usize;
    vec![
        Span::styled(glyph.repeat(filled), Style::new().fg(color)),
        Span::styled("┄".repeat(width - filled), Style::new().fg(p.border)),
    ]
}

pub(super) fn core_bar(busy: f32, times: CoreTime, width: usize, p: Palette) -> Vec<Span<'static>> {
    let colors = cpu_time_colors(p);
    let weights = [times.nice, times.user, times.system, times.steal]
        .map(|v| if v.is_finite() { v.max(0.) as f64 } else { 0. });
    let busy = (busy.clamp(0., 100.) as f64 * width as f64 / 100.).round() as usize;
    let total: f64 = weights.iter().sum();
    let mut cumulative = 0.;
    let mut filled = 0;
    let mut spans = vec![];
    for (index, weight) in weights.iter().enumerate() {
        cumulative += weight;
        let end = if total > 0. {
            (busy as f64 * cumulative / total).round() as usize
        } else {
            0
        }
        .min(busy);
        spans.push(Span::styled(
            CPU_GLYPHS[index].repeat(end.saturating_sub(filled)),
            Style::new().fg(colors[index]),
        ));
        filled = end;
    }
    let wait = ((times.iowait as f64 * width as f64 / 100.).round() as usize)
        .min(width.saturating_sub(filled));
    spans.push(Span::styled(
        CPU_GLYPHS[4].repeat(wait),
        Style::new().fg(colors[4]),
    ));
    spans.push(Span::styled(
        "┄".repeat(width - filled - wait),
        Style::new().fg(p.border),
    ));
    spans
}

pub(super) fn core_header(
    times: Option<CoreTime>,
    has_steal: bool,
    shown: usize,
    total: usize,
    width: usize,
    p: Palette,
) -> Line<'static> {
    let suffix = if shown < total {
        format!(" · {shown}/{total}")
    } else {
        String::new()
    };
    let heading = Span::styled("Per-core usage", Style::new().fg(p.muted));
    let colors = cpu_time_colors(p);
    if let Some(times) = times.filter(|t| valid(*t)) {
        let values = [
            times.nice,
            times.user,
            times.system,
            times.steal,
            times.iowait,
        ];
        for show_values in [true, false] {
            for show_steal in [true, false] {
                if !show_steal && has_steal {
                    continue;
                }
                let mut spans = vec![heading.clone()];
                for (index, label) in ["low", "user", "sys", "steal", "wait"].iter().enumerate() {
                    if index == 3 && !show_steal {
                        continue;
                    }
                    let glyph = CPU_GLYPHS[index];
                    spans.push(Span::styled(
                        if show_values {
                            format!("  {glyph} {label} {:.1}%", values[index])
                        } else {
                            format!("  {glyph} {label}")
                        },
                        Style::new().fg(colors[index]),
                    ));
                }
                spans.push(Span::styled(suffix.clone(), Style::new().fg(p.muted)));
                let line = Line::from(spans);
                if line.width() <= width {
                    return line;
                }
            }
        }
    }
    Line::from(vec![
        heading,
        Span::styled(suffix, Style::new().fg(p.muted)),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        app::{App, View},
        config::Config,
        model::Snapshot,
    };
    use ratatui::{Terminal, backend::TestBackend, buffer::Buffer, widgets::Paragraph};

    fn rgb(color: Color) -> [f64; 3] {
        let Color::Rgb(r, g, b) = color else {
            panic!("CPU colors must be RGB")
        };
        [r as f64, g as f64, b as f64]
    }
    fn luminance(color: Color) -> f64 {
        let [r, g, b] = rgb(color).map(|v| {
            let v = v / 255.;
            if v <= 0.04045 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }
        });
        0.2126 * r + 0.7152 * g + 0.0722 * b
    }
    fn draw(spans: Vec<Span<'static>>, width: u16) -> Buffer {
        let mut terminal = Terminal::new(TestBackend::new(width, 1)).unwrap();
        terminal
            .draw(|f| f.render_widget(Paragraph::new(Line::from(spans)), f.area()))
            .unwrap();
        terminal.backend().buffer().clone()
    }
    fn sample_times() -> CoreTime {
        CoreTime {
            nice: 10.,
            user: 30.,
            system: 20.,
            steal: 10.,
            iowait: 20.,
        }
    }

    #[test]
    fn htop_segment_order_and_scaled_widths_preserve_busy_then_idle_wait() {
        let p = super::super::palette("midnight");
        let times = sample_times();
        let buffer = draw(core_bar(70., times, 20, p), 20);
        let glyphs: String = buffer.content.iter().map(|c| c.symbol()).collect();
        assert_eq!(glyphs, "▒▒██████▓▓▓▓▥▥░░░░┄┄");
        for width in 0..=80 {
            for busy in [0., 0.01, 2.5, 45.3, 70., 90., 100.] {
                for wait in [0., 20., 200.] {
                    let spans = core_bar(
                        busy,
                        CoreTime {
                            iowait: wait,
                            ..times
                        },
                        width,
                        p,
                    );
                    assert_eq!(Line::from(spans.clone()).width(), width);
                    let filled: usize = spans[..4].iter().map(Span::width).sum();
                    assert_eq!(filled, (busy as f64 * width as f64 / 100.).round() as usize);
                    if busy as f64 * width as f64 / 100. >= 0.5 {
                        assert!(filled > 0);
                    }
                }
            }
        }
        for invalid in [
            CoreTime {
                nice: f32::NAN,
                ..times
            },
            CoreTime {
                steal: -1.,
                ..times
            },
        ] {
            assert!(!valid(invalid));
            assert_eq!(Line::from(core_bar(90., invalid, 10, p)).width(), 10);
        }
    }

    #[test]
    fn legend_degrades_values_then_labels_and_only_omits_unused_steal() {
        let p = super::super::palette("midnight");
        let times = Some(CoreTime {
            nice: 1.2,
            user: 21.6,
            system: 0.8,
            steal: 0.,
            iowait: 0.2,
        });
        let full = "Per-core usage  ▒ low 1.2%  █ user 21.6%  ▓ sys 0.8%  ▥ steal 0.0%  ░ wait 0.2% · 8/64";
        let no_steal = "Per-core usage  ▒ low 1.2%  █ user 21.6%  ▓ sys 0.8%  ░ wait 0.2% · 8/64";
        let labels = "Per-core usage  ▒ low  █ user  ▓ sys  ▥ steal  ░ wait · 8/64";
        let short_labels = "Per-core usage  ▒ low  █ user  ▓ sys  ░ wait · 8/64";
        for expected in [
            full,
            no_steal,
            labels,
            short_labels,
            "Per-core usage · 8/64",
        ] {
            let width = expected.chars().count();
            let line = core_header(times, false, 8, 64, width, p);
            let buffer = draw(line.spans, width as u16);
            let text: String = buffer.content.iter().map(|c| c.symbol()).collect();
            assert_eq!(text.trim_end(), expected);
        }
        assert!(
            core_header(times, true, 8, 64, no_steal.chars().count(), p)
                .to_string()
                .contains("steal")
        );
        assert_eq!(
            core_header(None, false, 8, 8, 100, p).to_string(),
            "Per-core usage"
        );
    }

    #[test]
    fn all_themes_draw_five_distinct_high_contrast_htop_colors_on_both_pages() {
        for theme in crate::config::THEMES {
            let p = super::super::palette(theme);
            let colors = cpu_time_colors(p);
            let contrasts = colors.map(|color| {
                let (a, b) = (luminance(color), luminance(p.panel));
                (a.max(b) + 0.05) / (a.min(b) + 0.05)
            });
            let mut distances = vec![];
            for a in 0..5 {
                for b in a + 1..5 {
                    distances.push(
                        rgb(colors[a])
                            .into_iter()
                            .zip(rgb(colors[b]))
                            .map(|(a, b)| (a - b).powi(2))
                            .sum::<f64>()
                            .sqrt(),
                    );
                }
            }
            // Keep the existing RGB distance floor for all ten pairs, including
            // gray; separate glyphs preserve meaning without hue information.
            assert!(contrasts.iter().all(|v| *v >= 3.), "{theme}: {contrasts:?}");
            assert!(
                distances.iter().all(|v| *v >= 60.),
                "{theme}: {distances:?}"
            );
            let [nice, user, system, steal, wait] = colors.map(rgb);
            assert!(nice[2] > nice[0] && nice[2] > nice[1]);
            assert!(user[1] > user[0] && user[1] > user[2]);
            assert!(system[0] > system[1] && system[0] > system[2]);
            assert!(steal[0] < steal[1] && steal[0] < steal[2]);
            assert_eq!(wait, [wait[0]; 3]);
            println!(
                "{theme}: {colors:?}; contrast {contrasts:.2?}; min RGB distance {:.2}",
                distances.iter().copied().fold(f64::INFINITY, f64::min)
            );
            let mut app = App::new(Config {
                theme: theme.to_string(),
                ..Default::default()
            });
            app.config.display.overview_panel = "cpu".into();
            app.ingest(Snapshot {
                cores: vec![70.],
                core_times: vec![sample_times()],
                cpu_breakdown: [
                    ("nice".into(), 10.),
                    ("user".into(), 30.),
                    ("system".into(), 15.),
                    ("irq".into(), 3.),
                    ("softirq".into(), 2.),
                    ("steal".into(), 10.),
                    ("iowait".into(), 20.),
                ]
                .into(),
                ..Default::default()
            });
            for view in [View::Overview, View::System] {
                app.set_view(view);
                let mut terminal = Terminal::new(TestBackend::new(300, 80)).unwrap();
                terminal.draw(|f| super::super::draw(f, &mut app)).unwrap();
                let buffer = terminal.backend().buffer();
                for (index, label) in ["nice", "user", "system", "steal", "iowait"]
                    .iter()
                    .enumerate()
                {
                    if view == View::Overview {
                        assert!(buffer.content.iter().filter(|c| c.symbol() == CPU_GLYPHS[index] && c.fg == colors[index]).count() >= 2, "{theme} {label}");
                    } else {
                        let row = buffer
                            .content
                            .chunks(300)
                            .find(|row| {
                                row.iter()
                                    .map(|c| c.symbol())
                                    .collect::<String>()
                                    .contains(&format!(" {label:<7}"))
                            })
                            .unwrap();
                        assert!(
                            row.iter()
                                .any(|c| c.symbol() == CPU_GLYPHS[index] && c.fg == colors[index]),
                            "{theme} {label}"
                        );
                        if *label == "system" {
                            assert!(
                                row.iter()
                                    .map(|c| c.symbol())
                                    .collect::<String>()
                                    .contains("20.0%")
                            );
                        }
                    }
                }
            }
        }
    }
}
