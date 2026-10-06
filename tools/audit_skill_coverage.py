#!/usr/bin/env python3
"""Inventory every class skill/subskill and audit static calculation coverage.

Usage: python3 tools/audit_skill_coverage.py [--output /tmp/skill-audit.json]

This tool only reads the repository. It never infers game damage coefficients
from English descriptions. An encoded calculation path is not proof of parity
with the game: all records retain gameValidation="unknown" until a separate
binary/measurement audit establishes that evidence. Spatial and support stats
without a DPS consumer are reported as unknown, not automatically as bugs.
"""

import argparse
import collections
import hashlib
import json
from pathlib import Path
import re
import sys


ENTITY_TAGS = {"Sentry", "Summon", "Guardian"}
ELEMENT_BREAK_KEYS = {
    "arcane_break", "poison_break", "lightning_break", "fire_break", "cold_break"
}
GEOMETRY_KEYS = {
    "projectile_size", "skill_range", "max_targets", "projectile_bounces",
    "chain_count", "pull_strength", "terrain_ignored", "reduced_movement_diminish",
    "slow_duration", "summon_dodge_chance", "summon_life_steal",
}
MECHANIC_DESCRIPTION = re.compile(
    r"\b(per second|every second|each second|every \w+ seconds|stacks?|stacking|"
    r"pulses?|tick|lasting|after|until|while|based on|depending|return|bounce|"
    r"split|chain|orbit|trail|double damage)\b", re.IGNORECASE
)


def load_json(path):
    return json.loads(path.read_text(encoding="utf-8"))


def normalize_name(name):
    return name.strip().lower()


def effect_keys(effect):
    return set(effect.get("base", {})) | set(effect.get("perRank", {}))


def has_direct_formula(skill):
    return bool(skill.get("damageFormula") or skill.get("damagePerRank"))


def has_attack_formula(skill):
    return skill.get("attackKind") == "attack" and bool(skill.get("attackScaling"))


def is_entity(skill):
    return bool(ENTITY_TAGS.intersection(skill.get("tags", [])))


def issue(code, status, message, evidence, **details):
    return dict(code=code, status=status, message=message, evidence=evidence, **details)


def cooldown_path_candidates(build, cooldown_code):
    """Recognize old inline and shared-helper paths, never claim execution proof."""
    helper_present = all(marker in cooldown_code for marker in (
        "fn effective_cooldown(", "skill.base_cooldown", '"cooldown_added"',
    ))
    spell_cooldown = bool(re.search(
        r"let cooldown\s*=\s*active_skill\s*\.and_then\(\|s\|\s*s\.base_cooldown\)", build
    )) or (helper_present and bool(re.search(
        r"let cooldown\s*=\s*active_skill\s*\.and_then\(\|s\|\s*"
        r"super::skill_cost::effective_cooldown\(s,\s*main_scoped\)\)", build
    )))
    spell_path = spell_cooldown and all(marker in build for marker in (
        "let effective_rate = if let Some(cooldown) = cooldown", 'let haste = stat("skill_haste")',
    ))
    attack_cooldown = "spec.base_cooldown.filter(|cd| *cd > 0.0)" in build or (
        helper_present and bool(re.search(
            r"super::skill_cost::effective_cooldown\(spec,\s*scoped\)\?", build
        ))
    )
    attack_path = attack_cooldown and "fn attack_action_rate_override(" in build
    return bool(spell_path), bool(attack_path)


