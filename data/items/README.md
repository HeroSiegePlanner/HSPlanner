Every single mod exist in [game-config.json](../game-config.json) so if you are looking for add more items or edit existing one please check this file.
## Relics

`relics.json` was refreshed from https://hero-siege-helper.vercel.app/relics on
2026-09-12 (155 relics). Ranges retain the source's tier 1 and tier 10 endpoints,
including descending negative values. Stable item IDs are preserved for saves.

Refresh with `python3 tools/update_relics.py` (or pass a saved HTML page for an
offline import). The importer validates all existing names and every stat line
before writing. The page lists Bomb twice; both effects belong to one item.

Relic skill links and formulas were checked against
[HS Helper abilities](https://hero-siege-helper.vercel.app/abilities) on 2026-09-14
(105 skills). Descriptions use the site's ability text and talent translations;
missing source descriptions remain empty. These definitions live in
`data/item-granted-skills.json`. Proc `grantedSkill` metadata links a name and
rank range to that catalog without adding a permanent `skillBonuses` entry or
enabling a simulated cast. The relic importer preserves these links on refresh.

Skill formulas are stored as skill-specific `uniqueEffects` and displayed under
Granted Skill Effects, not added to character stats. Blazing Trail occurs twice
in the ability data with the same ID; its existing class-skill formula agrees
with the relics page and is retained. Relic-specific
active/follower/proc damage is descriptive where the engine has no corresponding
skill simulation; proc chance uses its maximum
when the source supplies a range. The source provides endpoints, not individual
intermediate tiers; the tier editor interpolates these ranges.
