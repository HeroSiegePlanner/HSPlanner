//! DPS snapshot of every active skill on a fixed reference build.
//!
//! Not an assertion test: run it to compare planner numbers before and after a
//! model change.
//! `HSP_DPS_SNAPSHOT=/path/out.csv cargo test -p hsplanner-engine dps_snapshot -- --ignored`

use super::*;

const CLASSES: [&str; 24] = [
    "amazon",
    "bard",
    "butcher",
    "demon_slayer",
    "demonspawn",
    "exo",
    "illusionist",
    "jotunn",
    "marauder",
    "marksman",
    "necromancer",
    "nomad",
    "paladin",
    "pirate",
    "plague_doctor",
    "prophet",
    "pyromancer",
    "redneck",
    "samurai",
    "shaman",
    "shield_lancer",
    "stormweaver",
    "viking",
    "white_mage",
];

fn step_value(perf: &BuildPerformance, label: &str) -> Option<Ranged> {
    perf.calculation()
        .iter()
        .rev()
        .find(|s| s.label() == label)
        .map(|s| s.value())
}

fn fmt(v: Option<f64>) -> String {
    v.map_or(String::new(), |x| format!("{x:.3}"))
}

#[test]
#[ignore]
fn dps_snapshot() {
    let out = std::env::var("HSP_DPS_SNAPSHOT").unwrap_or_else(|_| "dps_snapshot.csv".into());
    let mut rows = vec![
        "class,skill,name,actions_per_s,hits_per_cast,hit_avg,hit_dps,combined_dps,dummy_conf"
            .to_string(),
    ];
    // Reference build: level 100, skill rank 20, no gear, no subskills; weapon
    // requirements lifted so attack skills still report.
    let stats = vec![CustomStat {
        stat_key: "skill_restrictions_removed".into(),
        value: "100".into(),
    }];
    let empty_f = HashMap::new();
    let empty_b = HashMap::new();
    let empty_u = HashMap::new();
    let inventory: Inventory = HashMap::new();
    for class in CLASSES {
        // HSP_ALL_BUFFS=1: every buff toggle of the class on.
        let buffs: HashMap<String, bool> = if std::env::var("HSP_ALL_BUFFS").is_ok() {
            crate::calc::data::get_skills_by_class(class)
                .iter()
                .map(|s| (s.id.clone(), true))
                .collect()
        } else {
            HashMap::new()
        };
        for spec in crate::calc::data::get_skills_by_class(class) {
            if spec.kind != crate::calc::types::SkillKind::Active {
                continue;
            }
            // HSP_ALL_SKILLS=1: every skill of the class at rank 20 (synergies on).
            let ranks: HashMap<String, u32> = if std::env::var("HSP_ALL_SKILLS").is_ok() {
                crate::calc::data::get_skills_by_class(class)
                    .iter()
                    .map(|s| (s.id.clone(), 20))
                    .collect()
            } else {
                [(spec.id.clone(), 20)].into_iter().collect()
            };
            let perf = compute_build_performance(&BuildPerformanceDeps {
                class_id: Some(class),
                level: 100,
                allocated_attrs: &HashMap::new(),
                inventory: &inventory,
                skill_ranks: &ranks,
                subskill_ranks: &empty_u,
                active_aura_id: None,
                active_buffs: &buffs,
                custom_stats: &stats,
                allocated_tree_nodes: &HashSet::new(),
                tree_socketed: &HashMap::new(),
                main_skill_id: Some(spec.id.as_str()),
                enemy_conditions: &empty_b,
                player_conditions: &HashMap::new(),
                skill_projectiles: &HashMap::new(),
                enemy_resistances: &HashMap::new(),
                proc_toggles: &empty_b,
                kills_per_sec: 0.0,
                entity_rates: &empty_f,
                stack_counts: &HashMap::new(),
                granted_skill_ranks: None,
                difficulty: None,
            });
            let rate = step_value(&perf, "Actions per second")
                .or_else(|| step_value(&perf, "Entity actions per second"))
                .map(|r| r.1)
                .or_else(|| {
                    perf.attack_damage
                        .as_ref()
                        .map(|a| a.attacks_per_second_max)
                });
            rows.push(format!(
                "{class},{},{},{},{},{},{},{},{}",
                spec.id,
                spec.name.replace(',', " "),
                fmt(rate),
                fmt(step_value(&perf, "Dummy hits per use")
                    .map(|h| h.1)
                    .or(perf.hits_per_cast.map(|h| h.1))
                    .or(Some(1.0))),
                fmt(perf
                    .attack_damage
                    .as_ref()
                    .map(|a| a.combined_avg_max as f64)
                    .or(perf.damage.as_ref().map(|d| d.avg_max as f64))),
                fmt(perf.hit_dps_max),
                fmt(perf.combined_dps_max),
                spec.dummy_hits.as_ref().map_or("-".to_string(), |d| d
                    .confidence
                    .clone()
                    .unwrap_or_default()),
            ));
        }
    }
    std::fs::write(&out, rows.join("\n") + "\n").expect("write snapshot");
    eprintln!("wrote {} rows to {out}", rows.len() - 1);
}

