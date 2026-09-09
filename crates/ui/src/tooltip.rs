//! Application tooltips: shown at once, anchored to the live cursor, kept open
//! through clicks and hidden the moment the trigger stops being hovered.
//! Own element instead of gpui's tooltip machinery, which either closes on
//! mouse down or keeps a leaving tooltip alive for 500ms.
use std::{panic::Location, rc::Rc, sync::Arc};

use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::{
    base::{Align, Placement, Positioner},
    prelude::*,
    *,
};

/// Above gpui-base popups (100) and dialogs (10 + layer).
const TOOLTIP_PRIORITY: usize = 200;

type Builder = Rc<dyn Fn(&mut Window, &mut App) -> AnyView>;

/// Zero-size absolute child covering its trigger; while hovered it defers the
/// tooltip view to the top of the window next to the cursor.
struct CursorTooltipAnchor {
    id: ElementId,
    build: Builder,
}

#[derive(Default)]
struct AnchorState {
    hitbox: Option<Hitbox>,
    view: Option<AnyView>,
}

impl IntoElement for CursorTooltipAnchor {
    type Element = Self;

    fn into_element(self) -> Self {
        self
    }
}

impl Element for CursorTooltipAnchor {
    type RequestLayoutState = ();
    type PrepaintState = (Hitbox, bool);

    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }

    fn source_location(&self) -> Option<&'static Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        let style = Style {
            position: Position::Absolute,
            inset: Edges::all(px(0.).into()),
            size: Size::full(),
            ..Style::default()
        };
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        id: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) -> (Hitbox, bool) {
        let id = id.expect("cursor tooltip anchor has an id");
        let hitbox = window.insert_hitbox(bounds, HitboxBehavior::Normal);
        window.with_element_state::<AnchorState, _>(id, |state, window| {
            let state = state.unwrap_or_default();
            // Hover comes from last frame's hitbox: this frame's hit test runs
            // after prepaint, and the listener below refreshes on every mouse move.
            let hovered = state
                .hitbox
                .as_ref()
                .is_some_and(|previous| previous.is_hovered(window));
            let view = hovered.then(|| state.view.unwrap_or_else(|| (self.build)(window, cx)));
            if let Some(view) = &view {
                let rem = window.rem_size();
                let mut popup =
                    Positioner::side(Bounds::new(window.mouse_position(), size(px(0.), px(0.))))
                        .placement(Placement::Right)
                        .align(Align::Start)
                        .offset(rem * (16. / 13.))
                        .margin(rem * (12. / 13.))
                        // Deferred draws inherit the trigger's text style; restore the app default.
                        .child(
                            div()
                                .font_family(crate::theme::FONT_FAMILY)
                                .font_weight(FontWeight::NORMAL)
                                .child(view.clone()),
                        )
                        .into_any_element();
                // Deferred draws only prepaint and paint, so lay the popup out here.
                popup.layout_as_root(AvailableSpace::min_size(), window, cx);
                window.defer_draw(popup, point(px(0.), px(0.)), TOOLTIP_PRIORITY, None);
            }
            (
                (hitbox.clone(), hovered),
                AnchorState {
                    hitbox: Some(hitbox),
                    view,
                },
            )
        })
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        (hitbox, hovered): &mut (Hitbox, bool),
        window: &mut Window,
        _: &mut App,
    ) {
        let (hitbox, hovered) = (hitbox.clone(), *hovered);
        window.on_mouse_event(move |_: &MouseMoveEvent, phase, window, _| {
            if phase.bubble() && (hovered || hitbox.is_hovered(window)) {
                window.refresh();
            }
        });
    }
}

/// Shared policy for buttons and custom tooltip triggers.
pub trait CursorTooltipExt: InteractiveElement + ParentElement + Sized {
    fn cursor_tooltip(self, text: impl Into<SharedString>) -> Self {
        let text = text.into();
        self.cursor_tooltip_view(move |window, cx| Tooltip::new(text.clone()).build(window, cx))
    }

    fn cursor_tooltip_view(
        mut self,
        build: impl Fn(&mut Window, &mut App) -> AnyView + 'static,
    ) -> Self {
        let id = match self.interactivity().element_id.clone() {
            Some(id) => ElementId::NamedChild(Arc::new(id), "cursor-tooltip".into()),
            None => ElementId::Name("cursor-tooltip".into()),
        };
        self.child(CursorTooltipAnchor {
            id,
            build: Rc::new(build),
        })
    }
}

impl<T: InteractiveElement + ParentElement> CursorTooltipExt for T {}
