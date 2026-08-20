use super::*;

// ---------- set bonuses ----------

pub fn apply_set_bonuses(
    inventory: &Inventory,
    attr_sources: &mut SourceMap,
    stat_sources: &mut SourceMap,
) {
    let mut counts: HashMap<String, u32> = HashMap::new();
    for item in inventory.values() {
        if let Some(base) = data::get_item(&item.base_id) {
            if let Some(set_id) = base.set_id.as_deref() {
                *counts.entry(set_id.to_string()).or_insert(0) += 1;
            }
        }
    }
    for (set_id, count) in counts.iter() {
        let Some(set) = data::get_set(set_id) else {
            continue;
        };
        for bonus in set.bonuses.iter() {
            if *count < bonus.pieces {
                continue;
            }
            let label = format!("{} ({}-set)", set.name, bonus.pieces);
            for (stat_key, &value) in bonus.stats.iter() {
                if value == 0.0 {
                    continue;
                }
                apply_contribution(
                    attr_sources,
                    stat_sources,
                    stat_key,
                    (value, value),
                    label.clone(),
                    SourceType::Item,
                    None,
                );
            }
        }
    }
}