#[test]
fn tick_hits_matches_rule_20_table() {
    // tools/tick_hits.py: (lifetime s, tick s) -> hits, active = life*60 - 2.
    for (life, tick, want) in [
        (1.25, 0.5, 3),
        (1.0, 0.5, 2),
        (2.0, 0.5, 4),
        (3.0, 0.5, 6),
        (4.0, 0.33, 12),
        (4.5, 0.4, 11),
        (5.0, 0.25, 19),
        (2.25, 0.5, 5),
    ] {
        let active = (life * 60.0_f64).round() as u32 - 2;
        assert_eq!(
            tick_hits(active, tick * 60.0, 0, 0),
            want,
            "{life} s / {tick} s"
        );
    }
}

fn dummy_hits_with(class: &str, skill: &str, subskills: &[(&str, u32)]) -> f64 {
    dummy_hits_with_stats(class, skill, subskills, &[])
}

fn dummy_hits_with_stats(
    class: &str,
    skill: &str,
    subskills: &[(&str, u32)],
    custom_stats: &[CustomStat],
) -> f64 {
    let ranks: HashMap<String, u32> = [(skill.to_string(), 20)].into_iter().collect();
    let sub_ranks: HashMap<String, u32> = subskills
        .iter()
        .map(|(id, r)| (subskill_key(skill, id), *r))
        .collect();
    let perf = compute_build_performance(&BuildPerformanceDeps {
        class_id: Some(class),
        level: 100,
        allocated_attrs: &HashMap::new(),
        inventory: &HashMap::new(),
        skill_ranks: &ranks,
        subskill_ranks: &sub_ranks,
        active_aura_id: None,
        active_buffs: &HashMap::new(),
        custom_stats,
        allocated_tree_nodes: &HashSet::new(),
        tree_socketed: &HashMap::new(),
        main_skill_id: Some(skill),
        enemy_conditions: &HashMap::new(),
        player_conditions: &HashMap::new(),
        skill_projectiles: &HashMap::new(),
        enemy_resistances: &HashMap::new(),
        proc_toggles: &HashMap::new(),
        kills_per_sec: 0.0,
        entity_rates: &HashMap::new(),
        stack_counts: &HashMap::new(),
        granted_skill_ranks: None,
        difficulty: None,
    });
    step_value(&perf, "Dummy hits per use")
        .expect("dummy hits")
        .1
}

#[test]
fn skill_duration_stretches_storm_cloud_and_blade_barrier() {
    let stat = |key: &str, value: &str| CustomStat {
        stat_key: key.into(),
        value: value.into(),
    };
    // 8 s cloud at +25% Skill Duration: 20 ticks instead of 17.
    let cloud = dummy_hits_with_stats("stormweaver", "storm_cloud", &[], &[stat("skill_duration", "25")]);
    assert!((cloud - 20.0).abs() < 1e-9, "{cloud}");
    // Barrier Extension r5 (+50%): the orbit simulation gives 1.50x the 35 hits.
    let unlocked = [stat("skill_restrictions_removed", "100")];
    let base = dummy_hits_with_stats("samurai", "blade_barrier", &[], &unlocked);
    let longer = dummy_hits_with_stats("samurai", "blade_barrier", &[("barrier_extension", 5)], &unlocked);
    assert!((base - 35.0).abs() < 1e-9, "{base}");
    assert!((longer / base - 1.5).abs() < 0.02, "{longer}");
}

