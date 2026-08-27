## Features
- Six white ring bases (Bronze Ring, Iron Ring, Ring, Socket Ring, Primal Ring, Golden Ring)
- Five white amulet bases (Amulet, Locket, Talisman, Carcanet, Necklace)
- Item-granted skill ranks are rollable too (issue #136): "Stat Rolls" also lists every ranged "+[X-Y] to <Skill>" bonus the item grants
- The affix picker lists one entry per affix family instead of one per tier
- Affix pools per item type (data mined)
- Roll sliders in the gear slot editor (issue #136)
- The gear slot configure panel is a single column of collapsible sections with an "Expand all / Collapse all" bar
- Cleaner rows in the gear slot editor
- Season 9 support removed
- Dropdowns (class, season, sort, random skill / element pickers) now match the item picker lists
- Items that roll "+X to Random Skill Element" (Phantom's Step, Captain's Anchor, Crow's Whisper, etc.) get an element picker in the gear slot, and the ranks land on the picked element's skills

## Fixes
- Runeword presets were missing on white caster weapons (wands, canes, staves, tomes, spellblades), which were all typed "Spell" and matched no runeword
- Wooden Shield, Buckler, Aegis and Monarch had no socket data, so no shield runeword ever showed up on them
- Lone Mystic runeword was an empty stub: no base types, no stats
- Four affix families now count towards the build instead of sitting under "Not Yet Supported"
- Rolled affixes get their own "Affixes" heading in item tooltips, so they no longer run together with the section above
- Affix rows in the gear slot editor lead with the stat they grant and keep the roll name in brackets after it instead of showing only the name
- Item tooltips dropped the text of any affix whose value is printed without brackets
- Item tooltips showed the affix range instead of the value the roll actually landed on
- Orb of Frost: casts once per its 1.75 s cooldown and speeds up with Skill Haste instead of Faster Cast Rate
- Magic Skill Damage now means the five elements (fire, cold, lightning, poison, arcane): "+X to Magic Skill Damage" from items and the tree is a flat bonus to every elemental skill
- Heroic and Satanic Set rarity colors were swapped everywhere: Heroic is green, Satanic Set is lime
