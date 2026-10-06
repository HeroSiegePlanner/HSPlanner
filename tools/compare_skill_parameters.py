#!/usr/bin/env python3
"""Compare extracted Hero Siege constructor metadata with planner skill data.

Usage: python3 tools/compare_skill_parameters.py extracted-skills.json --output report.json

This is a read-only, version-pinned candidate report, never runtime validation.
Extract input with extract_skill_parameters.py. Missing constructor writes remain
unknown: defaults, rank offsets, formula stages and unit conversions are not inferred.
"""

import argparse
import collections
import hashlib
import json
import math
from pathlib import Path
import re

from extract_skill_parameters import KNOWN_SHA256, PREFIX


ABILITY_ALIASES = {
    ("bard", "crowdDiver"): "crowd_dive",
    ("illusionist", "sandGuardian"): "sand_guardians",
    ("illusionist", "gravitationalSlam"): "gravitation_slam",
    ("necromancer", "raiseSkeletonWarrior"): "raise_skeleton",
    ("necromancer", "skeletonMastery"): "summon_mastery",
    ("nomad", "sunRay"): "healing_sunrays",
    ("pirate", "grenado"): "grenade_jump",
    ("pirate", "explosiveBullet"): "explosive_bullets",
    ("pirate", "freezeChainShot"): "freezing_chain_shot",
    ("prophet", "raven"): "summon_raven",
    ("prophet", "entColossus"): "summon_ent_colossus",
    ("prophet", "spiritOfForest"): "summon_spirit_of_the_forest",
    ("prophet", "spiritOfHawk"): "storm_hawk",
    ("prophet", "spiritOfEnt"): "spirit_of_the_ent",
    ("prophet", "spiritOfWendigo"): "spirit_of_the_wendigo",
    ("viking", "monsterThrow"): "throw",
    ("white_mage", "chainOfHolyLight"): "chain_of_holy_lightning",
}

METADATA_FIELDS = {
    "abilityCooldown": "baseCooldown",
    "abilityDuration": "effectDuration",
    "abilityCastSpeed": "baseCastRate",
    "abilityManaCost": "manaCostFormula.base",
    "abilityManaCostMult": "manaCostFormula.perLevel",
}
DIRECT_DAMAGE_LABELS = {
    "tal_damage_magic": "arcane",
    "tal_damage_fire": "fire",
    "tal_damage_cold": "cold",
    "tal_damage_lightning": "lightning",
    "tal_damage_poison": "poison",
    "tal_damage_physical": "physical",
}
ATTACK_LABELS = {
    "tal_attack_damage": "weaponDamagePct",
    "tal_attack_rating": "attackRatingPct",
    "tal_critical_chance": "criticalChance",
    "tal_critical_damage": "criticalDamage",
}


def normalized(value):
    return re.sub(r"[^a-z0-9]", "", value.lower())


def get_path(record, path):
    for part in path.split("."):
        if not isinstance(record, dict) or part not in record:
            return None
        record = record[part]
    return record


def field_record(fields, name):
    rows = [field for field in fields if field["field"] == name]
    if len(rows) > 1:
        raise ValueError(f"Ambiguous repeated constructor field: {name}")
    if not rows:
        return {"field": name, "value": None, "status": "missing_assignment"}
    row = dict(rows[0])
    row["status"] = "unresolved_assignment" if row.get("value") is None else "resolved_literal"
    return row


def numeric(value):
    return isinstance(value, (int, float)) and not isinstance(value, bool) and math.isfinite(value)


