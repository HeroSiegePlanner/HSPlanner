use std::{
    cell::RefCell,
    ops::Range,
    panic::Location,
    rc::Rc,
    sync::{Arc, LazyLock},
};

use gpui_kit::{
    App, AvailableSpace, Bounds, Element, ElementId, FontFeatures, GlobalElementId, Hsla,
    InspectorElementId, IntoElement, LayoutId, Pixels, ShapedLine, SharedString, Size, Style,
    TextStyle, Window, accesskit, point, px, size,
};
use unicode_segmentation::UnicodeSegmentation;

use crate::theme::TooltipTheme;

static GLOW_SAMPLES: LazyLock<Vec<(f32, f32, f32)>> = LazyLock::new(|| {
    let mut samples = Vec::new();
    for y in -4..=4 {
        for x in -4..=4 {
            let (x, y) = (x as f32 * 0.5, y as f32 * 0.5);
            samples.push((x, y, (-(x * x + y * y) / 2.).exp()));
        }
    }
    let total: f32 = samples.iter().map(|sample| sample.2).sum();
    samples.iter_mut().for_each(|sample| sample.2 /= total);
    samples
});

// Short tooltip labels need tracking and glow, which GPUI's Text does not expose.
pub struct TooltipText {
    id: ElementId,
    text: SharedString,
    tracking: f32,
    glow: Option<Hsla>,
    wrap: bool,
}

impl TooltipText {
    pub fn new(id: impl Into<ElementId>, text: impl Into<SharedString>, tracking: f32) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
            tracking: if matches!(
                crate::i18n::language(),
                crate::i18n::Language::Korean | crate::i18n::Language::Chinese
            ) {
                0.
            } else {
                tracking
            },
            glow: None,
            wrap: false,
        }
    }

    /// Wrap at word boundaries within the measured parent width. Overlong words
    /// break only between grapheme clusters; tracking and glow apply to each line.
    pub fn wrap(mut self) -> Self {
        self.wrap = true;
        self
    }

    pub fn glow(mut self, color: Option<Hsla>) -> Self {
        self.glow = color;
        self
    }
}

impl IntoElement for TooltipText {
    type Element = Self;

    fn into_element(self) -> Self {
        self
    }
}

fn cluster_offsets(
    text: &str,
    glyph_indices: impl Iterator<Item = usize>,
    spacing: Pixels,
) -> Vec<Pixels> {
    let starts: Vec<_> = text
        .grapheme_indices(true)
        .map(|(index, _)| index)
        .collect();
    glyph_indices
        .map(|index| {
            spacing
                * starts
                    .partition_point(|start| *start <= index)
                    .saturating_sub(1) as f32
        })
        .collect()
}

struct TrackedLine {
    line: ShapedLine,
    offsets: Vec<Pixels>,
    width: Pixels,
}

struct WrappedText {
    width: Option<Pixels>,
    lines: Vec<TrackedLine>,
    size: Size<Pixels>,
}

/// Layout state shared with GPUI's measured-layout callback.
pub struct TooltipTextLayout {
    single: TrackedLine,
    height: Pixels,
    color: Hsla,
    wrapped: Option<Rc<RefCell<Option<WrappedText>>>>,
}

fn tracked_line(
    text: SharedString,
    style: &TextStyle,
    font_size: Pixels,
    spacing: Pixels,
    window: &Window,
) -> TrackedLine {
    let line =
        window
            .text_system()
            .shape_line(text.clone(), font_size, &[style.to_run(text.len())], None);
    let offsets = cluster_offsets(
        &text,
        line.runs
            .iter()
            .flat_map(|run| run.glyphs.iter().map(|glyph| glyph.index)),
        spacing,
    );
    let width = line.width + spacing * text.graphemes(true).count() as f32;
    TrackedLine {
        line,
        offsets,
        width,
    }
}

