//! Read-only source previews use the same item cards and tree renderer as the planner.
use crate::{
    item_tooltip,
    scene::{Scene, Selection},
    tree::{Camera, Graph},
};
use gpui_kit::{
    component::{
        button::{Button, ButtonVariants},
        popover::Popover,
    },
    prelude::*,
    *,
};
use hsplanner_build::BuildSnapshot;
use hsplanner_engine::calc::{
    data,
    stats::{SourceContribution, SourceType},
    types::EquippedItem,
};
use hsplanner_ui::theme::TreeTheme;
use std::{
    collections::HashSet,
    sync::{Arc, LazyLock},
};

static TREE: LazyLock<Arc<Scene>> = LazyLock::new(|| Arc::new(Scene::load()));

fn item_name(source: &SourceContribution) -> &str {
    if let Some(forge) = &source.forge {
        return &forge.item_name;
    }
    let label = source.label.trim();
    if let Some((_, tail)) = label.rsplit_once(" in ")
        && let Some((name, suffix)) = tail.rsplit_once(" #")
        && suffix.starts_with(|c: char| c.is_ascii_digit())
    {
        return name;
    }
    if let Some((_, name)) = label.rsplit_once('(')
        && let Some(name) = name.strip_suffix(')')
    {
        return name;
    }
    label
}

fn find_item<'a>(
    source: &SourceContribution,
    snapshot: &'a BuildSnapshot,
) -> Option<&'a EquippedItem> {
    if source.forge.is_none()
        && !matches!(source.source_type, SourceType::Item | SourceType::Socket)
    {
        return None;
    }
    let name = item_name(source);
    let mut matches = snapshot
        .inventory
        .values()
        .filter(|item| data::get_item(&item.base_id).is_some_and(|base| base.name == name));
    let item = matches.next()?;
    // Source labels contain no slot ID. Never present an arbitrary roll for duplicate names.
    if matches.next().is_some() {
        return None;
    }
    Some(item)
}

fn find_node(source: &SourceContribution, graph: &Graph) -> Option<usize> {
    if source.source_type != SourceType::Tree
        || !(source.label.starts_with("Tree:")
            || source.label.starts_with("Incarnation:")
            || source.label.contains("(Tree Socket #"))
    {
        return None;
    }
    if let Some((_, suffix)) = source.label.rsplit_once('#') {
        let digits: String = suffix.chars().take_while(char::is_ascii_digit).collect();
        let id = digits.parse::<usize>().ok()?;
        return graph.nodes.iter().position(|node| node.id == id);
    }
    let name = source
        .label
        .strip_prefix("Tree: ")
        .or_else(|| source.label.strip_prefix("Incarnation: "))?
        .split(['(', ':'])
        .next()?
        .trim();
    let mut matches = graph
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, node)| graph.info.get(&node.id).is_some_and(|info| info.t == name));
    let (index, _) = matches.next()?;
    matches.next().is_none().then_some(index)
}

pub(super) fn wrap(
    id: ElementId,
    source: &SourceContribution,
    snapshot: &BuildSnapshot,
    child: Div,
) -> AnyElement {
    if let Some(item) = find_item(source, snapshot) {
        let ids: Vec<String> = snapshot
            .inventory
            .values()
            .map(|item| item.base_id.clone())
            .collect();
        let hover_item = item.clone();
        let hover_ids = ids.clone();
        let trigger = Button::new(ElementId::NamedChild(
            Arc::new(id.clone()),
            "preview-button".into(),
        ))
        .ghost()
        .w_full()
        .h_auto()
        .p_0()
        .justify_start()
        .accessibility_label(format!("Preview {}", item_name(source)))
        .child(
            div()
                .id("source-item-hover")
                .w_full()
                .tooltip(move |_, cx| {
                    let base = data::get_item(&hover_item.base_id).expect("resolved equipped item");
                    cx.new(|_| {
                        item_tooltip::ItemTooltip::new(item_tooltip::build_model(
                            base,
                            Some(&hover_item),
                            &hover_ids,
                        ))
                    })
                    .into()
                })
                .child(child.w_full()),
        );
        let item = item.clone();
        return Popover::new(id)
            .appearance(false)
            .w_full()
            .trigger(trigger)
            .content(move |_, window, cx| {
                div()
                    .w(rems(28.))
                    .max_w(window.viewport_size().width * 0.9)
                    .child(item_tooltip::item_card(Some(&item), &ids, window, cx))
            })
            .into_any_element();
    }
    if source.source_type == SourceType::Tree {
        let scene = TREE.clone();
        if let Some(index) = find_node(source, &scene.graph) {
            let allocated: HashSet<_> = scene
                .graph
                .nodes
                .iter()
                .enumerate()
                .filter(|(_, node)| snapshot.allocated_tree_nodes.contains(&(node.id as u32)))
                .map(|(index, _)| index)
                .collect();
            let socketed: HashSet<_> = scene
                .graph
                .nodes
                .iter()
                .enumerate()
                .filter(|(_, node)| snapshot.tree_socketed.contains_key(&(node.id as u32)))
                .map(|(index, _)| index)
                .collect();
            let preview = NodePreview {
                scene,
                index,
                allocated,
                socketed,
            };
            let hover = preview.clone();
            let trigger = Button::new(ElementId::NamedChild(
                Arc::new(id.clone()),
                "preview-button".into(),
            ))
            .ghost()
            .w_full()
            .h_auto()
            .p_0()
            .justify_start()
            .accessibility_label(format!(
                "Preview tree node #{}",
                preview.scene.graph.nodes[index].id
            ))
            .child(
                div()
                    .id("source-node-hover")
                    .w_full()
                    .tooltip(move |_, cx| cx.new(|_| hover.clone()).into())
                    .child(child.w_full()),
            );
            return Popover::new(id)
                .appearance(false)
                .w_full()
                .trigger(trigger)
                .content(move |_, _, cx| preview.content(cx))
                .into_any_element();
        }
    }
    child.into_any_element()
}

