fn main() {
    let map = app_lib::calc::commands::classify_tree_nodes_impl();
    println!("{}", serde_json::to_string_pretty(&map).unwrap());
}