// Keep shaping in GPUI: only choose byte ranges here, never split a Unicode
// grapheme or estimate glyph widths from character counts.
fn wrapped_ranges(
    text: &str,
    width: Option<Pixels>,
    mut measure: impl FnMut(&str) -> Pixels,
) -> Vec<Range<usize>> {
    let mut ranges = Vec::new();
    let mut base = 0;
    for paragraph in text.split('\n') {
        let Some(width) = width else {
            ranges.push(base..base + paragraph.len());
            base += paragraph.len() + 1;
            continue;
        };
        let ends: Vec<_> = paragraph
            .grapheme_indices(true)
            .map(|(index, cluster)| index + cluster.len())
            .collect();
        let word_ends: Vec<_> = paragraph
            .split_word_bound_indices()
            .map(|(index, word)| index + word.len())
            .filter(|end| ends.binary_search(end).is_ok())
            .collect();
        let first_line = ranges.len();
        let mut start = 0;
        while start < paragraph.len() {
            for cluster in paragraph[start..].graphemes(true) {
                if !cluster.chars().all(char::is_whitespace) {
                    break;
                }
                start += cluster.len();
            }
            if start == paragraph.len() {
                break;
            }
            let trim_end = |end: usize| {
                let mut trimmed = end;
                for cluster in paragraph[start..end].graphemes(true).rev() {
                    if !cluster.chars().all(char::is_whitespace) {
                        break;
                    }
                    trimmed -= cluster.len();
                }
                trimmed
            };
            let mut end = start;
            for &candidate in word_ends.iter().filter(|&&end| end > start) {
                if measure(&paragraph[start..trim_end(candidate)]) > width {
                    break;
                }
                end = candidate;
            }
            if end == start {
                // A word may exceed the card width. Keep at least one complete
                // grapheme even when that single cluster is itself wider.
                for &candidate in ends.iter().filter(|&&end| end > start) {
                    if end > start && measure(&paragraph[start..candidate]) > width {
                        break;
                    }
                    end = candidate;
                }
            }
            ranges.push(base + start..base + trim_end(end));
            start = end;
        }
        if ranges.len() == first_line {
            ranges.push(base..base);
        }
        base += paragraph.len() + 1;
    }
    ranges
}