#[test]
fn comet_extras_fall_on_the_cursor_and_volcano_stretches_with_duration() {
    let stat = |key: &str, value: &str| CustomStat {
        stat_key: key.into(),
        value: value.into(),
    };
    // Every extra comet falls on the cursor point, 30 px from the dummy.
    let comets = dummy_hits_with_stats("pyromancer", "comet", &[], &[stat("additional_spell_projectile", "1")]);
    assert!((comets - 2.0).abs() < 1e-9, "{comets}");
    // 4 s volcano at +25% Skill Duration: 5 s of 0.5 s ticks.
    let volcano = dummy_hits_with_stats("pyromancer", "volcano", &[], &[stat("skill_duration", "25")]);
    let want = 8.0 * tick_hits(298, 30.0, 0, 0) as f64 / tick_hits(238, 30.0, 0, 0) as f64;
    assert!((volcano - want).abs() < 1e-9, "{volcano} vs {want}");
}

#[test]
fn mine_field_ring_mines_reach_the_dummy_only_at_rank_one() {
    // 40 % r mines; rank 1 puts two ring mines below the cursor point, within the 37.6 px reach.
    for (rank, want) in [(1, 1.2), (2, 0.8), (3, 1.2)] {
        let hits = dummy_hits_with("marksman", "landmine", &[("mine_field", rank)]);
        assert!((hits - want).abs() < 1e-9, "rank {rank}: {hits}");
    }
}

#[test]
fn temporal_marksman_fires_one_lobbed_arrow_per_attack() {
    // Viking 3, Marksman 1, Pyromancer 1 hits, weighted by their attack rates.
    let base = dummy_hits_with("illusionist", "temporal_heroes", &[]);
    assert!((base - 1.67).abs() < 1e-9, "{base}");
    let augmented = dummy_hits_with("illusionist", "temporal_heroes", &[("augmented_heroics", 1)]);
    assert!((augmented - 1.67 * 1.77).abs() < 1e-9, "{augmented}");
}

#[test]
fn holy_hammer_band_uses_the_full_size_hammer() {
    // The cast writes scale 1.0 over Create's 0.75: r 27 + 12 = 39 px band, 78 of every 87 px of spiral.
    let hits = dummy_hits_with("paladin", "holy_hammer", &[]);
    assert!((hits - 0.9).abs() < 1e-9, "{hits}");
    // Size bonuses widen every spiral hammer and add up in its scale.
    let colossus = dummy_hits_with("paladin", "holy_hammer", &[("colossus_hammer", 1)]);
    assert!((colossus - 0.9 * 1.104).abs() < 1e-9, "{colossus}");
    let both = dummy_hits_with(
        "paladin",
        "holy_hammer",
        &[("colossus_hammer", 1), ("orbital_light", 1)],
    );
    assert!((both - 0.9 * (1.0 + 0.104 + 0.208)).abs() < 1e-9, "{both}");
}

#[test]
fn cannonball_chain_finds_no_second_target_on_a_lone_dummy() {
    // The chain skips the enemy the ball hit; its pop-up bomb hits instead, so longer chains add nothing.
    let base = dummy_hits_with("pirate", "cannonball", &[]);
    assert!((base - 5.0).abs() < 1e-9, "{base}");
    let scatter = dummy_hits_with("pirate", "cannonball", &[("cannon_scatter", 3)]);
    assert!((scatter - 5.0).abs() < 1e-9, "{scatter}");
}

#[test]
fn hydras_breeze_cones_hit_as_the_fan_lays_them_out() {
    // The cone grows to x1.55 before it fades: 1..5 cones 15 deg apart hit 1, 2, 3, 2, 3.
    for (rank, want) in [(1, 2.0), (2, 3.0), (3, 2.0)] {
        let hits = dummy_hits_with("pyromancer", "breath_of_fire", &[("hydras_breeze", rank)]);
        assert!((hits - want).abs() < 1e-9, "rank {rank}: {hits}");
    }
}

#[test]
fn sand_gush_waves_need_the_controller_of_sandstorm_or_sand_tornado() {
    // Base use: the player-centred wave; its two tornadoes miss a dummy within 20 px.
    let base = dummy_hits_with("nomad", "sand_gush", &[]);
    assert!((base - 1.0).abs() < 1e-9, "{base}");
    for node in ["sandstorm", "sand_tornado"] {
        let hits = dummy_hits_with("nomad", "sand_gush", &[(node, 1)]);
        assert!((hits - 32.8).abs() < 1e-9, "{node}: {hits}");
    }
    // Tornado bonuses count only once the controller makes its tornadoes.
    let winds = dummy_hits_with("nomad", "sand_gush", &[("gushing_winds", 1)]);
    assert!((winds - 1.0).abs() < 1e-9, "{winds}");
    let storm = dummy_hits_with("nomad", "sand_gush", &[("sandstorm", 1), ("desert_storm", 1)]);
    assert!((storm - 39.7).abs() < 1e-9, "{storm}");
}

