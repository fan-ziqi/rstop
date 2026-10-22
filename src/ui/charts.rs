//! Shared, sample-aligned terminal charts for every telemetry page.
use super::{Palette, block, text};
use crate::{model::bytes, plot::Window};
use ratatui::{prelude::*, widgets::*};
use std::collections::VecDeque;

pub(super) type Series<'a> = (&'a str, &'a VecDeque<f64>, Color);
pub(super) type CpuSeries<'a> = (&'a str, Option<&'a VecDeque<f64>>);

#[derive(Clone, Copy)]
enum CpuDirection {
    Up,
    Down,
}

fn cpu_color(level: f64, palette: Palette) -> Color {
    let light = matches!(palette.panel, Color::Rgb(r, g, b) if u16::from(r) + u16::from(g) + u16::from(b) > 540);
    let (green, yellow, red) = if light {
        ((0, 125, 72), (162, 124, 0), (187, 39, 39))
    } else {
        // btop's default CPU gradient, with darker stops above for light panels.
        ((119, 202, 155), (203, 192, 108), (220, 76, 76))
    };
    let level = level.clamp(0., 1.);
    let (start, end, fraction) = if level <= 0.5 {
        (green, yellow, level * 2.)
    } else {
        (yellow, red, (level - 0.5) * 2.)
    };
    let blend =
        |a: u8, b: u8| (f64::from(a) + (f64::from(b) - f64::from(a)) * fraction).round() as u8;
    Color::Rgb(
        blend(start.0, end.0),
        blend(start.1, end.1),
        blend(start.2, end.2),
    )
}

pub(super) fn unit_label(unit: &str) -> &str {
    match unit {
        "B/s SI" => "B/s",
        "ibit/s" => "bit/s",
        _ => unit.trim(),
    }
}

/// Put the graph's unit before optional details, so compact titles retain it.
pub(super) fn title_with_unit(title: &str, unit: &str) -> String {
    let unit = unit_label(unit);
    let (name, details) = title.split_once(" · ").unwrap_or((title, ""));
    let name = if unit.is_empty() || name.contains(&format!("({unit})")) {
        name.to_owned()
    } else {
        format!("{name} ({unit})")
    };
    if details.is_empty() {
        name
    } else {
        format!("{name} · {details}")
    }
}

fn value_label(value: f64, unit: &str) -> String {
    if !value.is_finite() {
        return "—".into();
    }
    match unit {
        "B" => bytes(value),
        "B/s" => format!("{}/s", bytes(value)),
        "bit/s" | "ibit/s" | "B/s SI" => {
            let bits = unit != "B/s SI";
            crate::config::Display {
                network_bits: bits,
                network_binary: unit == "ibit/s",
                ..Default::default()
            }
            .rate(value / if bits { 8. } else { 1. })
        }
        "%" | "°C" | "°F" => format!("{value:.0}{unit}"),
        "" => format!("{value:.0}"),
        _ => format!("{value:.0} {}", unit_label(unit)),
    }
}

pub(super) struct Trend<'a> {
    pub series: &'a [Series<'a>],
    pub max: f64,
    pub window: Window<'a>,
    pub palette: Palette,
    pub axes: bool,
    pub unit: &'a str,
}

impl Trend<'_> {
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        self.render_scaled(frame, area, false);
    }
    pub fn render_scaled(&self, frame: &mut Frame, mut area: Rect, logarithmic: bool) {
        if self.axes && area.height >= 4 && area.width >= 14 {
            let label = |v| value_label(v, self.unit);
            let midpoint = if logarithmic {
                (self.max + 1.).sqrt() - 1.
            } else {
                self.max / 2.
            };
            let labels = [label(self.max), label(midpoint), label(0.)];
            let left = labels
                .iter()
                .map(|label| Line::raw(label.as_str()).width())
                .max()
                .unwrap_or(0) as u16
                + 1;
            for (value, y) in
                labels
                    .iter()
                    .zip([area.y, area.y + (area.height - 2) / 2, area.bottom() - 2])
            {
                frame.render_widget(
                    Paragraph::new(value.as_str()).style(Style::new().fg(self.palette.muted)),
                    Rect::new(area.x, y, left, 1),
                );
            }
            frame.render_widget(
                Paragraph::new(if let Some(time) = self.window.time {
                    format!("−{}", crate::app::commands::time_label(time.seconds))
                } else {
                    format!(
                        "{} samples",
                        self.window
                            .capacity(area.width.saturating_sub(left) as usize * 2)
                    )
                })
                .style(Style::new().fg(self.palette.muted)),
                Rect::new(
                    area.x + left,
                    area.bottom() - 1,
                    area.width.saturating_sub(left),
                    1,
                ),
            );
            frame.render_widget(
                Paragraph::new("now")
                    .alignment(Alignment::Right)
                    .style(Style::new().fg(self.palette.muted)),
                Rect::new(area.right() - 3, area.bottom() - 1, 3, 1),
            );
            area.x += left;
            area.width = area.width.saturating_sub(left);
            area.height -= 1;
        }
        frame.render_widget(Raster(self, logarithmic, None), area);
    }
}

