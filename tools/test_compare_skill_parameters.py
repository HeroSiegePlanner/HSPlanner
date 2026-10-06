"""Identity and fail-closed coefficient comparison regressions."""

import unittest

from compare_skill_parameters import ABILITY_ALIASES, build_report
from extract_skill_parameters import KNOWN_SHA256, PREFIX


def planner(skill_id="fireball", name="Fireball", class_id="pyromancer", **fields):
    return {"id": skill_id, "name": name, "classId": class_id, **fields}


def constructor(ability_id="fireBall", class_name="Pyromancer", **fields):
    return {
        "class_function": PREFIX + class_name, "function_address": "0x1000", "abilityId": ability_id,
        "fields": [{"field": name, "value": value, "source": hex(0x1000 + index * 16),
                    "assignment": hex(0x1004 + index * 16) if value is not None else None}
                   for index, (name, value) in enumerate({"abilityId": ability_id, **fields}.items())],
    }


def report(skills, sources):
    return build_report({"sha256": KNOWN_SHA256, "skills": sources}, skills)


class CompareSkillParametersTests(unittest.TestCase):
    def test_curated_alias_and_normalized_identity_keep_source_evidence(self):
        result = report([planner(), planner("crowd_dive", "Crowd Dive", "bard")],
                        [constructor(), constructor("crowdDiver", "Bard")])
        self.assertEqual(result["counts"]["mappedSkills"], 2)
        self.assertEqual(result["counts"]["curatedAliases"], 1)
        bard = result["skills"][0]
        self.assertEqual((bard["skillId"], bard["abilityId"]), ("crowd_dive", "crowdDiver"))
        self.assertEqual(bard["identityEvidence"]["assignment"], "0x1004")
        self.assertEqual(len(ABILITY_ALIASES), 17)

    def test_source_and_planner_must_form_bijection(self):
        cases = [
            ([planner(), planner("extra", "Extra")], [constructor()]),
            ([planner()], [constructor(), constructor()]),
            ([planner()], [constructor("unknown")]),
            ([planner(), planner()], [constructor()]),
            ([planner()], [constructor("fireBall", "OtherClass")]),
        ]
        for skills, sources in cases:
            with self.subTest(sources=sources), self.assertRaises(ValueError):
                report(skills, sources)

    def test_normalization_collision_and_stale_alias_fail_closed(self):
        with self.assertRaisesRegex(ValueError, "found 2"):
            report([planner(), planner("other", "Fire Ball")], [constructor()])
        with self.assertRaisesRegex(ValueError, "Alias target missing"):
            report([planner("other", "Crowd Diver", "bard")], [constructor("crowdDiver", "Bard")])

    def test_fingerprint_and_identity_assignment_must_match(self):
        with self.assertRaisesRegex(ValueError, "fingerprint"):
            build_report({"sha256": "different", "skills": []}, [])
        source = constructor()
        source["fields"][0]["value"] = "other"
        with self.assertRaisesRegex(ValueError, "identity disagrees"):
            report([planner()], [source])

    def test_unknown_and_absent_writes_are_not_assumed_zero(self):
        result = report([planner(baseCooldown=0, effectDuration=0)],
                        [constructor(abilityCooldown=None)])["skills"][0]
        cooldown, duration = result["comparisons"][:2]
        self.assertEqual(cooldown["status"], "unknown_game_value")
        self.assertEqual(cooldown["gameField"]["status"], "unresolved_assignment")
        self.assertEqual(duration["gameField"]["status"], "missing_assignment")
        self.assertIsNone(duration["gameField"]["value"])
        self.assertEqual(duration["candidateGameValues"], [])

    def test_metadata_mismatches_preserve_raw_values_and_never_claim_runtime(self):
        result = report([planner(baseCooldown=5, effectDuration=5, baseCastRate=2,
                                 manaCostFormula={"base": 8, "perLevel": 1})],
                        [constructor(abilityCooldown=8, abilityDuration=5, abilityCastSpeed=1.75,
                                     abilityManaCost=8, abilityManaCostMult=0.75)])
        rows = {row["plannerPath"]: row for row in result["skills"][0]["comparisons"]}
        self.assertEqual(rows["baseCooldown"]["status"], "candidate_mismatch")
        self.assertEqual(rows["effectDuration"]["status"], "candidate_match")
        self.assertEqual(rows["baseCastRate"]["gameField"]["value"], 1.75)
        self.assertEqual(rows["manaCostFormula.perLevel"]["status"], "candidate_mismatch")
        self.assertEqual(result["runtimeValidation"], "unknown")
        self.assertTrue(all(row["runtimeValidation"] == "unknown" for row in rows.values()))

    def test_weapon_damage_uses_descriptor_slot_not_field_one(self):
        result = report([planner("strike", "Strike", damageType="poison", attackKind="attack",
                                 damageFormula={"base": 4, "perLevel": 14},
                                 attackScaling={"weaponDamagePct": {"base": 185, "perLevel": 35}})],
                        [constructor("strike", abilityDescription1="tal_damage_poison",
                                     abilityStartingValue1=4, abilityValue1=14,
                                     abilityDescription2="tal_attack_damage",
                                     abilityStartingValue2=85, abilityValue2=35)])["skills"][0]
        rows = {row["plannerPath"]: row for row in result["comparisons"]}
        weapon = rows["attackScaling.weaponDamagePct.base"]
        self.assertEqual(weapon["gameField"]["field"], "abilityStartingValue2")
        self.assertEqual(weapon["matchingInterpretations"], ["neutral_weapon_100_plus_literal"])
        self.assertEqual(rows["damageFormula.base"]["gameField"]["field"], "abilityStartingValue1")

    def test_critical_descriptor_cannot_validate_weapon_formula(self):
        result = report([planner("blade_strike", "Blade Strike", "nomad", attackKind="attack",
                                 attackScaling={"weaponDamagePct": {"base": 67.5, "perLevel": 22.5}})],
                        [constructor("bladeStrike", "Nomad", abilityDescription1="tal_critical_damage",
                                     abilityStartingValue1=67.5, abilityValue1=22.5)])["skills"][0]
        weapon = [row for row in result["comparisons"] if row["plannerPath"].startswith("attackScaling.weaponDamagePct")]
        self.assertEqual(len(weapon), 1)
        self.assertEqual(weapon[0]["status"], "missing_weapon_descriptor")
        self.assertEqual(result["coefficients"][0]["candidatePlannerFormula"], "attackScaling.criticalDamage")

    def test_different_element_and_unknown_labels_remain_unmapped(self):
        result = report([planner(damageType="fire", damageFormula={"base": 3, "perLevel": 5})],
                        [constructor(abilityDescription1="tal_damage_poison", abilityStartingValue1=3,
                                     abilityValue1=5, abilityDescription2=None, abilityValue2=8)])["skills"][0]
        self.assertTrue(all(row["candidatePlannerFormula"] is None for row in result["coefficients"]))
        self.assertFalse(any(row["plannerPath"].startswith("damageFormula") for row in result["comparisons"]))

    def test_repeated_constructor_writes_are_ambiguous(self):
        source = constructor(abilityCooldown=8)
        source["fields"].append({"field": "abilityCooldown", "value": 3})
        with self.assertRaisesRegex(ValueError, "Ambiguous repeated"):
            report([planner()], [source])


if __name__ == "__main__":
    unittest.main()
