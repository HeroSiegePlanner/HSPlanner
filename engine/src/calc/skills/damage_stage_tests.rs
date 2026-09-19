use super::*;

fn fireball(
    scaling: DamageScaling,
    synergy: f64,
    stats: &[(&str, Ranged)],
    scoped: &[(&str, Ranged)],
) -> SkillDamageBreakdown {
    let skill = Skill {
        name: "Fireball".into(),
        tags: vec!["Spell".into()],
        damage_type: Some("fire".into()),
        damage_formula: Some(super::super::DamageFormula {
            base: 4.0,
            per_level: 18.0,
        }),
        damage_scaling: scaling,
        damage_per_rank: None,
        bonus_sources: vec![BonusSource::AttributePoint {
            source: "Intelligence".into(),
            stat: "fire_skill_damage".into(),
            value: synergy,
        }],
        attack_kind: None,
        attack_scaling: None,
    };
    let input = SkillInput {
        skill: &skill,
        allocated_rank: 1.0,
        attributes: &AttrMap::from([("Intelligence".into(), (1.0, 1.0))]),
        stats: &stats
            .iter()
            .map(|(key, value)| (key.to_string(), *value))
            .collect(),
        scoped: &scoped
            .iter()
            .map(|(key, value)| (key.to_string(), *value))
            .collect(),
        skill_ranks_by_name: &SkillRanks::new(),
        item_skill_bonuses: &ItemSkillBonuses::new(),
        enemy_conditions: &ConditionMap::new(),
        enemy_resistances: &ResistMap::new(),
        skills_by_name: &HashMap::new(),
        projectile_count: 1,
        of_total_damage: 0.0,
        conversion_flat: 0.0,
        conversion_skill_damage_pct: 0.0,
    };
    compute_skill_damage(&input).unwrap()
}

#[test]
fn verified_elemental_base_is_added_after_rank_flat_synergy_and_generic_bonuses() {
    // Independent runtime vectors: A + (L*r + F)*(1+Y)*(1+G), then own S.
    for (synergy, generic, flat, subtree, expected) in [
        (0.0, 0.0, 0.0, 0.0, 22),
        (100.0, 0.0, 0.0, 0.0, 40),
        (0.0, 100.0, 0.0, 0.0, 40),
        (0.0, 0.0, 0.0, 100.0, 44),
        (100.0, 100.0, 0.0, 100.0, 152),
        (100.0, 100.0, 10.0, 100.0, 232),
    ] {
        let d = fireball(
            DamageScaling::RankAndFlat,
            synergy,
            &[
                ("fire_skill_damage", (generic, generic)),
                ("flat_magic_skill_damage", (flat, flat)),
            ],
            &[("subtree_damage", (subtree, subtree))],
        );
        assert_eq!(
            d.hit_max, expected,
            "Y={synergy}, G={generic}, F={flat}, S={subtree}"
        );
    }
}

#[test]
fn legacy_formula_keeps_its_existing_scaling_until_verified() {
    let d = fireball(
        DamageScaling::Full,
        100.0,
        &[("fire_skill_damage", (100.0, 100.0))],
        &[],
    );
    assert_eq!(d.hit_max, 88);
}

#[test]
fn own_subtree_pool_is_additive_and_keeps_both_range_endpoints() {
    let d = fireball(
        DamageScaling::RankAndFlat,
        0.0,
        &[("fire_skill_damage", (0.0, 100.0))],
        &[("subtree_damage", (0.0, 100.0))],
    );
    assert_eq!((d.hit_min, d.hit_max), (22, 80));
    let d = fireball(
        DamageScaling::RankAndFlat,
        0.0,
        &[("fire_skill_damage", (100.0, 100.0))],
        &[("subtree_damage", (55.0, 55.0))],
    );
    assert_eq!(d.hit_max, 62); // Inferno Ball 15 + Splitfire 40, one pool.
}

#[test]
fn independent_proc_bonuses_add_to_the_same_pool_only_in_expected_damage() {
    // Rank-one rolls: 3%*35 + 1%*50 + 20%*60 = 13.55%.
    // With S=55 and 100% generic, ordinary hit=62; E[hit]=67.42.
    let d = fireball(
        DamageScaling::RankAndFlat,
        0.0,
        &[("fire_skill_damage", (100.0, 100.0))],
        &[
            ("subtree_damage", (55.0, 55.0)),
            ("subtree_damage_on_proc", (13.55, 13.55)),
        ],
    );
    assert_eq!(d.hit_max, 62);
    assert_eq!(d.avg_max, 67);
    assert_eq!(d.projectile_count, 1);
    assert_eq!(
        d.calculation
            .iter()
            .find(|s| s.label() == "Expected own subtree damage multiplier")
            .unwrap()
            .value(),
        (1.6855, 1.6855)
    );
}

#[test]
fn proc_expectation_preserves_ranges_and_independent_double_damage() {
    let d = fireball(
        DamageScaling::RankAndFlat,
        0.0,
        &[],
        &[
            ("subtree_damage_on_proc", (0.0, 100.0)),
            ("double_damage_chance", (0.0, 100.0)),
        ],
    );
    assert_eq!((d.hit_min, d.hit_max), (22, 22));
    assert_eq!((d.avg_min, d.avg_max), (22, 88));
}

#[test]
fn elemental_helper_rounds_up_before_the_own_subtree_multiplier() {
    // ceil(4 + 18*1.1) = 24; floor(24*1.15) = 27, not 26.
    let d = fireball(
        DamageScaling::RankAndFlat,
        0.0,
        &[("fire_skill_damage", (10.0, 10.0))],
        &[("subtree_damage", (15.0, 15.0))],
    );
    assert_eq!(d.hit_max, 27);
    assert_eq!(
        d.calculation
            .iter()
            .find(|s| s.label() == "Damage after generic helper rounding")
            .unwrap()
            .value(),
        (24.0, 24.0)
    );
}

#[test]
fn total_spell_damage_adds_to_generic_total_instead_of_multiplying_it() {
    let d = fireball(
        DamageScaling::RankAndFlat,
        0.0,
        &[
            ("damage", (20.0, 20.0)),
            ("spell_damage_more", (25.0, 25.0)),
        ],
        &[],
    );
    // ceil(22*1.45), not ceil(22*1.2*1.25).
    assert_eq!(d.hit_max, 32);
    assert_eq!(
        d.calculation
            .iter()
            .find(|s| s.label() == "Total damage multiplier")
            .unwrap()
            .value(),
        (1.45, 1.45)
    );
}

#[test]
fn positive_spell_damage_multiplies_and_floors_the_complete_generic_value() {
    let d = fireball(
        DamageScaling::RankAndFlat,
        0.0,
        &[("spell_damage", (10.0, 10.0))],
        &[("subtree_damage", (50.0, 50.0))],
    );
    // floor(22*1.1) * 1.5 = 36, not floor((4+18*1.1)*1.5)=35.
    assert_eq!(d.hit_max, 36);
    assert_eq!(d.skill_damage_max_pct, 0.0);
    let negative = fireball(
        DamageScaling::RankAndFlat,
        0.0,
        &[("spell_damage", (-50.0, 0.0))],
        &[],
    );
    assert_eq!((negative.hit_min, negative.hit_max), (22, 22));
}