struct Raster<'a>(&'a Trend<'a>, bool, Option<CpuDirection>);
impl Widget for Raster<'_> {
    fn render(self, area: Rect, buffer: &mut Buffer) {
        if area.is_empty() {
            return;
        }
        let width = area.width as usize * 2;
        let height = area.height as usize * 4;
        let mut bits = vec![0u8; area.width as usize * area.height as usize];
        let mut colors = vec![self.0.palette.panel; bits.len()];
        let inverted = matches!(self.2, Some(CpuDirection::Down));
        // One fixed color per terminal row prevents horizontal movement from
        // recoloring all the dots that share a braille character.
        let row_colors = self.2.map(|_| {
            (0..area.height)
                .map(|row| {
                    let distance = if inverted { row } else { area.height - 1 - row };
                    cpu_color(
                        f64::from(distance) / f64::from(area.height.saturating_sub(1).max(1)),
                        self.0.palette,
                    )
                })
                .collect::<Vec<_>>()
        });
        let max = self.0.max.max(1.);
        let y = |value: f64| {
            let value = value.clamp(0., max);
            let fraction = if self.1 {
                value.ln_1p() / max.ln_1p()
            } else {
                value / max
            };
            ((1. - fraction) * (height - 1) as f64).round() as i32
        };
        let mut dot = |x: i32, y: i32, color: Color| {
            if x < 0 || y < 0 || x >= width as i32 || y >= height as i32 {
                return;
            }
            let (x, y) = (x as usize, y as usize);
            let y = if inverted { height - 1 - y } else { y };
            let index = y / 4 * area.width as usize + x / 2;
            const DOTS: [[u8; 4]; 2] = [[0, 1, 2, 6], [3, 4, 5, 7]];
            bits[index] |= 1 << DOTS[x % 2][y % 4];
            colors[index] = row_colors.as_ref().map_or(color, |rows| rows[y / 4]);
        };
        for (_, values, color) in self.0.series {
            let columns = self.0.window.project(values, width);
            let mut previous = None;
            for (x, range) in columns.iter().enumerate() {
                let Some(range) = range else {
                    previous = None;
                    continue;
                };
                let x = x as i32;
                let end = y(range.last);
                if self.0.series.len() == 1 {
                    let top = if self.2.is_some() { y(range.high) } else { end };
                    for row in top..height as i32 {
                        dot(x, row, *color);
                    }
                }
                // Keep extrema when several samples share a column; never bridge missing values.
                for row in y(range.high)..=y(range.low) {
                    dot(x, row, *color);
                }
                if let Some((old_x, old_y)) = previous.filter(|_| !range.gap) {
                    line(old_x, old_y, x, end, |px, py| dot(px, py, *color));
                }
                dot(x, end, *color);
                previous = (!range.gap).then_some((x, end));
            }
        }
        for row in 0..area.height {
            for col in 0..area.width {
                let index = row as usize * area.width as usize + col as usize;
                if bits[index] != 0 {
                    buffer[(area.x + col, area.y + row)]
                        .set_char(char::from_u32(0x2800 + bits[index] as u32).unwrap())
                        .set_fg(colors[index]);
                }
            }
        }
    }
}

