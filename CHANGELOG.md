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
- The "to x" affixes (Sinister…Sacrilegious, Wishing…Anunnaki) claimed to grant "Increased Attack Speed when below 40% Maximum Life". Nothing else in the game data carries that stat and the engine never reads it, so the key was a guess against an unresolved description — cleared, and those affixes now sit under "Not Yet Supported" until the real stat is known
- Two affix groups each held two unrelated families under one id, so the picker collapsed them into a single nonsense row ("+[1-7] to x"). Split apart and given the item types the dump records: Sinister…Sacrilegious roll on chests, Wishing…Anunnaki on belts, gloves, helmets and weapons, Freljord ("Cannot be Frozen") on chests, while Tundra carries no item types anywhere and is hidden
- Codex affixes (Greed, Gluttony, Envy, Treasure, Darkness, Fear, Legion, Dwelling, Delirious, Heroic) were offered on gear. The decompile files them under `stat_codex_*` with no item types at all — gear rolls its own separate families for the same effects — so they are hidden from the picker, along with the equally item-typeless "N/A" summon-skill entries and Movement Phasing. Ticking "Show all affixes" still brings them back
- Unholy affixes leaked into the normal "Add Affix" list on every item. They are one flat pool of 74 unrelated stats rather than a tier ladder, so grouping mashed them into a single nonsense row ("+[1-800] to Strength · 74 tiers"). They are now offered only on the bases that actually roll them
- "+X to Random Skill Element" only worked when the base itself carried it (Phantom's Step and friends). Rolled as an affix it did nothing: no stat key, no element picker in the gear slot, and the engine only rerouted the implicit. All three are wired now — the affix lands on the picked element's skills like the implicit always did
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
