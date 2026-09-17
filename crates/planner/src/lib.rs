use hsplanner_ui::controls::PlannerControl;
use hsplanner_ui::i18n::{Locale, tr, trf};
use hsplanner_ui::tooltip::CursorTooltipExt;
mod build_panel;
mod build_session;
pub mod character;
pub mod config;
pub mod editor;
pub mod gear;
mod gear_sections;
mod gear_stash;
mod item_tooltip;
pub mod mercenary;
mod node_tooltip;
mod scene;
mod skill_details;
pub mod skills;
mod source_breakdown;
mod source_preview;
mod stat_labels;
pub mod stats;
pub mod stats_sidebar;
mod tree_chrome;
mod tree_jewelry;
mod tree_progression;
mod tree_suggest;
use hsplanner_ui::theme;
use hsplanner_ui::tooltip_text;
mod tree;

use build_session::{BuildSession, CalculationRequest, DocumentKey};
use gpui_kit::base::{Align, Disableable, Placement, Positioner};
use gpui_kit::component::{
    Sizable,
    button::Button,
    input::{Input, InputEvent, InputState},
};
use gpui_kit::{
    Bounds, Context, FocusHandle, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent,
    Pixels, Render, ScrollWheelEvent, Window, canvas, div, prelude::*, px, size,
};
use hsplanner_build::{BuildSnapshot, session::Session};

use node_tooltip::{NodeLines, NodeTooltip};
use scene::{PaintStats, Scene, Selection};
use std::{
    cell::Cell,
    collections::{HashMap, HashSet},
    rc::Rc,
    sync::Arc,
    time::Instant,
};
use tree::{Camera, TreeKind};

gpui_kit::actions!(
    tree_prototype,
    [ShowTooltipExample, ToggleTextEffects, ToggleInspectedNode]
);

struct MotionTest {
    start: Instant,
    previous: Instant,
    base: Camera,
    intervals: Vec<f64>,
    paint_times: Vec<f64>,
}

struct MotionResult {
    fps: f64,
    p95: f64,
    paint: f64,
    samples: usize,
}

impl MotionResult {
    fn summary(&self) -> String {
        trf(
            "tree.motion_result",
            &[
                ("fps", format!("{:.1}", self.fps)),
                ("p95", format!("{:.1}", self.p95)),
                ("paint", format!("{:.2}", self.paint)),
                ("samples", self.samples.to_string()),
            ],
        )
    }
}

#[derive(Clone, Copy)]
enum Command {
    Fit,
    Center,
    ZoomIn,
    ZoomOut,
    Reset,
    Motion,
    TooltipExample,
    TextEffects,
    ToggleNode,
    RetryCalculation,
}

pub struct TreeView {
    session: gpui_kit::Entity<Session>,
    subscriptions: Vec<gpui_kit::Subscription>,
    search: gpui_kit::Entity<InputState>,
    search_matches: Vec<usize>,
    search_index: usize,
    document: DocumentKey,
    scene: Rc<Scene>,
    camera: Camera,
    viewport: Rc<Cell<Bounds<Pixels>>>,
    paint_stats: Rc<Cell<PaintStats>>,
    node_lines: HashMap<usize, NodeLines>,
    example_ix: usize,
    example: Option<usize>,
    text_effects: bool,
    build: BuildSession,
    build_input: Arc<BuildSnapshot>,
    active: bool,
    fit_on_first_layout: bool,
    selected: HashSet<usize>,
    progression: tree_progression::ProgressionPreview,
    progression_slider: gpui_kit::Entity<gpui_kit::component::slider::SliderState>,
    progression_focus: FocusHandle,
    suggest: tree_suggest::SuggestPanel,
    suggest_open: bool,
    suggest_slider: gpui_kit::Entity<gpui_kit::component::slider::SliderState>,
    hovered: Option<usize>,
    inspected: Option<usize>,
    summary_open: bool,
    summary_scroll: gpui_kit::ScrollHandle,
    ether_summary: Vec<hsplanner_engine::calc::planner::EtherSummary>,
    cursor: [f32; 2],
    drag: Option<([f32; 2], Camera)>,
    dragged: bool,
    focus: FocusHandle,
    motion_test: Option<MotionTest>,
    motion_result: Option<MotionResult>,
}

