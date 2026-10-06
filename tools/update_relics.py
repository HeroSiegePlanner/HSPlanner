#!/usr/bin/env python3
"""Refresh current relic data from HS Helper; retain IDs used by saved builds.

Usage: python3 tools/update_relics.py [downloaded-relics.html]
No dependencies outside Python's standard library. Unknown stat lines abort the
update so a website layout/schema change cannot silently erase game data.
"""
from html.parser import HTMLParser
from pathlib import Path
import json
import re
import sys
from urllib.request import urlopen

SOURCE = 'https://hero-siege-helper.vercel.app/relics'
ROOT = Path(__file__).resolve().parents[1]
NUMBER = r'-?\d+(?:\.\d+)?'
RANGE = re.compile(r'\[(' + NUMBER + r')-(' + NUMBER + r')\]')
STAT_NAMES = {
    '+{}% Increased Attack Speed': 'increased_attack_speed',
    '+{}% Increased Critical Strike Damage': 'crit_damage',
    '+{}% Increased Critical Strike Chance': 'crit_chance',
    '+{}% Increased Movement Speed': 'movement_speed',
    '{}% Increased Movement Speed': 'movement_speed',
    '+{}% Increased Dexterity': 'increased_dexterity',
    'Mana Increased by {}%': 'increased_mana',
    'Life Increased by {}%': 'increased_life',
    'Replenish Mana {}%': 'mana_replenish_pct',
    'Replenish Life {}%': 'life_replenish_pct',
    'Mana Costs decreased by {}%': 'mana_cost_reduction',
    '+{} to All Attributes': 'all_attributes',
    '+{} to Light Radius': 'light_radius',
    '+{}% Increased Magic Find': 'magic_find',
    '+{} to Vitality': 'to_vitality',
    '+{} to Strength': 'to_strength',
    '+{} to Dexterity': 'to_dexterity',
    '+{} to Energy': 'to_energy',
    '+{} to Life': 'life',
    '+{} to Armor': 'to_armor',
    '+{} to Flat Physical Damage': 'additive_physical_damage',
    'Physical Damage Taken Reduced by {}%': 'physical_damage_reduction',
    'Magic Damage Taken Reduced by {}%': 'magic_damage_reduction',
    '+{}% Life stolen per Hit': 'life_steal',
    'Magic Skill Damage increased by {}%': 'magic_skill_damage',
    '{} to All Skills': 'all_skills',
    '+{}% Chance for a Deadly Blow': 'deadly_blow',
    '+{}% Extra Gold Dropped From Kills': 'gold_find',
    '+{}% Faster Hit Recovery': 'faster_hit_recovery',
    '+{}% Faster Cast Rate': 'faster_cast_rate',
    '+{}% increased Damage Returned to the Attacker': 'damage_return',
    '+{}% of target Defense ignored': 'defense_ignored',
    '+{}% Chance for a Crushing Blow': 'crushing_blow_chance',
    'Attack Damage increased by {}%': 'attack_damage',
    'Explosion Damage increased by {}%': 'explosion_damage',
    '+{}% to All Resistances': 'all_resistances',
    'Skill Movement Diminish decreased by {}%': 'reduced_movement_diminish',
    '{}% of Vitality added as Magic Skill Damage': 'vitality_converted_to_magic_skill_dmg',
    '{}% of Energy added as Magic Skill Damage': 'energy_converted_to_magic_skill_dmg',
    'Skill Haste Increased by {}%': 'skill_haste',
    '+{}% Decreased Merchant Prices': 'decreased_merchant_prices',
    'Extra Damage to Stunned Monsters {}%': 'extra_damage_stunned',
    'Bleed Damage increased by {}%': 'increased_bleeding_damage',
}
for element in ('Fire', 'Cold', 'Lightning', 'Poison', 'Arcane'):
    STAT_NAMES['-{}% to Enemy ' + element + ' Resistance'] = 'ignore_' + element.lower() + '_res'
    STAT_NAMES['+{} to ' + element + ' Skills'] = element.lower() + '_skills'
    STAT_NAMES[element + ' Skill Damage increased by {}%'] = element.lower() + '_skill_damage'


class Text(HTMLParser):
    def __init__(self):
        super().__init__()
        self.skip = 0
        self.parts = []

    def handle_starttag(self, tag, attrs):
        if tag in ('style', 'script'):
            self.skip += 1

    def handle_endtag(self, tag):
        if tag in ('style', 'script'):
            self.skip -= 1

    def handle_data(self, value):
        if not self.skip and value.strip():
            self.parts.append(' '.join(value.split()))


def text(html):
    parser = Text()
    parser.feed(html)
    return parser.parts