def comparison(skill, path, source, *, descriptor=None, coefficient=None):
    raw, planner = source.get("value"), get_path(skill, path)
    candidates = [{"interpretation": "raw_literal", "value": raw}] if numeric(raw) else []
    if descriptor == "tal_attack_damage" and coefficient == "base" and numeric(raw):
        candidates.append({"interpretation": "neutral_weapon_100_plus_literal", "value": 100 + raw})
    matches = [row["interpretation"] for row in candidates
               if numeric(planner) and math.isclose(row["value"], planner, rel_tol=0, abs_tol=1e-9)]
    if not numeric(raw):
        status = "unknown_game_value"
    elif not numeric(planner):
        status = "missing_planner_value"
    elif matches:
        status = "candidate_match"
    else:
        status = "candidate_mismatch"
    return {
        "plannerPath": path, "plannerValue": planner, "gameField": source,
        "descriptor": descriptor, "coefficient": coefficient,
        "candidateGameValues": candidates, "matchingInterpretations": matches,
        "status": status, "runtimeValidation": "unknown",
    }


def compare_skill(skill, source):
    fields = source["fields"]
    comparisons = [comparison(skill, path, field_record(fields, field))
                   for field, path in METADATA_FIELDS.items()]
    coefficients = []
    for index in range(1, 5):
        names = [f"abilityDescription{index}", f"abilityStartingValue{index}", f"abilityValue{index}"]
        if not any(field["field"] in names for field in fields):
            continue
        description, base, slope = [field_record(fields, name) for name in names]
        label = description.get("value")
        target = None
        if label in DIRECT_DAMAGE_LABELS and skill.get("damageType") == DIRECT_DAMAGE_LABELS[label]:
            target = "damageFormula"
        elif label in ATTACK_LABELS and skill.get("attackKind") == "attack":
            target = "attackScaling." + ATTACK_LABELS[label]
        coefficients.append({
            "index": index, "descriptor": description, "base": base, "perRank": slope,
            "candidatePlannerFormula": target, "runtimeValidation": "unknown",
        })
        if target:
            comparisons.append(comparison(skill, target + ".base", base,
                                          descriptor=label, coefficient="base"))
            comparisons.append(comparison(skill, target + ".perLevel", slope,
                                          descriptor=label, coefficient="perRank"))
    if get_path(skill, "attackScaling.weaponDamagePct") is not None and not any(
            row["descriptor"].get("value") == "tal_attack_damage" for row in coefficients):
        comparisons.append({
            "plannerPath": "attackScaling.weaponDamagePct", "status": "missing_weapon_descriptor",
            "plannerValue": get_path(skill, "attackScaling.weaponDamagePct"),
            "runtimeValidation": "unknown",
        })
    return comparisons, coefficients