impl Element for TooltipText {
    type RequestLayoutState = TooltipTextLayout;
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }

    fn source_location(&self) -> Option<&'static Location<'static>> {
        None
    }

    fn a11y_role(&self) -> Option<accesskit::Role> {
        Some(accesskit::Role::Label)
    }

    fn write_a11y_info(&self, node: &mut accesskit::Node) {
        node.set_value(self.text.to_string());
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        _: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut style = window.text_style();
        let font_size = style.font_size.to_pixels(window.rem_size());
        let height = style
            .line_height
            .to_pixels(font_size.into(), window.rem_size());
        if self.tracking != 0. {
            let mut features = style.font_features.tag_value_list().to_vec();
            features.retain(|(name, _)| name != "liga" && name != "clig");
            features.extend([("liga".into(), 0), ("clig".into(), 0)]);
            style.font_features = FontFeatures(Arc::new(features));
        }
        let spacing = font_size * self.tracking;
        let single = tracked_line(self.text.clone(), &style, font_size, spacing, window);
        let color = style.color;
        if !self.wrap {
            // Preserve the existing intrinsic single-line layout exactly.
            let width = single.width;
            let layout = window
                .request_measured_layout(Style::default(), move |_, _, _, _| size(width, height));
            return (
                layout,
                TooltipTextLayout {
                    single,
                    height,
                    color,
                    wrapped: None,
                },
            );
        }
        let wrapped: Rc<RefCell<Option<WrappedText>>> = Rc::new(RefCell::new(None));
        let measured = wrapped.clone();
        let text = self.text.clone();
        let layout =
            window.request_measured_layout(Style::default(), move |known, available, window, _| {
                let width = known.width.or(match available.width {
                    AvailableSpace::Definite(width) => Some(width.max(px(0.))),
                    AvailableSpace::MinContent => Some(px(0.)),
                    AvailableSpace::MaxContent => None,
                });
                let mut measured = measured.borrow_mut();
                if let Some(cached) = measured.as_ref()
                    && cached.width == width
                {
                    return cached.size;
                }
                let ranges = wrapped_ranges(&text, width, |part| {
                    tracked_line(part.to_owned().into(), &style, font_size, spacing, window).width
                });
                let lines: Vec<_> = ranges
                    .into_iter()
                    .map(|range| {
                        tracked_line(
                            text[range].to_owned().into(),
                            &style,
                            font_size,
                            spacing,
                            window,
                        )
                    })
                    .collect();
                let size = size(
                    lines
                        .iter()
                        .map(|line| line.width)
                        .fold(px(0.), Pixels::max),
                    height * lines.len() as f32,
                );
                *measured = Some(WrappedText { width, lines, size });
                size
            });
        (
            layout,
            TooltipTextLayout {
                single,
                height,
                color,
                wrapped: Some(wrapped),
            },
        )
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        _: &mut Window,
        _: &mut App,
    ) {
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        state: &mut Self::RequestLayoutState,
        _: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let wrapped = state.wrapped.as_ref().map(|wrapped| wrapped.borrow());
        let lines = wrapped
            .as_deref()
            .and_then(Option::as_ref)
            .map_or(std::slice::from_ref(&state.single), |wrapped| {
                wrapped.lines.as_slice()
            });
        let mut passes = Vec::new();
        if let Some(glow) = self.glow {
            // Sample both CSS Gaussian shadows; glyph masks stay in GPUI's native cache.
            for &(blur, opacity) in &cx.global::<TooltipTheme>().title_shadows {
                let sigma = window.rem_size() * (blur / 13. / 2.);
                passes.extend(GLOW_SAMPLES.iter().map(|&(x, y, weight)| {
                    (
                        point(sigma * x, sigma * y),
                        glow.opacity(opacity * weight),
                        true,
                    )
                }));
            }
        }
        passes.push((point(px(0.), px(0.)), state.color, false));
        window.paint_layer(bounds, |window| {
            for (offset, color, shadow) in passes {
                for (line_index, tracked) in lines.iter().enumerate() {
                    let line = &tracked.line;
                    let baseline = bounds.origin
                        + point(
                            px(0.),
                            state.height * line_index as f32
                                + (state.height - line.ascent - line.descent) / 2.
                                + line.ascent,
                        );
                    for (ix, (run, glyph)) in line
                        .runs
                        .iter()
                        .flat_map(|run| run.glyphs.iter().map(move |glyph| (run, glyph)))
                        .enumerate()
                    {
                        let origin =
                            baseline + glyph.position + point(tracked.offsets[ix], px(0.)) + offset;
                        let result = if glyph.is_emoji {
                            if shadow {
                                continue;
                            }
                            window.paint_emoji(origin, run.font_id, glyph.id, line.font_size)
                        } else {
                            window.paint_glyph(origin, run.font_id, glyph.id, line.font_size, color)
                        };
                        if let Err(error) = result {
                            log::warn!("Tooltip text rendering failed: {error}");
                            return;
                        }
                    }
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wrapped_parts(text: &str, width: Option<f32>) -> Vec<&str> {
        wrapped_ranges(text, width.map(px), |part| {
            px(part.graphemes(true).count() as f32)
        })
        .into_iter()
        .map(|range| &text[range])
        .collect()
    }

    #[test]
    fn wrapping_prefers_words_and_breaks_an_overlong_word() {
        assert_eq!(wrapped_parts("Alpha Beta", Some(7.)), ["Alpha", "Beta"]);
        assert_eq!(wrapped_parts("Longword", Some(3.)), ["Lon", "gwo", "rd"]);
        assert_eq!(wrapped_parts("Alpha Beta", Some(10.)), ["Alpha Beta"]);
    }

    #[test]
    fn wrapping_preserves_combining_and_emoji_clusters_at_tiny_widths() {
        let text = "A\u{301}👩‍💻ż";
        assert_eq!(wrapped_parts(text, Some(1.)), ["A\u{301}", "👩‍💻", "ż"]);
        assert_eq!(wrapped_parts(text, Some(0.)), ["A\u{301}", "👩‍💻", "ż"]);
    }

    #[test]
    fn wrapping_handles_empty_paragraphs_and_unconstrained_measurement() {
        assert_eq!(wrapped_parts("", Some(10.)), [""]);
        assert_eq!(wrapped_parts("A\n\nB\n", Some(10.)), ["A", "", "B", ""]);
        assert_eq!(wrapped_parts("Alpha Beta", None), ["Alpha Beta"]);
    }

    #[test]
    fn tracking_keeps_combining_marks_and_emoji_together() {
        let text = "A\u{301}👩‍💻ż";
        let offsets = cluster_offsets(text, text.char_indices().map(|(index, _)| index), px(1.2));
        assert_eq!(
            offsets,
            [px(0.), px(0.), px(1.2), px(1.2), px(1.2), px(2.4)]
        );
        assert!(cluster_offsets("", std::iter::empty(), px(1.)).is_empty());
    }

    #[test]
    fn glow_preserves_opacity_and_has_no_directional_bias() {
        let total: f32 = GLOW_SAMPLES.iter().map(|sample| sample.2).sum();
        let x: f32 = GLOW_SAMPLES.iter().map(|sample| sample.0 * sample.2).sum();
        let y: f32 = GLOW_SAMPLES.iter().map(|sample| sample.1 * sample.2).sum();
        assert!((total - 1.).abs() < 0.00001);
        assert!(x.abs() < 0.00001 && y.abs() < 0.00001);
    }
}
