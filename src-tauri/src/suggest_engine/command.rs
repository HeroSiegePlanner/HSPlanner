use super::algo::suggest;
use super::types::{PrecomputedInput, SuggestResult};

// A panic inside suggest() must degrade to "no suggestions" instead of
// re-panicking on the IPC runtime thread (calc_warmup handles its JoinError
// the same way).
async fn join_or_default(
    task: tauri::async_runtime::JoinHandle<SuggestResult>,
) -> SuggestResult {
    task.await.unwrap_or_else(|e| {
        eprintln!("suggest_tree_nodes task panicked: {e}");
        SuggestResult::default()
    })
}

#[tauri::command]
pub async fn suggest_tree_nodes(
    app: tauri::AppHandle,
    input: PrecomputedInput,
) -> SuggestResult {
    // Run the CPU-heavy greedy/DPS loop on a blocking thread so the Tauri event
    // loop (and therefore the webview) stays responsive while it iterates.
    // SeasonScope installed inside the closure so it never crosses an .await.
    let season = input.season.clone();
    join_or_default(tauri::async_runtime::spawn_blocking(move || {
        let _scope = crate::calc::season::SeasonScope::enter(season);
        suggest(&input, Some(&app))
    }))
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panicked_task_returns_default_result() {
        let result = tauri::async_runtime::block_on(async {
            let task = tauri::async_runtime::spawn_blocking(|| -> SuggestResult {
                panic!("boom")
            });
            join_or_default(task).await
        });
        assert_eq!(result, SuggestResult::default());
    }
}