#[derive(Clone)]
struct NodePreview {
    scene: Arc<Scene>,
    index: usize,
    allocated: HashSet<usize>,
    socketed: HashSet<usize>,
}
impl Render for NodePreview {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.content(cx)
    }
}
impl NodePreview {
    fn content(&self, cx: &App) -> Div {
        use gpui_kit::component::ActiveTheme;
        let node = &self.scene.graph.nodes[self.index];
        let info = self.scene.graph.info.get(&node.id);
        let scene = self.scene.clone();
        let center = [node.x, node.y];
        let allocated = self.allocated.clone();
        let socketed = self.socketed.clone();
        let index = self.index;
        div()
            .w_80()
            .bg(cx.theme().popover)
            .text_color(cx.theme().popover_foreground)
            .border_1()
            .border_color(cx.theme().border)
            .rounded(cx.theme().radius)
            .child(
                div()
                    .p_3()
                    .child(info.map_or_else(|| format!("Node #{}", node.id), |info| info.t.clone()))
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(format!("Incarnation Tree · #{}", node.id)),
                    ),
            )
            .child(
                canvas(
                    move |_, _, _| (),
                    move |bounds, _, window, _| {
                        let viewport =
                            [f32::from(bounds.size.width), f32::from(bounds.size.height)];
                        // Match Tauri's 360×240 world-space mini-map; physical bounds come from layout.
                        let camera = Camera::centered(center, viewport, viewport[0] / 360.);
                        scene.paint(
                            camera,
                            Selection {
                                allocated: &allocated,
                                preview: &HashSet::new(),
                                matches: &HashSet::new(),
                                searching: false,
                                socketed: &socketed,
                                progression_marker: Some(index),
                            },
                            TreeTheme::incarnation(),
                            bounds,
                            window,
                        );
                    },
                )
                .w_full()
                .h(rems(40. / 3.)),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn source(label: &str, kind: SourceType) -> SourceContribution {
        SourceContribution {
            label: label.into(),
            source_type: kind,
            value: (1., 1.),
            forge: None,
        }
    }
    #[::core::prelude::v1::test]
    fn item_preview_uses_player_equipment_and_rejects_ambiguous_slots() {
        let base = data::data().items.values().next().unwrap();
        let item = EquippedItem {
            base_id: base.id.clone(),
            ..Default::default()
        };
        let mut snapshot = BuildSnapshot::default();
        snapshot.inventory.insert("helmet".into(), item.clone());
        snapshot
            .merc_inventory
            .insert("helmet".into(), item.clone());
        let contribution = source(&base.name, SourceType::Item);
        assert!(find_item(&contribution, &snapshot).is_some());
        snapshot.inventory.insert("ring_1".into(), item);
        assert!(find_item(&contribution, &snapshot).is_none());
        snapshot.inventory.clear();
        assert!(find_item(&contribution, &snapshot).is_none());
    }
    #[::core::prelude::v1::test]
    fn ether_ids_cannot_resolve_to_incarnation_nodes() {
        let graph = Graph::load();
        let label = format!("Ether: Any #{}", graph.nodes[0].id);
        assert_eq!(find_node(&source(&label, SourceType::Tree), &graph), None);
    }
    #[::core::prelude::v1::test]
    fn resolves_socket_and_parent_item_labels() {
        assert_eq!(
            item_name(&source("Ruby in Crown #2 (Rainbow)", SourceType::Socket)),
            "Crown"
        );
        assert_eq!(
            item_name(&source("Bonus (Crown)", SourceType::Item)),
            "Crown"
        );
    }
    #[::core::prelude::v1::test]
    fn resolves_exact_tree_and_socket_ids_without_name_guessing() {
        let graph = Graph::load();
        let id = graph.nodes[0].id;
        for label in [
            format!("Tree: Any name #{id} (conditional)"),
            format!("Jewel (Tree Socket #{id})"),
        ] {
            assert_eq!(
                find_node(&source(&label, SourceType::Tree), &graph),
                Some(0)
            );
        }
        assert_eq!(
            find_node(&source("Tree: Missing #999999", SourceType::Tree), &graph),
            None
        );
        assert_eq!(
            find_node(&source(&format!("Item #{id}"), SourceType::Item), &graph),
            None
        );
    }
}