def build_report(extracted, planner_skills):
    if extracted.get("sha256") != KNOWN_SHA256:
        raise ValueError("Unsupported executable fingerprint in extraction")
    classes = {}
    by_id = {}
    candidates = collections.defaultdict(set)
    for skill in planner_skills:
        key = (skill["classId"], skill["id"])
        if key in by_id:
            raise ValueError(f"Duplicate planner identity: {key}")
        by_id[key] = skill
        norm_class = normalized(skill["classId"])
        if norm_class in classes and classes[norm_class] != skill["classId"]:
            raise ValueError(f"Ambiguous normalized class: {norm_class}")
        classes[norm_class] = skill["classId"]
        for name in (skill["id"], skill["name"]):
            candidates[(skill["classId"], normalized(name))].add(key)
    seen, source_seen, rows = set(), set(), []
    for source in extracted["skills"]:
        function, ability_id = source["class_function"], source["abilityId"]
        if not function.startswith(PREFIX) or not isinstance(ability_id, str):
            raise ValueError("Constructor identity is missing or unsupported")
        class_id = classes.get(normalized(function[len(PREFIX):]))
        if class_id is None:
            raise ValueError(f"Unknown constructor class: {function}")
        identity = field_record(source["fields"], "abilityId")
        if identity.get("value") != ability_id:
            raise ValueError(f"Constructor identity disagrees with abilityId assignment: {ability_id}")
        source_key = (class_id, ability_id)
        if source_key in source_seen:
            raise ValueError(f"Duplicate constructor identity: {source_key}")
        source_seen.add(source_key)
        alias = ABILITY_ALIASES.get(source_key)
        if alias is not None:
            key, method = (class_id, alias), "curated_alias"
            if key not in by_id:
                raise ValueError(f"Alias target missing from planner: {key}")
        else:
            matches = candidates.get((class_id, normalized(ability_id)), set())
            if len(matches) != 1:
                raise ValueError(f"Expected one exact normalized identity for {source_key}; found {len(matches)}")
            key, method = next(iter(matches)), "exact_normalized_class_plus_id_or_name"
        if key in seen:
            raise ValueError(f"Multiple constructors map to planner identity: {key}")
        seen.add(key)
        skill = by_id[key]
        comparisons, coefficients = compare_skill(skill, source)
        rows.append({
            "classId": class_id, "skillId": skill["id"], "name": skill["name"],
            "abilityId": ability_id, "mappingMethod": method, "identityEvidence": identity,
            "function": function, "functionAddress": source.get("function_address"),
            "plannerSource": skill.get("_source"), "runtimeValidation": "unknown",
            "comparisons": comparisons, "coefficients": coefficients,
            "constructorFields": [field_record(source["fields"], field["field"]) for field in source["fields"]],
        })
    if missing := sorted(set(by_id) - seen):
        raise ValueError(f"Planner skills missing constructor mappings: {missing}")
    rows.sort(key=lambda row: (row["classId"], row["skillId"]))
    return {
        "schemaVersion": 1, "binarySha256": extracted["sha256"], "runtimeValidation": "unknown",
        "scope": "Complete constructor/planner identity bijection and candidate literal comparisons only",
        "limitations": [
            "Equal literals are not runtime/formula validation; mismatches are investigation candidates.",
            "Missing/unresolved writes stay unknown; no constructor defaults or rank offsets inferred.",
            "Cast speed versus base cast rate and mana base/slope unit/indexing equivalence is unverified.",
            "Weapon raw intercept and intercept plus neutral 100 are alternative candidates, not proven stages.",
            "Descriptions select coefficient slots only; passive recipients, subtrees, collisions and DPS are not audited.",
        ],
        "counts": {
            "classes": len(classes), "plannerSkills": len(by_id), "constructors": len(source_seen),
            "mappedSkills": len(rows), "curatedAliases": sum(row["mappingMethod"] == "curated_alias" for row in rows),
            "comparisonStatuses": dict(sorted(collections.Counter(
                item["status"] for row in rows for item in row["comparisons"]).items())),
            "unresolvedConstructorAssignments": sum(field["status"] == "unresolved_assignment"
                for row in rows for field in row["constructorFields"]),
        },
        "skills": rows,
    }


def load_planner_skills(directory):
    skills, hashes = [], {}
    for path in sorted(directory.glob("*.json")):
        content = path.read_text(encoding="utf-8")
        hashes[str(path)] = hashlib.sha256(path.read_bytes()).hexdigest()
        for skill in json.loads(content):
            match = re.search(r'"id"\s*:\s*"' + re.escape(skill["id"]) + '"', content)
            skill["_source"] = {"file": str(path), "line": content.count("\n", 0, match.start()) + 1}
            skills.append(skill)
    if not skills:
        raise ValueError(f"No planner skills found in {directory}")
    return skills, hashes


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("extracted", type=Path)
    parser.add_argument("--data", type=Path, default=Path(__file__).resolve().parents[1] / "data/skills")
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    try:
        skills, hashes = load_planner_skills(args.data)
        report = build_report(json.loads(args.extracted.read_text(encoding="utf-8")), skills)
        report["inputSha256"] = {str(args.extracted): hashlib.sha256(args.extracted.read_bytes()).hexdigest(), **hashes}
        output = json.dumps(report, indent=2, ensure_ascii=False, allow_nan=False) + "\n"
        if args.output:
            args.output.write_text(output, encoding="utf-8")
        else:
            print(output, end="")
    except (ValueError, KeyError, TypeError, OSError) as error:
        parser.exit(2, f"Skill metadata comparison failed: {error}\n")


if __name__ == "__main__":
    main()
