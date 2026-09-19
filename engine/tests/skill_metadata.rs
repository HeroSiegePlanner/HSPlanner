//! Curated literals from the supplied game binary; independent of planner values.
use app_lib::calc::data;

#[test]
fn corrected_skill_metadata_matches_binary_assignments() {
    let evidence: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/skill-metadata-evidence.json")).unwrap();
    for row in evidence["verifiedFields"].as_array().unwrap() {
        let class = row["class"].as_str().unwrap();
        let id = row["skill"].as_str().unwrap();
        let skill = data::get_skills_by_class(class)
            .iter()
            .find(|s| s.id == id)
            .unwrap();
        let path: Vec<_> = row["field"].as_str().unwrap().split('.').collect();
        let actual = match path.as_slice() {
            ["baseCooldown"] => skill.base_cooldown,
            ["effectDuration"] => skill.effect_duration,
            ["damageFormula", field] => skill.damage_formula.map(|formula| match *field {
                "base" => formula.base,
                "perLevel" => formula.per_level,
                field => panic!("Unsupported evidence formula field {field}"),
            }),
            ["attackScaling", "weaponDamagePct", field] => skill
                .attack_scaling
                .and_then(|scaling| scaling.weapon_damage_pct)
                .map(|formula| match *field {
                    "base" => formula.base,
                    "perLevel" => formula.per_level,
                    field => panic!("Unsupported evidence formula field {field}"),
                }),
            ["passiveStats", rank, key] => skill
                .passive_stats
                .as_ref()
                .and_then(|p| match *rank {
                    "base" => p.base.as_ref(),
                    "perRank" => p.per_rank.as_ref(),
                    field => panic!("Unsupported evidence rank {field}"),
                })
                .and_then(|b| b.get(*key))
                .copied(),
            field => panic!("Unsupported evidence field {field:?}"),
        };
        assert_eq!(
            actual,
            row["value"].as_f64(),
            "{class}/{id} {} at {}",
            row["gameField"],
            row["assignment"]
        );
    }
}

#[test]
fn corrected_subskill_metadata_matches_binary_assignments() {
    let evidence: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/subskill-metadata-evidence.json")).unwrap();
    for row in evidence["verifiedFields"].as_array().unwrap() {
        let class = row["class"].as_str().unwrap();
        let id = row["skill"].as_str().unwrap();
        let node_id = row["node"].as_str().unwrap();
        let field = row["field"].as_str().unwrap();
        let skill = data::get_skills_by_class(class)
            .iter()
            .find(|s| s.id == id)
            .unwrap();
        let node = skill
            .subskills
            .as_ref()
            .unwrap()
            .iter()
            .find(|n| n.id == node_id)
            .unwrap();
        assert_eq!(
            u64::from(node.position_index),
            row["positionIndex"].as_u64().unwrap(),
            "{class}/{id}/{node_id}: binary node mapping"
        );
        let path: Vec<_> = field.split('.').collect();
        let actual = match path.as_slice() {
            ["proc", "chance", "base"] => node.proc.as_ref().and_then(|p| p.chance.base),
            ["proc", "chance", "perRank"] => node.proc.as_ref().and_then(|p| p.chance.per_rank),
            ["effects", rank, key] | ["proc", "effects", rank, key] => {
                let effect = if path[0] == "proc" {
                    node.proc.as_ref().and_then(|p| p.effects.as_ref())
                } else {
                    node.effects.as_ref()
                };
                effect
                    .and_then(|effect| match *rank {
                        "base" => effect.base.as_ref(),
                        "perRank" => effect.per_rank.as_ref(),
                        field => panic!("Unsupported evidence rank {field}"),
                    })
                    .and_then(|stats| stats.get(*key))
                    .copied()
            }
            _ => panic!("Unsupported evidence field {field}"),
        };
        assert_eq!(
            actual,
            row["value"].as_f64(),
            "{class}/{id}/{node_id} {field} at {}",
            row["assignment"]
        );
    }
}
