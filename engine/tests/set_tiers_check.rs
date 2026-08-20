use app_lib::calc::data;
use app_lib::calc::stats::apply_set_bonuses;
use app_lib::calc::types::{EquippedItem, Inventory};
use std::collections::HashMap;

fn applied(item_ids: &[String], n: usize) -> usize {
    let mut inv: Inventory = HashMap::new();
    for (i, id) in item_ids.iter().take(n).enumerate() {
        inv.insert(
            format!("slot_{i}"),
            EquippedItem { base_id: id.clone(), ..Default::default() },
        );
    }
    let mut attrs = HashMap::new();
    let mut stats = HashMap::new();
    apply_set_bonuses(&inv, &mut attrs, &mut stats);
    attrs.values().map(|v: &Vec<_>| v.len()).sum::<usize>()
        + stats.values().map(|v: &Vec<_>| v.len()).sum::<usize>()
}

#[test]
fn thresholds_unlock_progressively() {
    let mut by_set: HashMap<String, Vec<String>> = HashMap::new();
    for item in data::data().items.values() {
        if let Some(id) = item.set_id.as_deref() {
            by_set.entry(id.to_string()).or_default().push(item.id.clone());
        }
    }

    let mut checked = 0;
    for (set_id, set) in data::data().sets.iter() {
        let Some(ids) = by_set.get(set_id) else { continue };
        let max = ids.len();

        assert_eq!(applied(ids, 1), 0, "{}: 1 sztuka nie moze nic dawac", set.name);

        let mut prev = 0;
        for n in 2..=max {
            let now = applied(ids, n);
            assert!(now >= prev, "{}: {n} sztuk dalo mniej niz {}", set.name, n - 1);
            prev = now;
        }

        for b in set.bonuses.iter() {
            if b.stats.values().all(|v| *v == 0.0) {
                continue;
            }
            let n = b.pieces as usize;
            assert!(
                applied(ids, n) > applied(ids, n - 1),
                "{}: prog {n} nie dolozyl nic",
                set.name
            );
        }

        let unreachable: Vec<_> = set.bonuses.iter().filter(|b| b.pieces as usize > max).collect();
        assert!(unreachable.is_empty(), "{}: progi poza zasiegiem", set.name);
        checked += 1;
    }
    println!("sprawdzone sety: {checked}");
    assert_eq!(checked, 65);
}
