## Features
- Import items from the game: paste a tooltip screenshot (Gear → Import screenshot) and the built-in OCR reads the name, implicit rolls, augment, sockets and granted skills into an editable item
- Tree node suggester rebuilt on the real calc engine
- Difficulty preset (Normal / Nightmare / Hell / Inferno) under Config → Character
- Torch of Shadow: pick which class its "+[1-3] to All Skills (Class)" rolled it only pays out on a build of that class
- Item procs that cast a class skill now add proc DPS, toggled under Config → Procs
- Proc-cast skills now count the points spent in their own subtree
- Tundra Hunter's Long Coat: its Set Sail proc buff now grants cold skill damage and mana replenish, toggled under Item Blessings
- Overloaded Dice: pick which skill its "+1 to Random Skill Sub Skills" rolled
- Rage stacks now working and each stack gives 5% Attack Speed

## Fixes
- Item database audited against the game's own item definitions
- 57 missing or wrong item sprites added (S10 uniques, base rings, relics and more)
- Weakening Precision (Frost Sunder) did nothing
- Tree-node suggester ignored stack payouts, so it scored every Rage node (max stacks, damage/attack speed per stack) as worth nothing
- Manahunger, Elemental Break and other Spell-branch tree notes no longer boost skills without the Spell tag
- "+X% Increased Spell Damage" tree notes did nothing
- Frost Sunder Onslaught works as intended
- Frost Sunder throws 4 icicles instead 1
- "Increased Magic Skills Damage per 750 points in Mana" (Soulburn Essence) did nothing
- Attack skills crit twice over
- Tree notes worded "Increased Damage" when wielding an Axe / Dual Wielding / using a Two Handed Weapon only scaled the weapon's own damage roll, so they decayed to nothing once flat physical
- Bard skill tree