impl TreeView {
    pub fn new(
        session: gpui_kit::Entity<Session>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        Self::with_scene(session, Scene::load(), window, cx)
    }

    pub fn new_ether(
        session: gpui_kit::Entity<Session>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        Self::with_scene(
            session,
            Scene::from_graph(tree::Graph::load_ether()),
            window,
            cx,
        )
    }

    fn with_scene(
        session: gpui_kit::Entity<Session>,
        scene: Scene,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let scene = Rc::new(scene);
        let build_input = Arc::new(session.read(cx).snapshot().clone());
        let document = DocumentKey::from_session(session.read(cx));
        let selected = scene
            .graph
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, node)| {
                scene
                    .graph
                    .kind
                    .nodes(&build_input)
                    .contains(&(node.id as u32))
            })
            .map(|(ix, _)| ix)
            .collect();
        let viewport = [
            f32::from(window.viewport_size().width),
            f32::from(window.viewport_size().height),
        ];
        let camera = Camera::fit(scene.graph.bounds, viewport);
        let node_lines = if scene.graph.kind == TreeKind::Ether {
            scene
                .graph
                .info
                .iter()
                .map(|(id, info)| {
                    (
                        *id,
                        NodeLines {
                            parsed: info.l.clone(),
                            unsupported: vec![],
                        },
                    )
                })
                .collect()
        } else {
            node_tooltip::load_lines()
        };
        let placeholder = if scene.graph.kind == TreeKind::Ether {
            "tree.search_ether"
        } else {
            "tree.search_nodes"
        };
        let search = cx.new(|cx: &mut Context<InputState>| {
            cx.observe_global_in::<Locale>(window, move |input, window, cx| {
                input.set_placeholder(tr(placeholder), window, cx);
            })
            .detach();
            InputState::new(window, cx).placeholder(tr(placeholder))
        });
        let focus = cx.focus_handle();
        let summary_open = scene.graph.kind == TreeKind::Ether;
        let ether_summary =
            hsplanner_engine::calc::planner::summarize_ether(&build_input.allocated_ether_nodes);
        let progression = tree_progression::ProgressionPreview::new(
            &scene.graph,
            scene.graph.kind.nodes(&build_input),
        );
        let progression_slider = cx.new(|_| tree_progression::slider_state(progression.total()));
        let progression_focus = cx.focus_handle().tab_stop(true);
        let suggest_slider = cx.new(|_| tree_suggest::slider_state());
        let mut app = Self {
            session: session.clone(),
            subscriptions: vec![],
            search: search.clone(),
            search_matches: vec![],
            search_index: 0,
            document,
            scene,
            camera,
            focus,
            viewport: Rc::new(Cell::new(Bounds::default())),
            paint_stats: Rc::new(Cell::new(PaintStats::default())),
            node_lines,
            example_ix: 0,
            example: None,
            text_effects: true,
            build: BuildSession::default(),
            build_input,
            active: false,
            fit_on_first_layout: true,
            selected,
            progression,
            progression_slider: progression_slider.clone(),
            progression_focus,
            suggest: tree_suggest::SuggestPanel::new(),
            suggest_open: false,
            suggest_slider: suggest_slider.clone(),
            hovered: None,
            inspected: None,
            summary_open,
            summary_scroll: gpui_kit::ScrollHandle::new(),
            ether_summary,
            cursor: [0.0; 2],
            drag: None,
            dragged: false,
            motion_test: None,
            motion_result: None,
        };
        app.subscriptions
            .push(cx.observe(&session, |this, _, cx| this.sync_document(cx)));
        app.observe_progression(&progression_slider, cx);
        app.observe_suggest_budget(&suggest_slider, cx);
        app.subscriptions.push(cx.subscribe(
            &search,
            |this, _, event: &InputEvent, cx| match event {
                InputEvent::Change => {
                    let query = this.search.read(cx).value().to_lowercase();
                    this.search_matches = this.scene.graph.search(&query);
                    this.search_index = 0;
                    cx.notify();
                }
                InputEvent::PressEnter { .. } => this.next_search_match(cx),
                _ => {}
            },
        ));
        app.refresh_build(cx);
        app
    }

    pub fn set_active(&mut self, active: bool, cx: &mut Context<Self>) {
        if self.active == active {
            return;
        }
        self.active = active;
        self.hovered = None;
        self.example = None;
        self.refresh_build(cx);
        cx.notify();
    }

    fn next_search_match(&mut self, cx: &mut Context<Self>) {
        if self.search_matches.is_empty() {
            return;
        }
        let index = self.search_matches[self.search_index % self.search_matches.len()];
        self.search_index = (self.search_index + 1) % self.search_matches.len();
        let node = &self.scene.graph.nodes[index];
        let [width, height] = self.dimensions();
        self.camera = Camera::centered([node.x, node.y], [width, height], 1.2);
        self.inspected = Some(index);
        self.hovered = None;
        self.example = None;
        self.refresh_build(cx);
        cx.notify();
    }

    pub fn performance(&self) -> Option<Arc<hsplanner_engine::calc::planner::PlannerPerformance>> {
        self.build.performance()
    }

    fn sync_document(&mut self, cx: &mut Context<Self>) {
        let session = self.session.read(cx);
        let document = DocumentKey::from_session(session);
        if document == self.document {
            return;
        }
        self.build_input = Arc::new(session.snapshot().clone());
        self.document = document;
        if self.scene.graph.kind == TreeKind::Ether {
            self.ether_summary = hsplanner_engine::calc::planner::summarize_ether(
                &self.build_input.allocated_ether_nodes,
            );
        }
        self.selected = self
            .scene
            .graph
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, node)| {
                self.scene
                    .graph
                    .kind
                    .nodes(&self.build_input)
                    .contains(&(node.id as u32))
            })
            .map(|(ix, _)| ix)
            .collect();
        self.reset_progression(cx);
        self.suggest.clear();
        self.refresh_build(cx);
        cx.notify();
    }

    fn apply_node(&mut self, index: usize, cx: &mut Context<Self>) {
        if self.finish_progression(cx) {
            return;
        }
        let nodes = self
            .scene
            .graph
            .ordered_toggle(self.scene.graph.kind.nodes(&self.build_input), index);
        self.session.update(cx, |session, cx| {
            session.edit(|draft| self.scene.graph.kind.apply(&mut draft.snapshot, &nodes));
            cx.notify();
        });
        self.sync_document(cx);
    }

    fn save_selection(&mut self, cx: &mut Context<Self>) {
        let request = CalculationRequest::new(&self.scene.graph, &self.selected, None);
        self.session.update(cx, |session, cx| {
            session.edit(|draft| {
                self.scene
                    .graph
                    .kind
                    .apply(&mut draft.snapshot, &request.selected);
                if self.scene.graph.kind == TreeKind::Incarnation && request.selected.is_empty() {
                    draft.snapshot.tree_socketed.clear();
                }
            });
            cx.notify();
        });
        self.sync_document(cx);
    }

    fn refresh_build(&mut self, cx: &mut Context<Self>) {
        if !self.active && self.scene.graph.kind == TreeKind::Ether {
            return;
        }
        let mut request = CalculationRequest::new(
            &self.scene.graph,
            &self.selected,
            if self.active && !self.progression.is_preview() {
                self.hovered.or(self.example).or(self.inspected)
            } else {
                None
            },
        );
        request.document = self.document.clone();
        if let Some(preview) = &mut request.preview
            && let Some(index) = self
                .scene
                .graph
                .nodes
                .iter()
                .position(|node| node.id as u32 == preview.node_id)
        {
            preview.path = self
                .scene
                .graph
                .ordered_toggle(self.scene.graph.kind.nodes(&self.build_input), index);
        }
        if self.build.request != request {
            self.build.request = request;
            self.build.error = None;
        }
        if !self.build.in_flight && !self.build.is_current() && self.build.error.is_none() {
            self.start_calculation(cx);
        }
    }

    fn start_calculation(&mut self, cx: &mut Context<Self>) {
        self.build.in_flight = true;
        let request = self.build.request.clone();
        let job_request = request.clone();
        let input = self.build_input.clone();
        let cached = self
            .build
            .result
            .as_ref()
            .filter(|result| {
                result.request.document == request.document
                    && result.request.selected == request.selected
            })
            .map(|result| result.current.clone());
        let task = cx.background_spawn(async move {
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                build_session::calculate(&input, job_request, cached)
            }))
            .map_err(|_| "tree.calculate_failed".to_owned())
        });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |this, cx| {
                this.build.in_flight = false;
                let outdated = this.build.request != request;
                match result {
                    Ok(result)
                        if result.request.document == this.build.request.document
                            && result.request.selected == this.build.request.selected =>
                    {
                        this.build.result = Some(Rc::new(result));
                        this.build.error = None;
                    }
                    Err(error) if !outdated => this.build.error = Some(error),
                    _ => {}
                }
                if outdated {
                    this.start_calculation(cx);
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn toggle_inspected(&mut self, cx: &mut Context<Self>) {
        if self.finish_progression(cx) {
            return;
        }
        self.stop_motion();
        if let Some(ix) = self.hovered.or(self.example).or(self.inspected) {
            self.apply_node(ix, cx);
            self.inspected = Some(ix);
            self.refresh_build(cx);
            cx.notify();
        }
    }

    fn dimensions(&self) -> [f32; 2] {
        let size = self.viewport.get().size;
        [f32::from(size.width), f32::from(size.height)]
    }

    fn local(&self, position: gpui_kit::Point<Pixels>) -> [f32; 2] {
        let local = position - self.viewport.get().origin;
        [f32::from(local.x), f32::from(local.y)]
    }

    fn stop_motion(&mut self) {
        if let Some(test) = self.motion_test.take() {
            self.camera = test.base;
        }
    }

    fn command(&mut self, command: Command, cx: &mut Context<Self>) {
        if matches!(command, Command::ToggleNode) {
            self.toggle_inspected(cx);
            return;
        }
        if matches!(command, Command::RetryCalculation) {
            self.build.error = None;
            self.refresh_build(cx);
            cx.notify();
            return;
        }
        if matches!(command, Command::TextEffects) {
            self.text_effects = !self.text_effects;
            cx.notify();
            return;
        }
        if matches!(command, Command::TooltipExample) {
            self.show_tooltip_example(cx);
            return;
        }
        self.example = None;
        let was_running = self.motion_test.is_some();
        self.stop_motion();
        let [width, height] = self.dimensions();
        match command {
            Command::TooltipExample
            | Command::TextEffects
            | Command::ToggleNode
            | Command::RetryCalculation => unreachable!(),
            Command::Fit => self.camera = Camera::fit(self.scene.graph.bounds, [width, height]),
            Command::Center => {
                let center = if self.scene.graph.kind == TreeKind::Ether {
                    let root = &self.scene.graph.nodes[self.scene.graph.roots[0]];
                    [root.x, root.y]
                } else {
                    [1270.0, 1490.0]
                };
                self.camera = Camera::centered(center, [width, height], 0.8)
            }
            Command::ZoomIn => self.camera.zoom([width / 2.0, height / 2.0], 1.3),
            Command::ZoomOut => self.camera.zoom([width / 2.0, height / 2.0], 1.0 / 1.3),
            Command::Reset => {
                self.selected.clear();
                self.save_selection(cx);
                self.inspected = None;
            }
            Command::Motion if !was_running => {
                let now = Instant::now();
                self.motion_result = None;
                self.motion_test = Some(MotionTest {
                    start: now,
                    previous: now,
                    base: self.camera,
                    intervals: Vec::new(),
                    paint_times: Vec::new(),
                });
            }
            Command::Motion => {}
        }
        self.hovered = None;
        self.drag = None;
        self.refresh_build(cx);
        cx.notify();
    }

    fn animate(&mut self, window: &Window) {
        let [width, height] = self.dimensions();
        let Some(test) = &mut self.motion_test else {
            return;
        };
        let now = Instant::now();
        let elapsed = now.duration_since(test.start).as_secs_f32();
        if elapsed > 1.0 {
            test.intervals
                .push(now.duration_since(test.previous).as_secs_f64() * 1000.0);
            test.paint_times.push(self.paint_stats.get().milliseconds);
        }
        test.previous = now;
        if elapsed >= 9.0 {
            let mut test = self.motion_test.take().unwrap();
            self.camera = test.base;
            test.intervals.sort_by(f64::total_cmp);
            let mean = test.intervals.iter().sum::<f64>() / test.intervals.len().max(1) as f64;
            let p95 = test
                .intervals
                .get((test.intervals.len() as f64 * 0.95) as usize)
                .copied()
                .unwrap_or(0.0);
            let paint = test.paint_times.iter().sum::<f64>() / test.paint_times.len().max(1) as f64;
            let result = MotionResult {
                fps: 1000.0 / mean.max(0.001),
                p95,
                paint,
                samples: test.intervals.len(),
            };
            log::info!("Motion test: {}", result.summary().replace('\n', "; "));
            self.motion_result = Some(result);
        } else {
            self.camera = test.base;
            self.camera.zoom(
                [width / 2.0, height / 2.0],
                1.0 + 0.55 * (elapsed * 0.8).sin(),
            );
            self.camera.offset[0] += (elapsed * 1.1).sin() * width * 0.15;
            self.camera.offset[1] += (elapsed * 0.7).sin() * height * 0.15;
            window.request_animation_frame();
        }
    }

    fn mouse_down(&mut self, event: &MouseDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        self.stop_motion();
        window.focus(&self.focus, cx);
        self.drag = Some((self.local(event.position), self.camera));
        self.dragged = false;
        cx.notify();
    }

    fn mouse_move(&mut self, event: &MouseMoveEvent, _: &mut Window, cx: &mut Context<Self>) {
        let position = self.local(event.position);
        self.cursor = position;
        let had_example = self.example.take().is_some();
        if let Some((start, camera)) = self.drag {
            if event.pressed_button == Some(MouseButton::Left) {
                let delta = [position[0] - start[0], position[1] - start[1]];
                self.dragged |= delta[0].hypot(delta[1]) > 4.0;
                if self.dragged {
                    self.camera.offset = [camera.offset[0] + delta[0], camera.offset[1] + delta[1]];
                    self.hovered = None;
                    cx.notify();
                    return;
                }
            } else {
                self.drag = None;
            }
        }
        if self.motion_test.is_some() || self.progression.is_preview() {
            return;
        }
        let hovered = self.scene.graph.hit(self.camera, position);
        if had_example || self.hovered != hovered || hovered.is_some() {
            self.hovered = hovered;
            if hovered.is_some() {
                self.inspected = hovered;
            }
            self.refresh_build(cx);
            cx.notify();
        }
    }

    fn mouse_up(&mut self, event: &MouseUpEvent, _: &mut Window, cx: &mut Context<Self>) {
        if self.drag.take().is_none() {
            return;
        }
        if !self.dragged
            && let Some(index) = self
                .scene
                .graph
                .hit(self.camera, self.local(event.position))
        {
            if self.finish_progression(cx) {
                return;
            }
            self.apply_node(index, cx);
            self.inspected = Some(index);
            self.refresh_build(cx);
        }
        self.dragged = false;
        cx.notify();
    }

    fn scroll(&mut self, event: &ScrollWheelEvent, _: &mut Window, cx: &mut Context<Self>) {
        self.stop_motion();
        let delta = event.delta.pixel_delta(px(28.0));
        if event.modifiers.shift {
            self.camera.offset[0] += f32::from(delta.x) + f32::from(delta.y);
        } else {
            self.camera.zoom(
                self.local(event.position),
                (f32::from(delta.y) * 0.006).exp(),
            );
        }
        self.hovered = None;
        self.example = None;
        self.drag = None;
        cx.notify();
    }

    fn button(
        &self,
        label: &'static str,
        command: Command,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let id = match command {
            Command::Fit => "overview",
            Command::Center => "starting-nodes",
            Command::ZoomIn => "zoom-in",
            Command::ZoomOut => "zoom-out",
            Command::Reset => "clear-selection",
            Command::Motion => "motion-test",
            Command::TooltipExample => "tooltip-examples",
            Command::TextEffects => "text-effects",
            Command::ToggleNode => "toggle-inspected-node",
            Command::RetryCalculation => "retry-calculation",
        };
        hsplanner_ui::controls::planner_button(id, hsplanner_ui::controls::ButtonTone::Neutral, cx)
            .label(label)
            .disabled(matches!(command, Command::ToggleNode) && self.inspected.is_none())
            .small()
            .cursor_tooltip(match command {
                Command::TooltipExample => tr("tree.tooltip_example"),
                Command::TextEffects => tr("tree.text_effects"),
                Command::ToggleNode => tr("tree.apply_node_path"),
                Command::ZoomIn => tr("tree.zoom_in"),
                Command::ZoomOut => tr("tree.zoom_out"),
                _ => label,
            })
            .on_click(cx.listener(move |this, _, window, cx| {
                window.focus(&this.focus, cx);
                this.command(command, cx);
            }))
    }

    fn show_tooltip_example(&mut self, cx: &mut Context<Self>) {
        const EXAMPLES: &[usize] = &[0, 1025, 67, 5, 253, 853, 412, 90];
        self.stop_motion();
        let id = EXAMPLES[self.example_ix % EXAMPLES.len()];
        self.example_ix += 1;
        if let Some(ix) = self.scene.graph.nodes.iter().position(|node| node.id == id) {
            let node = &self.scene.graph.nodes[ix];
            let [width, height] = self.dimensions();
            self.camera = Camera::centered([node.x, node.y], [width, height], 1.2);
            self.cursor = [width / 2., height / 2.];
            self.example = Some(ix);
            self.inspected = Some(ix);
            self.drag = None;
            self.hovered = None;
            self.refresh_build(cx);
            cx.notify();
        }
    }
}

impl gpui_kit::Focusable for TreeView {
    fn focus_handle(&self, _: &gpui_kit::App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for TreeView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.fit_on_first_layout && self.dimensions()[0] > 0. {
            self.camera = Camera::fit(self.scene.graph.bounds, self.dimensions());
            self.fit_on_first_layout = false;
        }
        self.animate(window);
        let scene = self.scene.clone();
        let viewport = self.viewport.clone();
        let paint_stats = self.paint_stats.clone();
        let camera = self.camera;
        let progression_preview = self.progression.is_preview();
        let hovered = self
            .hovered
            .or(self.example)
            .filter(|_| !progression_preview);
        let selected = if progression_preview {
            self.progression.visible().clone()
        } else {
            self.selected.clone()
        };
        let progression_marker = self.progression.marker();
        let mut preview: HashSet<_> = hovered
            .filter(|id| !selected.contains(id))
            .map(|id| {
                let ids: HashSet<_> = self
                    .scene
                    .graph
                    .ordered_toggle(self.scene.graph.kind.nodes(&self.build_input), id)
                    .into_iter()
                    .collect();
                self.scene
                    .graph
                    .nodes
                    .iter()
                    .enumerate()
                    .filter(|(_, n)| ids.contains(&(n.id as u32)))
                    .map(|(i, _)| i)
                    .collect()
            })
            .unwrap_or_default();
        if self.suggest_open && !progression_preview {
            preview.extend(self.suggest.added.iter().copied());
        }
        let matches: HashSet<_> = self.search_matches.iter().copied().collect();
        let searching = !self.search.read(cx).value().trim().is_empty();
        let progression_node_ids: HashSet<_> = if progression_preview {
            selected
                .iter()
                .map(|&index| self.scene.graph.nodes[index].id)
                .collect()
        } else {
            HashSet::new()
        };
        let socketed: HashSet<_> = if self.scene.graph.kind == TreeKind::Incarnation {
            self.build_input
                .tree_socketed
                .keys()
                .map(|id| *id as usize)
                .filter(|id| !progression_preview || progression_node_ids.contains(id))
                .collect()
        } else {
            HashSet::new()
        };
        let tree_theme = self.tree_theme();
        let graph = div()
            .id("tree-canvas")
            .relative()
            .size_full()
            .overflow_hidden()
            .cursor(if self.drag.is_some() {
                gpui_kit::CursorStyle::ClosedHand
            } else {
                gpui_kit::CursorStyle::OpenHand
            })
            .child(
                canvas(
                    move |bounds, _, _| {
                        viewport.set(bounds);
                    },
                    move |bounds, _, window, _| {
                        let previous = paint_stats.get();
                        let current = scene.paint(
                            camera,
                            Selection {
                                allocated: &selected,
                                preview: &preview,
                                matches: &matches,
                                searching,
                                socketed: &socketed,
                                progression_marker,
                            },
                            tree_theme,
                            bounds,
                            window,
                        );
                        paint_stats.set(current);
                        if previous.visible != current.visible
                            || previous.image_errors != current.image_errors
                        {
                            window.request_animation_frame();
                        }
                    },
                )
                .size_full(),
            )
            .on_mouse_down(MouseButton::Left, cx.listener(Self::mouse_down))
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(|this, event: &MouseDownEvent, window, cx| {
                    if let Some(ix) = this
                        .scene
                        .graph
                        .hit(this.camera, this.local(event.position))
                    {
                        let id = this.scene.graph.nodes[ix].id as u32;
                        this.open_jewelry(id, window, cx);
                    }
                    cx.stop_propagation();
                }),
            )
            .on_mouse_move(cx.listener(Self::mouse_move))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::mouse_up))
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|this, _: &MouseUpEvent, _, cx| {
                    this.drag = None;
                    this.dragged = false;
                    cx.notify();
                }),
            )
            .on_hover(cx.listener(|this, over: &bool, _, cx| {
                if !over {
                    this.hovered = None;
                    cx.notify();
                }
            }))
            .on_scroll_wheel(cx.listener(Self::scroll));
        let tooltip = hovered.filter(|_| self.build.is_current()).map(|ix| {
            let node = &self.scene.graph.nodes[ix];
            let cursor = self.viewport.get().origin
                + gpui_kit::point(px(self.cursor[0]), px(self.cursor[1]));
            let offset = window.rem_size() * (16. / 13.);
            let margin = window.rem_size() * (12. / 13.);
            gpui_kit::deferred(
                Positioner::side(Bounds::new(
                    cursor + gpui_kit::point(px(0.), offset),
                    size(px(0.), px(0.)),
                ))
                .placement(Placement::Right)
                .align(Align::Start)
                .offset(offset)
                .margin(margin)
                .child(
                    NodeTooltip::new(
                        node,
                        self.scene.graph.info.get(&node.id),
                        self.node_lines.get(&node.id),
                    )
                    .effects(self.text_effects)
                    .socket(
                        self.build_input.tree_socketed.get(&(node.id as u32)),
                        self.selected.contains(&ix),
                    )
                    .performance(self.build.preview(), self.build.in_flight),
                ),
            )
            .with_priority(200)
        });
        div()
            .size_full()
            .flex()
            .flex_col()
            .relative()
            .bg(tree_theme.background())
            .text_color(cx.global::<theme::TooltipTheme>().text)
            .font_family(theme::FONT_FAMILY)
            .track_focus(&self.focus)
            .key_context("TreePrototype")
            .on_action(
                cx.listener(|this, _: &ShowTooltipExample, _, cx| this.show_tooltip_example(cx)),
            )
            .on_action(cx.listener(|this, _: &ToggleTextEffects, _, cx| {
                this.command(Command::TextEffects, cx)
            }))
            .on_action(
                cx.listener(|this, _: &ToggleInspectedNode, _, cx| this.toggle_inspected(cx)),
            )
            .on_key_down(
                cx.listener(|this, event: &gpui_kit::KeyDownEvent, window, cx| {
                    if event.keystroke.key == "escape" && (this.summary_open || this.suggest_open) {
                        this.summary_open = false;
                        if this.suggest_open {
                            this.suggest_open = false;
                            this.suggest.clear();
                        }
                        this.hovered = None;
                        this.example = None;
                        window.focus(&this.focus, cx);
                        cx.notify();
                        return;
                    }
                    if !this.focus.is_focused(window) {
                        return;
                    }
                    let command = match event.keystroke.key.as_str() {
                        "f" => Some(Command::Fit),
                        "c" => Some(Command::Center),
                        "enter" => Some(Command::ToggleNode),
                        "t" => Some(Command::TooltipExample),
                        "e" => Some(Command::TextEffects),
                        "+" | "=" => Some(Command::ZoomIn),
                        "-" => Some(Command::ZoomOut),
                        "escape" => {
                            this.stop_motion();
                            this.hovered = None;
                            this.example = None;
                            this.drag = None;
                            cx.notify();
                            None
                        }
                        _ => None,
                    };
                    if let Some(command) = command {
                        this.command(command, cx);
                    }
                }),
            )
            .child(graph)
            .child(self.toolbar(cx))
            .child(self.status_bar(cx))
            .child(self.progression_bar(window, cx))
            .when(self.suggest_open, |view| {
                view.child(
                    div()
                        .absolute()
                        .top_12()
                        .bottom_16()
                        .right_3()
                        .occlude()
                        .child(self.suggest_panel(window, cx)),
                )
            })
            .when(self.summary_open, |view| {
                view.child(
                    div()
                        .absolute()
                        .top(gpui_kit::rems(45. / 13.))
                        .right_3p5()
                        .bottom_16()
                        .child(self.ether_summary_panel(window, cx)),
                )
            })
            .children(tooltip)
    }
}