#[test]
fn sand_vortex_keeps_hitting_while_it_fades() {
    let hits = dummy_hits_with("nomad", "sand_vortex", &[]);
    assert!((hits - 13.0).abs() < 1e-9, "{hits}");
}

#[test]
fn lunar_orbit_casts_one_moon_until_lunar_architect() {
    // One moon on the next ring of the player's counter, dummy at 72 px.
    let base = dummy_hits_with("exo", "lunar_orbit", &[]);
    assert!((base - 2.6).abs() < 1e-9, "{base}");
    // Extrasolar Moons adds moons only inside Lunar Architect's branch.
    let lone = dummy_hits_with("exo", "lunar_orbit", &[("extrasolar_moons", 2)]);
    assert!((lone - 2.6).abs() < 1e-9, "{lone}");
    let architect = dummy_hits_with("exo", "lunar_orbit", &[("lunar_architect", 1)]);
    assert!((architect - 13.8).abs() < 1e-9, "{architect}");
    let both = dummy_hits_with("exo", "lunar_orbit", &[("lunar_architect", 1), ("extrasolar_moons", 2)]);
    assert!((both - 18.34).abs() < 1e-9, "{both}");
}

#[test]
fn black_hole_keeps_hitting_while_it_shrinks() {
    let hits = dummy_hits_with("exo", "black_hole", &[]);
    assert!((hits - 12.0).abs() < 1e-9, "{hits}");
}

#[test]
fn circle_of_guardians_strikes_land_with_the_real_hitbox() {
    // The 80x48 strike sprite reaches 86 px from 32 px above the guardian: 0.94 of strikes hit.
    let hits = dummy_hits_with("illusionist", "circle_of_guardians", &[]);
    assert!((hits - 0.94).abs() < 1e-9, "{hits}");
}

#[test]
fn skill_duration_leaves_the_single_holy_bolt_hit() {
    // The base bolt has no tick (only Divine Assbeating adds one), so it hits once.
    let stat = CustomStat {
        stat_key: "skill_duration".into(),
        value: "50".into(),
    };
    let hits = dummy_hits_with_stats("paladin", "holy_bolt", &[], &[stat]);
    assert!((hits - 1.0).abs() < 1e-9, "{hits}");
}

#[test]
fn skill_duration_leaves_the_plague_doctor_base_hits() {
    // Miasma's and Toxic Flask's ticking clouds belong to sub-skills, not the base use.
    let stat = |key: &str, value: &str| CustomStat {
        stat_key: key.into(),
        value: value.into(),
    };
    let longer = [stat("skill_duration", "50"), stat("skill_restrictions_removed", "100")];
    let miasma = dummy_hits_with_stats("plague_doctor", "miasma", &[], &longer);
    assert!((miasma - 2.0).abs() < 1e-9, "{miasma}");
    let flask = dummy_hits_with_stats("plague_doctor", "toxic_flask", &[], &longer);
    assert!((flask - 1.0).abs() < 1e-9, "{flask}");
}

#[test]
fn plague_rats_with_the_24_px_mask_hit_less_off_axis() {
    // Rats at +-10 deg pass 27.8 px from the dummy and hit for 47.9 % of aim directions.
    let rats = dummy_hits_with("plague_doctor", "plague_of_rats", &[]);
    assert!((rats - 1.48).abs() < 1e-9, "{rats}");
}

#[test]
fn skill_duration_leaves_the_passing_shurikens() {
    // Each single-hit shuriken flies past the dummy in ~5 frames; a longer life adds no hit.
    let stat = |key: &str, value: &str| CustomStat {
        stat_key: key.into(),
        value: value.into(),
    };
    let longer = [stat("skill_duration", "50"), stat("skill_restrictions_removed", "100")];
    let hits = dummy_hits_with_stats("samurai", "shuriken_throw", &[], &longer);
    assert!((hits - 6.0).abs() < 1e-9, "{hits}");
}

#[test]
fn skill_duration_leaves_the_single_lightning_surge_ray() {
    // The ticking tether exists only with Concurrent Current; the base ray hits once.
    let stat = CustomStat {
        stat_key: "skill_duration".into(),
        value: "50".into(),
    };
    let hits = dummy_hits_with_stats("stormweaver", "lightning_surge", &[], &[stat]);
    assert!((hits - 1.0).abs() < 1e-9, "{hits}");
}

