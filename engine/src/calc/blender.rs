//! Single-target Blender estimates. Binary evidence and unmeasured assumptions
//! are documented in engine/src/calc/blender-model.md; no animation count is an exact DPS.
use super::skills::calculation::{number, range, scalar, CalculationStep};
use super::skills::{rg, Ranged, StatMap};
use super::subskill::subskill_key;
use super::types::SkillSpec;
use std::collections::HashMap;

pub(crate) fn rank(spec: &SkillSpec, ranks: &HashMap<String, u32>, id: &str) -> u32 {
    let cap = spec
        .subskills
        .as_ref()
        .and_then(|s| s.iter().find(|s| s.id == id))
        .map(|s| s.max_rank)
        .unwrap_or(0);
    ranks
        .get(&subskill_key(&spec.id, id))
        .copied()
        .unwrap_or(0)
        .min(cap)
}

/// Phase-averaged local revolutions, not a guaranteed hit on creation. At 60
/// updates/s: angular speed starts at 240 deg/s and grows by 34.2 deg/s².
fn local_turns(seconds: f64) -> f64 {
    (240.0 * seconds + 17.1 * seconds * seconds) / 360.0
}

pub(crate) fn contacts(
    spec: &SkillSpec,
    ranks: &HashMap<String, u32>,
    stats: &StatMap,
    scoped: &StatMap,
    trace: &mut Vec<CalculationStep>,
) -> Ranged {
    let nano = rank(spec, ranks, "a_i_empowered_nanoblenders") > 0;
    let returning = !nano && rank(spec, ranks, "blenderang") > 0;
    let duration = rg(stats, "skill_duration");
    let scoped_duration = rg(scoped, "skill_duration");
    let orbital_duration = if returning {
        (0.0, 0.0)
    } else {
        rg(stats, "orbital_skill_duration")
    };
    let speed = if returning {
        (0.0, 0.0)
    } else {
        rg(stats, "orbital_skill_speed")
    };
    let area = rg(stats, "area_of_effect");
    // Explicit coverage heuristic: increased radius helps a glancing pass;
    // coverage saturates at 100%, never becomes an unbounded damage multiplier.
    let coverage = |aoe: f64| (0.75 * (1.0 + aoe / 100.0).max(0.0)).min(1.0);
    let calc = |d: f64, orbital: f64, velocity: f64, aoe: f64| {
        let seconds = spec.effect_duration.unwrap_or(5.0) * (1.0 + (d + orbital) / 100.0).max(0.0);
        let speed = (1.0 + velocity / 100.0).max(0.0);
        let turns = if nano {
            seconds * speed / 1.5
        } else if returning {
            // Outbound and return pass, bounded by the configured lifetime.
            // No claim of continuous contact during travel/spin acceleration.
            2.0 * (seconds / 5.0).min(1.0)
        } else {
            local_turns(seconds) * speed
        };
        turns * coverage(aoe)
    };
    let hits = (
        calc(
            duration.0 + scoped_duration.0,
            orbital_duration.0,
            speed.0,
            area.0,
        ),
        calc(
            duration.1 + scoped_duration.1,
            orbital_duration.1,
            speed.1,
            area.1,
        ),
    );
    let mode = if nano {
        "Nanoblenders: duration / 1.5s outer orbit"
    } else if returning {
        "Blenderang: estimated outbound + return contact; no guaranteed contact during travel"
    } else {
        "Blender: (240*T + 17.1*T²) / 360 local rotations at 60 updates/s"
    };
    trace.push(CalculationStep::new("Blender contacts per blade per cast", format!("Estimated {mode}; T = {}s × (1 + ({} global + {} subtree + {} orbital)% duration / 100); orbital speed {}%; coverage min(1, 0.75 × (1 + {}% area / 100)); casts may overlap", number(spec.effect_duration.unwrap_or(5.0)), range(duration), range(scoped_duration), range(orbital_duration), range(speed), range(area)), hits));
    hits
}

