use app_lib::calc::{data, tree::parse::classify_tree_node_line};
use std::collections::BTreeMap;

fn main() {
    let mut lines = BTreeMap::<(&str, String), Vec<u32>>::new();
    for (id, node) in data::tree_nodes() {
        for (kind, line) in node
            .lines
            .iter()
            .map(|line| ("line", line.as_str()))
            .chain(node.note.as_deref().map(|note| ("note", note)))
        {
            lines
                .entry((kind, line.to_string()))
                .or_default()
                .push(id.parse().unwrap());
        }
    }
    for ((kind, line), mut ids) in lines {
        ids.sort_unstable();
        println!(
            "{kind}\t{ids:?}\t{line}\t{:?}",
            classify_tree_node_line(&line)
        );
    }
}