#[test]
fn skill_duration_leaves_the_two_torrent_explosions() {
    // The ticking vortex is Whirlpool's; the base marker makes two single-hit explosions.
    let stat = |key: &str, value: &str| CustomStat {
        stat_key: key.into(),
        value: value.into(),
    };
    let longer = [stat("skill_duration", "50"), stat("skill_restrictions_removed", "100")];
    let hits = dummy_hits_with_stats("pirate", "torrent", &[], &longer);
    assert!((hits - 2.0).abs() < 1e-9, "{hits}");
}

#[test]
fn torrent_markers_leave_the_cursor_with_extra_projectiles() {
    // With n > 1 the markers ring the cursor 70 px out; only their r 80 second blast reaches: 2, 2, 3, 4, 4 hits.
    let with = |extra: &str| {
        let stats = [
            CustomStat {
                stat_key: "additional_spell_projectile".into(),
                value: extra.into(),
            },
            CustomStat {
                stat_key: "skill_restrictions_removed".into(),
                value: "100".into(),
            },
        ];
        dummy_hits_with_stats("pirate", "torrent", &[], &stats)
    };
    assert!((with("1") - 2.5).abs() < 1e-9, "{}", with("1"));
    assert!((with("4") - 4.0).abs() < 1e-9, "{}", with("4"));
}

#[test]
fn multinet_extra_net_lands_past_the_dummy() {
    // The extra net lands 64 px further along the aim, outside the 43 px blast reach.
    let unlocked = [CustomStat {
        stat_key: "skill_restrictions_removed".into(),
        value: "100".into(),
    }];
    let hits = dummy_hits_with_stats("marauder", "retiarius_net", &[("multinet_throw", 5)], &unlocked);
    assert!((hits - 1.0).abs() < 1e-9, "{hits}");
}

#[test]
fn skill_duration_leaves_the_passing_shadow_bolt() {
    // The bolt's 3 ticks land while it crosses the dummy (frames 31-49 of 120), not over its life.
    let stat = CustomStat {
        stat_key: "skill_duration".into(),
        value: "50".into(),
    };
    let hits = dummy_hits_with_stats("white_mage", "shadow_bolt", &[], &[stat]);
    assert!((hits - 3.0).abs() < 1e-9, "{hits}");
}

#[test]
fn skill_duration_leaves_the_marching_black_mass() {
    // Each cultist hits once while it marches past; a longer life only lengthens the march.
    let stat = CustomStat {
        stat_key: "skill_duration".into(),
        value: "50".into(),
    };
    let hits = dummy_hits_with_stats("white_mage", "black_mass", &[], &[stat]);
    assert!((hits - 15.0).abs() < 1e-9, "{hits}");
}

#[test]
fn skill_duration_stretches_the_four_second_shredder_trap() {
    // The case sets destroyTimer to 4 s (240 frames); +50% gives 358 active frames = 15 ticks.
    let stat = |key: &str, value: &str| CustomStat {
        stat_key: key.into(),
        value: value.into(),
    };
    let longer = [stat("skill_duration", "50"), stat("skill_restrictions_removed", "100")];
    let hits = dummy_hits_with_stats("demon_slayer", "shredder_trap", &[], &longer);
    let want = 10.0 * tick_hits(358, 24.0, 0, 0) as f64 / tick_hits(238, 24.0, 0, 0) as f64;
    assert!((hits - want).abs() < 1e-9, "{hits} vs {want}");
}

#[test]
fn bone_barrage_bones_hit_with_the_symmetric_mask() {
    // Offsets -12, -6, then 0/+6; the +-19 px mask misses only the first bone (33 px off).
    let hits = dummy_hits_with("demonspawn", "bone_barrage", &[]);
    assert!((hits - 19.0).abs() < 1e-9, "{hits}");
}

#[test]
fn subskill_dummy_effects_change_hits() {
    let base = dummy_hits_with("shaman", "twisters", &[]);
    assert!((base - 1.36).abs() < 1e-9, "{base}");
    // Explosive Gust: an explosion on every hit doubles the hits.
    let gust = dummy_hits_with("shaman", "twisters", &[("explosive_gust", 1)]);
    assert!((gust - 2.0 * base).abs() < 1e-9, "{gust}");
    // Twistering Multitude r3: the card's H(6) = 2.72 replaces the projectile slope.
    let multi = dummy_hits_with("shaman", "twisters", &[("twistering_multitude", 3)]);
    assert!((multi - (base + 1.36)).abs() < 1e-9, "{multi}");
}