def records_from(html):
    headings = list(re.finditer(r'<h([1-6])\b[^>]*>(.*?)</h\1>', html, re.S))
    records = {}
    for ix, match in enumerate(headings):
        name = ' '.join(text(match[2]))
        end = headings[ix + 1].start() if ix + 1 < len(headings) else len(html)
        body = text(html[match.end():end])
        if 'Relic' in body:
            records.setdefault(name, []).extend(body[body.index('Relic') + 1:])
    return records


def number(value):
    value = float(value)
    return int(value) if value.is_integer() else value


def update(item, name, lines):
    implicit, skills, procs, effects = {}, {}, [], []
    skill_name = name
    for line in lines:
        ranged = RANGE.search(line)
        pattern = RANGE.sub('{}', line)
        if ranged and pattern in STAT_NAMES:
            implicit[STAT_NAMES[pattern]] = [number(ranged[1]), number(ranged[2])]
        elif 'Chance ' in line and ' to cast ' in line:
            trigger = next((value for phrase, value in {
                'when Struck': 'when_struck', 'when Striking': 'on_hit',
                'when Attacking': 'on_attack', 'when Casting': 'on_cast',
                'after each Kill': 'on_kill',
            }.items() if phrase in line), None)
            if trigger is None:
                raise ValueError(f'Unknown trigger: {line}')
            chance = RANGE.match(line)
            chance = number(chance[2]) if chance else number(line.split('%')[0])
            description = 'cast ' + line.split(' to cast ', 1)[1]
            skill_name, rank = description[5:].rsplit(' Level ', 1)
            ranged_rank = RANGE.fullmatch(rank)
            rank = [number(ranged_rank[1]), number(ranged_rank[2])] if ranged_rank else number(rank)
            proc = dict(trigger=trigger, chance=chance, description=description)
            proc['grantedSkill'] = dict(name=skill_name, rank=rank)
            if RANGE.match(line):
                proc['details'] = line
            procs.append(proc)
        elif 'per level]' in line:
            effects.append(f'{skill_name}: {line}')
        elif re.fullmatch(r'\+(?:\[[^]]+\]|\d+) to .+', line):
            amount, skill_name = line[1:].split(' to ', 1)
            ranged = RANGE.fullmatch(amount)
            skills[skill_name] = [number(ranged[1]), number(ranged[2])] if ranged else number(amount)
        else:
            raise ValueError(f'Unrecognized relic line for {name}: {line}')
    for field in ('implicit', 'skillBonuses', 'procs', 'uniqueEffects'):
        item.pop(field, None)
    item['name'] = name
    for field, value in [('implicit', implicit), ('skillBonuses', skills), ('procs', procs), ('uniqueEffects', effects)]:
        if value:
            item[field] = value
    return item


def main():
    html = Path(sys.argv[1]).read_text() if len(sys.argv) > 1 else urlopen(SOURCE, timeout=60).read().decode()
    records = records_from(html)
    path = ROOT / 'data/items/relics.json'
    existing = json.loads(path.read_text())
    aliases = {"Devil's Horn": 'Devil Horn'}
    by_name = {aliases.get(item['name'], item['name']): item for item in existing}
    missing = by_name.keys() - records.keys()
    if missing or len(records) < 150:
        raise ValueError(f'Incomplete source; missing existing relics: {sorted(missing)}')
    canonical = {re.sub(r'[^a-z0-9]', '', skill['name'].lower()): skill['name']
                 for skill in json.loads((ROOT / 'data/item-granted-skills.json').read_text())}
    updated = []
    for name, lines in records.items():
        item = by_name.get(name, dict(
            id='relic_relic_' + re.sub('[^a-z0-9]+', '_', name.lower()).strip('_'),
            name=name, baseType='Relic', slot='relic_1', rarity='relic', sockets=0, maxSockets=0,
        ))
        item = update(item, name, lines)
        if 'skillBonuses' in item:
            item['skillBonuses'] = {canonical.get(re.sub(r'[^a-z0-9]', '', name.lower()), name): value
                                    for name, value in item['skillBonuses'].items()}
        for proc in item.get('procs', []):
            grant = proc['grantedSkill']
            grant['name'] = canonical.get(re.sub(r'[^a-z0-9]', '', grant['name'].lower()), grant['name'])
        if 'uniqueEffects' in item:
            item['uniqueEffects'] = [
                canonical.get(re.sub(r'[^a-z0-9]', '', name.lower()), name) + ': ' + formula
                for name, formula in (effect.split(': ', 1) for effect in item['uniqueEffects'])
            ]
        updated.append(item)
    updated.sort(key=lambda item: item['id'])
    assert len({item['id'] for item in updated}) == len(updated)
    path.write_text(json.dumps(updated, indent=2, ensure_ascii=False) + '\n')
    print(f'Updated {len(updated)} relics from {SOURCE}; {len(updated) - len(existing)} added.')


if __name__ == '__main__':
    main()
