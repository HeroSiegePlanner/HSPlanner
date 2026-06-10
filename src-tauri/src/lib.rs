pub mod calc;
mod suggest_engine;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .plugin(tauri_plugin_updater::Builder::new().build())
    .plugin(tauri_plugin_process::init())
    .plugin(tauri_plugin_opener::init())
    .setup(|app| {
      if cfg!(debug_assertions) {
        app.handle().plugin(
          tauri_plugin_log::Builder::default()
            .level(log::LevelFilter::Info)
            .build(),
        )?;
      }
      Ok(())
    })
    .invoke_handler(tauri::generate_handler![
      calc::commands::compute_skill_damage,
      calc::commands::compute_weapon_damage,
      calc::commands::calc_build_performance,
      calc::commands::calc_build_stats,
      calc::commands::calc_stat_breakdown,
      calc::commands::calc_warmup,
      calc::commands::passive_stats_at_rank,
      calc::commands::mana_cost_at_rank,
      calc::commands::subskill_aggregation,
      suggest_engine::command::suggest_tree_nodes,
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
