//! Dev-build overlay: frames per second, frame interval and the newest log lines.
use crate::{controls::PlannerControl, debug_log, theme};
use gpui_kit::{component::Sizable, component::button::Button, prelude::*, *};
use std::{
    cell::RefCell,
    collections::VecDeque,
    rc::Rc,
    time::{Duration, Instant},
};

const WINDOW: Duration = Duration::from_secs(1);
const REFRESH: Duration = Duration::from_millis(500);

#[derive(Default)]
pub struct FrameStats {
    draws: VecDeque<Instant>,
}

impl FrameStats {
    pub fn record(&mut self, now: Instant) {
        self.draws.push_back(now);
        // Keep two so the interval survives an idle second.
        while self.draws.len() > 2 && now.duration_since(self.draws[0]) > WINDOW {
            self.draws.pop_front();
        }
    }

    pub fn fps(&self, now: Instant) -> usize {
        self.draws
            .iter()
            .filter(|at| now.duration_since(**at) <= WINDOW)
            .count()
    }

    pub fn frame_interval(&self) -> Option<Duration> {
        let n = self.draws.len();
        (n >= 2).then(|| self.draws[n - 1].duration_since(self.draws[n - 2]))
    }
}

pub struct DebugOverlay {
    visible: bool,
    stats: Rc<RefCell<FrameStats>>,
    ticker: Option<Task<()>>,
}

impl DebugOverlay {
    pub fn new(visible: bool, cx: &mut Context<Self>) -> Self {
        let mut overlay = Self {
            visible: false,
            stats: Rc::default(),
            ticker: None,
        };
        if visible {
            overlay.toggle(cx);
        }
        overlay
    }

    pub fn toggle(&mut self, cx: &mut Context<Self>) {
        self.visible = !self.visible;
        self.ticker = self.visible.then(|| {
            cx.spawn(async move |this, cx| {
                loop {
                    cx.background_executor().timer(REFRESH).await;
                    if this.update(cx, |_, cx| cx.notify()).is_err() {
                        break;
                    }
                }
            })
        });
        cx.notify();
    }
}

fn line_color(line: &str, p: &theme::TooltipTheme) -> Hsla {
    if line.starts_with("[ERROR") {
        p.negative
    } else if line.starts_with("[WARN") {
        p.accent
    } else {
        p.muted
    }
}

impl Render for DebugOverlay {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.visible {
            return div();
        }
        let p = cx.global::<theme::TooltipTheme>();
        let now = Instant::now();
        let stats = self.stats.borrow();
        let fps = stats.fps(now);
        let frame = stats
            .frame_interval()
            .map(|d| format!("{:.1} ms", d.as_secs_f64() * 1000.))
            .unwrap_or_else(|| "–".into());
        drop(stats);
        let viewport = window.viewport_size();
        let summary = format!(
            "{fps} fps · {frame} · {}×{} @{:.1}x",
            f32::from(viewport.width).round(),
            f32::from(viewport.height).round(),
            window.scale_factor()
        );
        let lines = debug_log::lines();
        let copy_text = lines.join("\n");
        let counter = self.stats.clone();
        div()
            .absolute()
            .right(px(12.))
            .bottom(px(40.))
            .w(px(360.))
            .max_h(viewport.height * 0.4)
            .occlude()
            .flex()
            .flex_col()
            .bg(p.panel.opacity(0.92))
            .border_1()
            .border_color(p.border)
            .rounded_sm()
            .font_family(theme::MONO_FONT_FAMILY)
            .text_size(px(10.))
            .text_color(p.text)
            .child(canvas(
                |_, _, _| (),
                move |_, _, _, _| counter.borrow_mut().record(Instant::now()),
            ))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_2()
                    .py_1()
                    .border_b_1()
                    .border_color(p.border)
                    .child(div().text_color(p.accent).child("DEBUG"))
                    .child(div().flex_1().min_w_0().truncate().child(summary))
                    .child(
                        Button::new("debug-copy")
                            .planner_style(cx)
                            .small()
                            .label("Copy logs")
                            .on_click(move |_, _, cx| {
                                cx.write_to_clipboard(ClipboardItem::new_string(copy_text.clone()))
                            }),
                    )
                    .child(
                        Button::new("debug-close")
                            .planner_style(cx)
                            .small()
                            .label("×")
                            .on_click(cx.listener(|this, _, _, cx| this.toggle(cx))),
                    ),
            )
            .child(
                div()
                    .id("debug-log")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .px_2()
                    .py_1()
                    .children(
                        lines
                            .iter()
                            .rev()
                            .map(|line| div().text_color(line_color(line, p)).child(line.clone())),
                    ),
            )
            .child(
                div()
                    .px_2()
                    .py_0p5()
                    .border_t_1()
                    .border_color(p.border)
                    .text_color(p.faint)
                    .child("newest first · overlay refresh 2/s · cmd-shift-d"),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[::core::prelude::v1::test]
    fn fps_counts_only_draws_inside_the_last_second() {
        let start = Instant::now();
        let mut stats = FrameStats::default();
        for ms in [0, 100, 200, 1500, 1600] {
            stats.record(start + Duration::from_millis(ms));
        }
        let now = start + Duration::from_millis(1600);
        assert_eq!(stats.fps(now), 2);
        assert_eq!(stats.frame_interval(), Some(Duration::from_millis(100)));
    }

    #[::core::prelude::v1::test]
    fn interval_survives_an_idle_second() {
        let start = Instant::now();
        let mut stats = FrameStats::default();
        stats.record(start);
        stats.record(start + Duration::from_millis(16));
        stats.record(start + Duration::from_secs(5));
        assert_eq!(stats.fps(start + Duration::from_secs(5)), 1);
        assert_eq!(
            stats.frame_interval(),
            Some(Duration::from_millis(5000 - 16))
        );
    }
}