/// Extra on-hit effects are separate from the primary hit. Hit-count feedback
/// is deliberately not recursive: microblades cannot spawn more microblades.
pub(crate) fn secondary(
    spec: &SkillSpec,
    ranks: &HashMap<String, u32>,
    hits_per_second: Ranged,
    primary_dps: Ranged,
    kills_per_second: f64,
    trace: &mut Vec<CalculationStep>,
) -> Ranged {
    let mut micro_dps = (0.0, 0.0);
    for node in spec.subskills.as_deref().unwrap_or(&[]) {
        let level = rank(spec, ranks, &node.id);
        if level == 0 {
            continue;
        }
        let chance = node
            .proc
            .as_ref()
            .map(|p| {
                (p.chance.base.unwrap_or(0.0) + p.chance.per_rank.unwrap_or(0.0) * level as f64)
                    .clamp(0.0, 100.0)
                    / 100.0
            })
            .unwrap_or(0.0);
        let procs = (hits_per_second.0 * chance, hits_per_second.1 * chance);
        let (description, value) = match node.id.as_str() {
            "attachable_microblades" => {
                let bonus = node.proc.as_ref().and_then(|p| p.effects.as_ref()).and_then(|e| e.per_rank.as_ref()).and_then(|m| m.get("physical_skill_damage")).copied().unwrap_or(35.0);
                // Constructor: 3 blades, 2 seconds. Collision handler uses
                // damage * (1 + proc bonus), and omits the microblade proc slot.
                let contacts = 3.0 * local_turns(2.0) * 0.75;
                let factor = chance * (1.0 + bonus * level as f64 / 100.0) * contacts;
                micro_dps = (primary_dps.0 * factor, primary_dps.1 * factor);
                (format!("Estimated extra DPS: primary DPS × {}% proc chance × (1 + {}% damage) × {} contacts over 2s from 3 blades; stationary target, 75% coverage, no recursive procs", number(chance * 100.0), number(bonus * level as f64), number(contacts)), micro_dps)
            }
            "attachment_malfunction" => (format!("{}% on-hit chance; estimated triggers/s shown. Targeted departure/return changes placement, not base damage. Average coverage retained; no invented extra-hit multiplier", number(chance * 100.0)), procs),
            "blood_thirsting_killing_machine" => (format!("{}% on-hit chance; estimated stack grants/s shown. +{}% movement speed per stack; no direct DPS multiplier or assumed permanent maximum stacks", number(chance * 100.0), number(4.0 * level as f64)), procs),
            "blending_blood_pact" => ("Damage bonus included in physical skill damage. Estimated maximum-life % drain/s shown (4% per rank per kill); nonlethal, Blender ends if life cannot be paid. DPS assumes sufficient life".into(), scalar(4.0 * level as f64 * kills_per_second.max(0.0))),
            "industrial_sized" => ("Area increases estimated contact coverage up to 100%; does not multiply single-hit damage".into(), scalar(level as f64)),
            "no_bits_nor_pieces" => ("Attack-rating bonus included in the attack breakdown. No fabricated accuracy-to-DPS conversion without enemy evasion".into(), scalar(10.0 * level as f64)),
            "blenderang" => ("Damage and haste included; range affects reachable targets, not guaranteed single-target hits. Nanoblenders take movement precedence when both transformations are selected".into(), scalar(level as f64)),
            "will_it_blend" => ("Physical damage bonus included. Knockback/slow change positioning; no additional damage multiplier for control".into(), scalar(level as f64)),
            "it_will_blend" => ("Conditional damage included only when the target is marked bleeding; this node does not itself inflict bleed".into(), scalar(level as f64)),
            _ => ("Included through the skill's damage, critical, duration or separate group/blade calculation".into(), scalar(level as f64)),
        };
        trace.push(CalculationStep::new(
            format!("Blender · {}", node.name),
            description,
            value,
        ));
    }
    micro_dps
}