pub(super) fn cpu_chart(
    frame: &mut Frame,
    mut area: Rect,
    upper: CpuSeries<'_>,
    lower: Option<CpuSeries<'_>>,
    window: Window<'_>,
    p: Palette,
) {
    if area.height < 2 || area.width < 2 {
        return;
    }
    let title = match lower {
        Some(lower) if lower.0 != upper.0 => {
            format!("CPU history (%) · ↑ {} / ↓ {}", upper.0, lower.0)
        }
        _ if upper.0 != "total" => format!("CPU history (%) · {}", upper.0),
        _ => "CPU history (%)".into(),
    };
    text(
        frame,
        Rect::new(area.x, area.y, area.width, 1),
        Line::styled(title, Style::new().fg(p.muted)),
        p,
    );
    area.y += 1;
    area.height -= 1;
    if area.height >= 4 && area.width >= 14 {
        let axis = 5;
        let time = if let Some(time) = window.time {
            format!("−{}", crate::app::commands::time_label(time.seconds))
        } else {
            format!(
                "{} samples",
                window.capacity((area.width - axis) as usize * 2)
            )
        };
        text(
            frame,
            Rect::new(area.x + axis, area.bottom() - 1, area.width - axis, 1),
            Line::styled(time, Style::new().fg(p.muted)),
            p,
        );
        frame.render_widget(
            Paragraph::new("now")
                .alignment(Alignment::Right)
                .style(Style::new().fg(p.muted)),
            Rect::new(area.right() - 3, area.bottom() - 1, 3, 1),
        );
        area.height -= 1;
        let center = if lower.is_some() {
            area.height.div_ceil(2) - 1
        } else {
            area.height - 1
        };
        let mut labels = vec![("100%", 0), ("0%", center)];
        if lower.is_some() {
            labels.push(("100%", area.height - 1));
        }
        for (label, row) in labels {
            text(
                frame,
                Rect::new(area.x, area.y + row, axis, 1),
                Line::styled(label, Style::new().fg(p.muted)),
                p,
            );
        }
        area.x += axis;
        area.width -= axis;
    }
    let upper_height = if lower.is_some() {
        area.height.div_ceil(2)
    } else {
        area.height
    };
    for (metric, rect, direction) in [
        (
            Some(upper),
            Rect::new(area.x, area.y, area.width, upper_height),
            CpuDirection::Up,
        ),
        (
            lower,
            Rect::new(
                area.x,
                area.y + upper_height,
                area.width,
                area.height - upper_height,
            ),
            CpuDirection::Down,
        ),
    ] {
        let Some((name, values)) = metric else {
            continue;
        };
        if rect.is_empty() {
            continue;
        }
        if let Some(values) = values {
            let trend = Trend {
                series: &[(name, values, p.mint)],
                max: 100.,
                window,
                palette: p,
                axes: false,
                unit: "%",
            };
            frame.render_widget(Raster(&trend, false, Some(direction)), rect);
        } else {
            text(
                frame,
                rect,
                Line::styled(format!("{name} · unavailable"), Style::new().fg(p.muted)),
                p,
            );
        }
    }
}