def audit(root):
    data_paths = sorted((root / "data/skills").glob("*.json"))
    if not data_paths:
        raise ValueError(f"No class skill JSON files found under {root / 'data/skills'}")
    skills = []
    locations = {}
    for path in data_paths:
        text = path.read_text(encoding="utf-8")
        for skill in json.loads(text):
            skills.append(skill)
            match = re.search(r'"id"\s*:\s*"' + re.escape(skill["id"]) + '"', text)
            line = text.count("\n", 0, match.start()) + 1 if match else 1
            locations[(skill["classId"], skill["id"])] = {
                "file": str(path.relative_to(root)), "line": line,
            }

    config_path = root / "data/game-config.json"
    affix_path = root / "data/affix-tags.json"
    changes_path = root / "data/subskill-tags.json"
    config = load_json(config_path)
    definitions = {row["key"]: row for row in config["stats"]}
    scoped = {key for key, row in definitions.items() if row.get("skillScoped")}
    affixes = load_json(affix_path)
    tag_changes = load_json(changes_path)
    code = {}
    source_paths = []
    for path in sorted((root / "engine/src/calc").rglob("*.rs")):
        if path.name.endswith("tests.rs") or "tests" in path.parts:
            continue
        source_paths.append(path)
        code[str(path.relative_to(root))] = path.read_text(encoding="utf-8").split("#[cfg(test)]")[0]
    references = collections.defaultdict(list)
    for path, text in code.items():
        for line, content in enumerate(text.splitlines(), 1):
            for key in set(re.findall(r'"([a-z][a-z0-9_]+)"', content)):
                references[key].append({"file": path, "line": line})

    build = code.get("engine/src/calc/build.rs", "")
    attack_code = code.get("engine/src/calc/skills/attack.rs", "")
    conversion_code = code.get("engine/src/calc/skills/conversion.rs", "")
    type_code = code.get("engine/src/calc/types.rs", "")
    aggregation_code = code.get("engine/src/calc/subskill.rs", "")
    scoped_code = code.get("engine/src/calc/stats/attributes.rs", "")
    blender_gate = "apply_skill_physical_modifiers: is_blender" in build
    legacy_cadence = "else if active_skill.is_some_and(|s| s.uses_skill_haste)" in build
    metadata_cooldown_gate, attack_cooldown_gate = cooldown_path_candidates(
        build, code.get("engine/src/calc/skill_cost.rs", "")
    )
    weighted_proc_stats = "combined.entry(k.clone()).or_insert(0.0) += v" in aggregation_code
    blender_proc_exemption = 'value - agg.proc_stats.get(key)' in scoped_code
    fixed_proc_rate = bool(re.search(
        r'proc\.trigger\s*==\s*"on_kill"[^;]*else\s*\{\s*1\.0\s*\}', build
    ))
    unresolved = set(re.findall(r'\("([a-z_]+)",\s*Source::Unresolved,', conversion_code))
    by_name = {(skill["classId"], normalize_name(skill["name"])): skill for skill in skills}
    records = []
    node_records = []
    findings = []

    def add_findings(skill, found, node_id=None):
        for finding in found:
            record = {"classId": skill["classId"], "skillId": skill["id"], **finding}
            if node_id:
                record["nodeId"] = node_id
            findings.append(record)

    for skill in skills:
        sid, cid = skill["id"], skill["classId"]
        direct, attack = has_direct_formula(skill), has_attack_formula(skill)
        entity = is_entity(skill)
        hit_model = skill.get("hitModel", {})
        complete_tick_model = bool(hit_model.get("tickFrequency") and hit_model.get("lifetime"))
        cooldown = skill.get("baseCooldown", 0) > 0
        cooldown_path_detected = attack_cooldown_gate if attack else metadata_cooldown_gate
        cadence = bool(attack or entity or skill.get("usesAttackSpeed")
                       or skill.get("baseCastRate")
                       or cooldown and (skill.get("usesSkillHaste") or cooldown_path_detected))
        found = []
        if skill["kind"] == "active" and not (direct or attack):
            found.append(issue("no_direct_damage_formula", "unknown",
                "No direct/attack damage formula. This may be a valid support-only skill; inspect secondary effects before adding damage.",
                "engine/src/calc/skills/damage.rs: compute_skill_damage"))
        if skill["kind"] == "active" and (direct or attack) and not cadence:
            found.append(issue("missing_cadence_metadata", "metadata_gap",
                "Damage formula exists but no attack/entity/baseCastRate/usesAttackSpeed/usesSkillHaste cadence is encoded.",
                "engine/src/calc/build.rs: performance_from_stats"))
        if cooldown and not skill.get("usesSkillHaste") and (direct or attack):
            if cooldown_path_detected and not entity:
                found.append(issue("cooldown_cadence_implemented_unverified", "implemented_unverified",
                    "Source inspection detects a declared-cooldown ceiling and an action-rate path. This is an implementation candidate, not execution or game-parity proof.",
                    "engine/src/calc/build.rs: attack_action_rate_override / effective_rate", baseCooldown=skill["baseCooldown"]))
            elif not cooldown_path_detected and legacy_cadence:
                found.append(issue("cooldown_not_consumed_by_cast_rate", "unknown",
                    "Only the usesSkillHaste-selected cooldown branch was detected. Review the full rate path; entity actions can have independent cadence.",
                    "engine/src/calc/build.rs: eff_cast_min / eff_cast_max", baseCooldown=skill["baseCooldown"]))
        if attack and entity and "ad.attacks_per_second_min" in build and blender_gate:
            found.append(issue("entity_attack_uses_weapon_cadence", "implementation_gap",
                "Entity cadence is computed, but the attack DPS branch uses weapon APS; current explicit action-rate override is Blender-only.",
                "engine/src/calc/build.rs: AttackSkillInput, hit_dps_min"))
        skill_type = type_code.split("pub struct SkillSpec {", 1)[-1].split("\n}", 1)[0]
        if "cooldown" in skill and not re.search(r"pub cooldown\s*:", skill_type):
            found.append(issue("unparsed_top_level_cooldown", "metadata_gap",
                "SkillSpec has no top-level cooldown field; JSON value is ignored.",
                "engine/src/calc/types.rs: SkillSpec", value=skill["cooldown"]))
        if hit_model.get("tickFrequency") and not hit_model.get("lifetime"):
            found.append(issue("tick_frequency_without_lifetime", "unknown",
                "Tick frequency alone does not establish per-cast hits. Generic hits_per_cast returns one without lifetime.",
                "engine/src/calc/build.rs: hits_per_cast"))
        if skill.get("effectDuration") and not hit_model and sid != "blender" and not entity and (direct or attack):
            found.append(issue("duration_without_contact_model", "unknown",
                "effectDuration is not a generic damage-object lifetime. Description and object behavior must establish whether it affects damage, control or another effect.",
                "engine/src/calc/build.rs: hits_per_cast"))
        if blender_gate and attack and sid != "blender":
            found.append(issue("physical_skill_modifiers_blender_only", "implementation_gap",
                "Physical/tag skill damage and subtree attack_rating_pct are gated off on this attack path.",
                "engine/src/calc/skills/attack.rs: apply_skill_physical_modifiers"))
        proc = skill.get("proc")
        if proc:
            target = by_name.get((cid, normalize_name(proc.get("target", ""))))
            if target is None:
                found.append(issue("proc_target_not_found", "metadata_gap",
                    "Proc target does not resolve to a skill name in the owning class.",
                    "engine/src/calc/build.rs: proc_target_damage", target=proc.get("target")))
            elif not has_direct_formula(target):
                found.append(issue("proc_target_no_direct_formula", "unknown",
                    "Target resolves but lacks direct spell damage. The current proc target path may need attack or custom effect handling.",
                    "engine/src/calc/build.rs: proc_target_damage", targetId=target["id"]))
            if fixed_proc_rate and proc.get("trigger") != "on_kill":
                found.append(issue("proc_rate_fixed_one", "implementation_gap",
                    "Non-on-kill proc rate is one trigger/s rather than the main action/hit cadence.",
                    "engine/src/calc/build.rs: proc rate", trigger=proc.get("trigger")))

        for node in skill.get("subskills", []):
            node_id = sid + ":" + node["id"]
            effects, proc = node.get("effects", {}), node.get("proc", {})
            proc_effects = proc.get("effects", {})
            keys = effect_keys(effects) | effect_keys(proc_effects)
            node_found = []
            stat_records = []
            for key in sorted(keys):
                treatment = "aggregated_stat; precise game semantics unverified"
                if key in unresolved:
                    treatment = "explicit unresolved conversion source; no damage contribution"
                    node_found.append(issue("unresolved_conversion_source", "implementation_gap",
                        treatment, "engine/src/calc/skills/conversion.rs: CONVERSIONS", key=key))
                elif key in scoped and key not in ELEMENT_BREAK_KEYS and not references[key]:
                    treatment = "scoped stat has no literal engine consumer"
                    node_found.append(issue("scoped_key_without_consumer", "unknown",
                        "No consumer found in production calculation sources. Spatial/utility stats need not directly change single-target DPS; dynamic readers must be reviewed separately.",
                        "engine/src/calc/stats/attributes.rs: apply_subskill_aggregation", key=key,
                        spatialOrUtility=key in GEOMETRY_KEYS))
                elif blender_gate and attack and sid != "blender" and key in {"physical_skill_damage", "physical_skill_damage_more", "attack_rating_pct"}:
                    treatment = "aggregated but attack consumer gated to Blender"
                    node_found.append(issue("physical_skill_modifier_not_consumed", "implementation_gap",
                        treatment, "engine/src/calc/skills/attack.rs: apply_skill_physical_modifiers", key=key))
                elif key == "of_total_damage" and attack and "input.of_total_damage" not in attack_code:
                    treatment = "spell/element member only; physical attack member has no consumer"
                    node_found.append(issue("of_total_damage_missing_physical_member", "implementation_gap",
                        treatment, "engine/src/calc/skills/attack.rs: compute_attack_skill_damage", key=key))
                elif key == "skill_haste" and legacy_cadence and not skill.get("usesSkillHaste"):
                    if cooldown and cooldown_path_detected and not entity:
                        treatment = "cooldown/haste path detected; execution and game cadence unverified"
                        node_found.append(issue("haste_cadence_implemented_unverified", "implemented_unverified",
                            treatment, "engine/src/calc/build.rs: attack_action_rate_override / effective_rate", key=key))
                    else:
                        node_found.append(issue("haste_cadence_unresolved", "unknown",
                            "No non-entity declared-cooldown path is established for this owner. Verify deployment uptime, a separate trigger, missing metadata or another effect; absence of a direct DPS response is not proof the note is ignored.",
                            "engine/src/calc/build.rs: eff_cast_min / eff_cast_max", key=key))
                elif key == "skill_duration" and not complete_tick_model and sid != "blender":
                    node_found.append(issue("duration_without_complete_hit_model", "unknown",
                        "No complete repeat-hit model consumes this duration for DPS. It may instead affect crowd control or summon uptime.",
                        "engine/src/calc/build.rs: hits_per_cast", key=key))
                stat_records.append({
                    "key": key, "scope": "skill" if key in scoped else "shared",
                    "definedInGameConfig": key in definitions,
                    "base": effects.get("base", {}).get(key),
                    "perRank": effects.get("perRank", {}).get(key),
                    "procBase": proc_effects.get("base", {}).get(key),
                    "procPerRank": proc_effects.get("perRank", {}).get(key),
                    "treatment": treatment, "literalReferences": references[key],
                    "tagRule": affixes.get(key),
                })
            if proc:
                if proc_effects and weighted_proc_stats and not (sid == "blender" and blender_proc_exemption):
                    node_found.append(issue("proc_approximated_as_weighted_stat", "unknown",
                        "Chance-weighted effects enter main stats. Separate effect type, timing, overlap, duration and trigger uptime are not established by this representation.",
                        "engine/src/calc/subskill.rs: aggregate_subskill_stats"))
                if not proc_effects and not proc.get("appliesStates") and not proc.get("target") and sid != "blender":
                    node_found.append(issue("proc_chance_without_effect_metadata", "metadata_gap",
                        "Proc defines chance/trigger but no effect, target or applied state. Description remains unknown until explicit behavior is verified and encoded.",
                        "engine/src/calc/subskill.rs: aggregate_subskill_stats"))
                if proc.get("target"):
                    target = by_name.get((cid, normalize_name(proc["target"])))
                    if target is None:
                        node_found.append(issue("proc_target_not_found", "metadata_gap",
                            "Subskill proc target does not resolve to a same-class skill name.",
                            "engine/src/calc/build.rs: proc_target_damage", target=proc["target"]))
                    if fixed_proc_rate and proc.get("trigger") != "on_kill":
                        node_found.append(issue("proc_rate_fixed_one", "implementation_gap",
                            "Targeted subskill on-hit/on-cast proc is evaluated at one trigger/s.",
                            "engine/src/calc/build.rs: sub_proc rate"))
            if MECHANIC_DESCRIPTION.search(node.get("description", "")):
                node_found.append(issue("description_mechanics_unverified", "unknown",
                    "Description mentions timing, spatial, stacking or conditional behavior. No numeric behavior is inferred from this text.",
                    "subskill description"))
            node_records.append({
                "classId": cid, "skillId": sid, "id": node_id, "metadata": node,
                "gameValidation": "unknown", "statTreatment": stat_records,
                "tagChange": tag_changes.get(sid, {}).get(node["id"]), "findings": node_found,
            })
            add_findings(skill, node_found, node_id)
        records.append({
            "classId": cid, "id": sid, "source": locations[(cid, sid)],
            "metadata": {key: value for key, value in skill.items() if key != "subskills"},
            "gameValidation": "unknown",
            "encodedModel": {"directFormula": direct, "attackFormula": attack,
                             "entity": entity, "cadenceMetadata": cadence,
                             "completeTickModel": complete_tick_model,
                             "cooldownPathDetected": cooldown and cooldown_path_detected and not entity},
            "findings": found,
            "subskills": [sid + ":" + node["id"] for node in skill.get("subskills", [])],
        })
        add_findings(skill, found)

    class_rows = []
    for cid in sorted({skill["classId"] for skill in skills}):
        group = [skill for skill in skills if skill["classId"] == cid]
        active = [skill for skill in group if skill["kind"] == "active"]
        class_rows.append({
            "classId": cid, "skills": len(group), "active": len(active),
            "directFormulas": sum(has_direct_formula(skill) for skill in active),
            "attackFormulas": sum(has_attack_formula(skill) for skill in active),
            "subtrees": sum(bool(skill.get("subskills")) for skill in group),
            "nodes": sum(len(skill.get("subskills", [])) for skill in group),
        })
    snapshots = data_paths + [config_path, affix_path, changes_path] + source_paths
    return {
        "schemaVersion": 1,
        "scope": "Complete class skill/subskill metadata and static engine coverage; not proof of game parity.",
        "statusMeaning": {
            "implemented_unverified": "A candidate implementation path is present in source; execution and game parity still require independent evidence.",
            "implementation_gap": "A specific currently checked source path omits the encoded mechanic.",
            "metadata_gap": "Required field/target/effect representation is missing or inconsistent.",
            "unknown": "Needs runtime/game evidence; absence of direct DPS is not automatically an error.",
        },
        "totals": {"classes": len(class_rows), "skills": len(skills),
                   "active": sum(row["active"] for row in class_rows),
                   "subtrees": sum(row["subtrees"] for row in class_rows), "nodes": len(node_records)},
        "findingCounts": dict(sorted(collections.Counter(row["code"] for row in findings).items())),
        "sourceSha256": {str(path.relative_to(root)): hashlib.sha256(path.read_bytes()).hexdigest()
                         for path in snapshots},
        "classes": class_rows, "skills": records, "subskills": node_records,
        "findings": sorted(findings, key=lambda row: (row["status"], row["code"], row["classId"],
                                                     row["skillId"], row.get("nodeId", ""))),
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, help="Write JSON to this file instead of stdout.")
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1],
                        help="Repository root (defaults to this script's repository).")
    args = parser.parse_args()
    try:
        text = json.dumps(audit(args.root.resolve()), indent=2, ensure_ascii=False) + "\n"
        if args.output:
            args.output.write_text(text, encoding="utf-8")
        else:
            sys.stdout.write(text)
    except (OSError, ValueError, KeyError, TypeError) as error:
        parser.exit(2, f"Skill coverage audit failed: {error}\n")


if __name__ == "__main__":
    main()
