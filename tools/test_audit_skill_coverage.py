"""Regressions for coverage diagnostics after cooldown logic extraction."""

import unittest

from audit_skill_coverage import cooldown_path_candidates


class CooldownDetectionTests(unittest.TestCase):
    helper = '''
pub fn effective_cooldown(skill: &SkillSpec, stats: &StatMap) -> Option<f64> {
    let cooldown = skill.base_cooldown.unwrap_or(0.0) + r_max(rg(stats, "cooldown_added")).max(0.0);
    (cooldown > 0.5).then_some(cooldown)
}
'''
    shared = '''
fn attack_action_rate_override(spec: &SkillSpec, scoped: &StatMap) -> Option<Ranged> {
    let cooldown = super::skill_cost::effective_cooldown(spec, scoped)?;
}
let cooldown =
    active_skill.and_then(|s| super::skill_cost::effective_cooldown(s, main_scoped));
let effective_rate = if let Some(cooldown) = cooldown {
    let haste = stat("skill_haste");
};
'''

    def test_shared_helper_paths_are_detected(self):
        self.assertEqual(cooldown_path_candidates(self.shared, self.helper), (True, True))

    def test_missing_helper_does_not_establish_cooldown_path(self):
        self.assertEqual(cooldown_path_candidates(self.shared, ""), (False, False))

    def test_old_inline_paths_still_detected(self):
        inline = self.shared.replace("super::skill_cost::effective_cooldown(spec, scoped)",
                                     "spec.base_cooldown.filter(|cd| *cd > 0.0)")
        inline = inline.replace("super::skill_cost::effective_cooldown(s, main_scoped)", "s.base_cooldown")
        self.assertEqual(cooldown_path_candidates(inline, ""), (True, True))

    def test_missing_spell_rate_gate_is_not_a_spell_path(self):
        source = self.shared.replace("let effective_rate = if let Some(cooldown) = cooldown", "other")
        self.assertEqual(cooldown_path_candidates(source, self.helper), (False, True))


if __name__ == "__main__":
    unittest.main()
