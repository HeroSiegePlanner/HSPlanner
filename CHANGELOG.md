## Fixes
- Fix `Tire Fire` tags
- 143 weapons get their real base damage and attacks per second (from hero-siege-helper data) - attack skills with them no longer fall back to unarmed 2-6 damage, and the misfiled "+X Attacks per Second" implicit lines moved to the base Attacks/sec row
- Suggest Nodes works for attack skills - the optimizer only knew the spell damage formula, so melee builds scored 0 DPS and every node looked like no improvement
- Left panel "Hit damage" now shows attack skills' per-hit range (it read only the spell breakdown, so attacks showed a dash next to a real Hit DPS)
- Skill details no longer show a flat "Physical damage" range - melee skills scale with the equipped weapon, so the fixed numbers were misleading
- Amulet of Colosseum implicit is % Increased Attack Speed like in the game, not flat attacks per second
- Weapon-gated tree nodes (e.g. "while wielding a wand", "when using a Shield/Two Handed Weapon/Bow") now apply only with the matching weapon equipped - previously some applied always and others not at all
