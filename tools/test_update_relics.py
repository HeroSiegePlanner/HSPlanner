import unittest

from update_relics import records_from, update


class RelicImportTests(unittest.TestCase):
    def test_duplicate_cards_merge_and_ignore_styles_and_equipment(self):
        html = '''<h2>Wand</h2><div>Satanic Wand</div><div>+5 to Life</div>
        <h2>Bomb</h2><div>Relic</div><style>.css { color: red; }</style>
        <div>Explosion Damage increased by [5-50]%</div>
        <h3>Active</h3><h2>Bomb</h2><div>Relic</div><div>+1 to Naval Bomb</div>'''
        records = records_from(html)
        self.assertEqual(list(records), ['Bomb'])
        bomb = update({'id': 'stable_id'}, 'Bomb', records['Bomb'])
        self.assertEqual(bomb['id'], 'stable_id')
        self.assertEqual(bomb['implicit'], {'explosion_damage': [5, 50]})
        self.assertEqual(bomb['skillBonuses'], {'Naval Bomb': 1})

    def test_skill_damage_never_becomes_a_character_stat(self):
        item = update({'implicit': {'to_vitality': [3, 30]}}, 'Apple', [
            '+[2-50] to Apple Blast', '0 [+30 per level] Arcane Damage'])
        self.assertNotIn('implicit', item)
        self.assertEqual(item['uniqueEffects'], ['Apple Blast: 0 [+30 per level] Arcane Damage'])

    def test_negative_and_fractional_endpoints_keep_tier_order(self):
        item = update({}, 'Example', ['[-2--20]% Increased Movement Speed', 'Life Increased by [1.5-15]%'])
        self.assertEqual(item['implicit']['movement_speed'], [-2, -20])
        self.assertEqual(item['implicit']['increased_life'], [1.5, 15])

    def test_ranged_proc_chance_keeps_source_range_visible(self):
        item = update({}, 'Tooth', ['[1-6]% Chance when Striking to cast Stoned Level 1'])
        proc = item['procs'][0]
        self.assertEqual(proc['chance'], 6)
        self.assertIn('[1-6]%', proc['details'])
        self.assertEqual(proc['grantedSkill'], {'name': 'Stoned', 'rank': 1})

    def test_proc_skill_keeps_its_rank_range_without_becoming_a_passive_bonus(self):
        item = update({}, 'Frozen Orb', [
            '8% Chance when Striking to cast Chilling Strike Level [2-50]',
            '0 [+30 per level] Cold Damage'])
        self.assertEqual(item['procs'][0]['grantedSkill'], {
            'name': 'Chilling Strike', 'rank': [2, 50]})
        self.assertNotIn('skillBonuses', item)

    def test_unknown_source_stat_fails_instead_of_dropping_it(self):
        with self.assertRaises(ValueError):
            update({}, 'Example', ['New stat that needs a mapping [1-10]%'])


if __name__ == '__main__':
    unittest.main()