fn line(mut x: i32, mut y: i32, end_x: i32, end_y: i32, mut dot: impl FnMut(i32, i32)) {
    let dx = (end_x - x).abs();
    let dy = -(end_y - y).abs();
    let sx = if x < end_x { 1 } else { -1 };
    let sy = if y < end_y { 1 } else { -1 };
    let mut error = dx + dy;
    loop {
        dot(x, y);
        if x == end_x && y == end_y {
            break;
        }
        let twice = 2 * error;
        if twice >= dy {
            error += dy;
            x += sx;
        }
        if twice <= dx {
            error += dx;
            y += sy;
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn chart(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    series: &[Series<'_>],
    max: f64,
    unit: &str,
    window: Window,
    p: Palette,
) {
    if area.height < 2 {
        return;
    }
    let mut legend = vec![Span::styled(
        title_with_unit(title, unit),
        Style::new().fg(p.muted),
    )];
    if series.len() > 1 {
        for (name, _, color) in series {
            legend.push(Span::styled(format!("  ━ {name}"), Style::new().fg(*color)));
        }
    }
    frame.render_widget(
        Paragraph::new(Line::from(legend)),
        Rect::new(area.x, area.y, area.width, 1),
    );
    Trend {
        series,
        max,
        window,
        palette: p,
        axes: true,
        unit,
    }
    .render(
        frame,
        Rect::new(area.x, area.y + 1, area.width, area.height - 1),
    );
}

#[allow(clippy::too_many_arguments)]
pub(super) fn gpu_plot(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    series: &[Series<'_>],
    max: f64,
    unit: &str,
    window: Window,
    p: Palette,
) {
    let b = block(format!(" {} ", title_with_unit(title, unit)), p, p.blue);
    let mut inner = b.inner(area);
    frame.render_widget(b, area);
    if !series
        .iter()
        .any(|(_, v, _)| v.iter().any(|v| v.is_finite()))
    {
        text(frame, inner, "No samples · counter unavailable", p);
        return;
    }
    if series.len() > 1 && inner.height >= 4 {
        let legend = Line::from(
            series
                .iter()
                .map(|(name, values, color)| {
                    Span::styled(
                        format!(
                            "{name} {}  ",
                            values
                                .back()
                                .map(|v| value_label(*v, unit))
                                .unwrap_or_else(|| "—".into())
                        ),
                        Style::new().fg(*color),
                    )
                })
                .collect::<Vec<_>>(),
        );
        frame.render_widget(
            Paragraph::new(legend),
            Rect::new(inner.x, inner.y, inner.width, 1),
        );
        inner.y += 1;
        inner.height -= 1;
    }
    Trend {
        series,
        max,
        window,
        palette: p,
        axes: true,
        unit,
    }
    .render(frame, inner);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn braille_dot(buffer: &Buffer, x: usize, y: usize) -> bool {
        const DOTS: [[u8; 4]; 2] = [[0, 1, 2, 6], [3, 4, 5, 7]];
        let code = buffer[(x as u16 / 2, y as u16 / 4)]
            .symbol()
            .chars()
            .next()
            .unwrap() as u32;
        (0x2800..=0x28ff).contains(&code) && (code - 0x2800) & (1 << DOTS[x % 2][y % 4]) != 0
    }

    #[test]
    fn cpu_fill_mirrors_geometry_and_keeps_row_colors_stable_while_scrolling() {
        for theme in crate::config::THEMES {
            for height in [2, 5, 8] {
                let area = Rect::new(0, 0, 32, height);
                let mut values = VecDeque::from([100., 20., 75., f64::NAN, 50., 95., 35., 100.]);
                let render = |values: &VecDeque<_>, end, direction| {
                    let mut buffer = Buffer::empty(area);
                    let trend = Trend {
                        series: &[("total", values, Color::Green)],
                        max: 100.,
                        window: Window {
                            samples: 64,
                            end,
                            time: None,
                        },
                        palette: super::super::palette(theme),
                        axes: false,
                        unit: "%",
                    };
                    Raster(&trend, false, Some(direction)).render(area, &mut buffer);
                    buffer
                };
                let upper = render(&values, 8, CpuDirection::Up);
                let lower = render(&values, 8, CpuDirection::Down);
                for y in 0..height as usize * 4 {
                    for x in 0..64 {
                        assert_eq!(
                            braille_dot(&upper, x, y),
                            braille_dot(&lower, x, height as usize * 4 - 1 - y)
                        );
                    }
                }
                let Color::Rgb(r, g, _) = upper[(31, 0)].fg else {
                    panic!("Missing red outer edge");
                };
                assert!(r > g);
                let Color::Rgb(r, g, _) = upper[(31, height - 1)].fg else {
                    panic!("Missing green center");
                };
                assert!(g > r);
                // Missing and uncollected time stays blank in both halves.
                for y in 0..height as usize * 4 {
                    assert!(!braille_dot(&upper, 59, y));
                    assert!(!braille_dot(&lower, 59, y));
                    assert!(!braille_dot(&upper, 0, y));
                }
                values.push_back(40.);
                let after = render(&values, 9, CpuDirection::Up);
                for y in 0..height as usize * 4 {
                    for x in 2..62 {
                        let existed = braille_dot(&upper, x + 1, y);
                        assert_eq!(existed, braille_dot(&after, x, y));
                        if existed {
                            assert_eq!(
                                upper[((x + 1) as u16 / 2, y as u16 / 4)].fg,
                                after[(x as u16 / 2, y as u16 / 4)].fg
                            );
                        }
                    }
                }
                for row in 0..height {
                    assert_eq!(upper[(31, row)].fg, lower[(31, height - 1 - row)].fg);
                }
            }
        }
    }

    #[test]
    fn cpu_chart_handles_compact_sizes_and_keeps_disabled_or_missing_metrics_explicit() {
        use ratatui::{Terminal, backend::TestBackend};
        let values = VecDeque::from([100.; 64]);
        let window = Window {
            samples: 64,
            end: 64,
            time: None,
        };
        let p = super::super::palette("midnight");
        for width in [2, 12, 32, 80] {
            for height in [2, 3, 6, 10, 20] {
                for lower in [None, Some(("total", Some(&values))), Some(("system", None))] {
                    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
                    terminal
                        .draw(|frame| {
                            cpu_chart(
                                frame,
                                frame.area(),
                                ("total", Some(&values)),
                                lower,
                                window,
                                p,
                            )
                        })
                        .unwrap();
                    let buffer = terminal.backend().buffer();
                    let text: String = buffer.content.iter().map(|cell| cell.symbol()).collect();
                    if width >= 32 && height >= 6 {
                        assert!(text.contains("CPU history (%)"));
                        assert!(text.contains("now"));
                        if lower.is_none() {
                            assert_eq!(text.matches("100%").count(), 1);
                        } else {
                            assert_eq!(text.matches("100%").count(), 2);
                        }
                        if matches!(lower, Some(("system", None))) {
                            assert!(text.contains("system · unavailable"));
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn compact_plots_keep_units_when_axes_do_not_fit_and_legends_match_the_scale() {
        use ratatui::{Terminal, backend::TestBackend};
        let values = VecDeque::from([1., 2., 3.]);
        for height in [3, 8] {
            for (unit, expected) in [
                ("%", "(%)"),
                ("B", "(B)"),
                ("B/s", "(B/s)"),
                ("B/s SI", "(B/s)"),
                ("bit/s", "(bit/s)"),
                ("ibit/s", "(bit/s)"),
                ("W", "(W)"),
                ("°C", "(°C)"),
                ("°F", "(°F)"),
                (" K", "(K)"),
                ("MHz", "(MHz)"),
                ("ms", "(ms)"),
            ] {
                let mut terminal = Terminal::new(TestBackend::new(80, height)).unwrap();
                for framed in [false, true] {
                    terminal
                        .draw(|frame| {
                            let series = [
                                ("read", &values, Color::Green),
                                ("write", &values, Color::Blue),
                            ];
                            if framed {
                                gpu_plot(
                                    frame,
                                    frame.area(),
                                    "Metric · current value",
                                    &series,
                                    100.,
                                    unit,
                                    Window {
                                        samples: 10,
                                        end: 3,
                                        time: None,
                                    },
                                    super::super::palette("midnight"),
                                );
                            } else {
                                chart(
                                    frame,
                                    frame.area(),
                                    "History",
                                    &series,
                                    100.,
                                    unit,
                                    Window {
                                        samples: 10,
                                        end: 3,
                                        time: None,
                                    },
                                    super::super::palette("midnight"),
                                );
                            }
                        })
                        .unwrap();
                    let text: String = terminal
                        .backend()
                        .buffer()
                        .content
                        .iter()
                        .map(|c| c.symbol())
                        .collect();
                    assert!(
                        text.contains(expected),
                        "{unit}, {height} rows, framed={framed}: {text}"
                    );
                }
            }
        }
        assert_eq!(value_label(1024., "B"), "1.0 KiB");
        assert_eq!(value_label(1024., "B/s"), "1.0 KiB/s");
        assert_eq!(value_label(1000., "B/s SI"), "1.0 kB/s");
        assert_eq!(value_label(8000., "bit/s"), "8.0 Kbit/s");
        assert_eq!(value_label(8192., "ibit/s"), "8.0 Kibit/s");
        assert_eq!(value_label(f64::NAN, "%"), "—");
    }

    fn pixels(values: &VecDeque<f64>, end: u64) -> Vec<Vec<bool>> {
        let area = Rect::new(0, 0, 32, 8);
        let mut buffer = Buffer::empty(area);
        let series = [("value", values, Color::Green)];
        let trend = Trend {
            series: &series,
            max: 100.,
            window: Window {
                samples: 64,
                end,
                time: None,
            },
            palette: super::super::palette("midnight"),
            axes: false,
            unit: "%",
        };
        Raster(&trend, false, None).render(area, &mut buffer);
        let mut pixels = vec![vec![false; 64]; 32];
        const DOTS: [[u8; 4]; 2] = [[0, 1, 2, 6], [3, 4, 5, 7]];
        for y in 0..32 {
            for x in 0..64 {
                let code = buffer[(x as u16 / 2, y as u16 / 4)]
                    .symbol()
                    .chars()
                    .next()
                    .unwrap() as u32;
                pixels[y][x] = (0x2800..=0x28ff).contains(&code)
                    && (code - 0x2800) & (1 << DOTS[x % 2][y % 4]) != 0;
            }
        }
        pixels
    }

    #[test]
    fn successive_frames_translate_old_geometry_by_one_dot_column() {
        for length in [3, 24, 64] {
            let mut values: VecDeque<_> = (0..length)
                .map(|i| [20., 30., 45., 65., 50., 35.][i % 6])
                .collect();
            let before = pixels(&values, length as u64);
            values.push_back(42.);
            if values.len() > 64 {
                values.pop_front();
            }
            let after = pixels(&values, length as u64 + 1);
            for y in 0..32 {
                for x in 2..62 {
                    assert_eq!(before[y][x + 1], after[y][x], "length={length} x={x} y={y}");
                }
            }
        }
    }
}
