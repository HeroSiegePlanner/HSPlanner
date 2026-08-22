## Features
- Random Skill Element: items that roll "+X to Random Skill Element" (Phantom's Step, Captain's Anchor, Crow's Whisper…) get an element picker in the gear slot, and the ranks land on the picked element's skills
- Season 10 heroic items get their stats: Ghost Armada, Jar of Parasites, Parasitic Heart, Ghastly Skull, Captain's Anchor, Parasite Loop, Skeleton Crew's Band, Blood Maggot Pendant, Grimtide's Necklace, Infected Grasp, Leviathan's Ribcage, Captain's Attire, Phantom's Step, Ghostplunderer's Marchers, Leviathan's Crown, Parasite Queen's Tiara, Overgrowth, Leviathan's Spine, Phantom Strike, Phantom Scimitar, Conjured Tentacle, Grimtide's Scimitar, Ethereal Musket
- Season 10 item skills (Will-O-Wisp, Ghost Crew, Phantom Momentum, Scarlet Sacrifice, Heart Surge, Spectral Scatter) show their descriptions in item tooltips
- Unholy slots: items with "Unholy" special effects list them as their own tooltip section
- Random Skill rolls: items that roll "+X to Random Skill" get a skill picker in the gear slot, and the ranks land on the picked skill
- Season 10 is now the default season. Everyone starts there unless they switch
- Item text editor: "Custom affixes" list — click a stat to insert a `[custom]` implicit line

## Fixes
- Satan's Unholy Bible and Belt of Infinite Wealth: "Physical Damage Taken Reduced by -10%" is a real 10% reduction, not a penalty
- Tree node tooltip: "This Node" now shows only the hovered node's own contribution; the nodes it would orphan stay under "With Cleanup" (it used to show the whole cleanup twice)
- Magic Skill Damage now means the five elements (fire, cold, lightning, poison, arcane): "+X to Magic Skill Damage" from items and the tree is a flat bonus to every elemental skill (it was dead on items and hit physical skills from the tree), and "Increased (Total) Magic Skill Damage" and "Flat Elemental Skill Damage" no longer buff physical skills
- Celestial Might now actually converts elemental skill damage into arcane instead of being ignored
- Engineer's Mini Drone: "+[1-2] to All Skills" is Marksman-only, as the item says
- Class set bonuses: all 43 "+X to All Skills (Class)" 4-set bonuses now only pay out for that class
- Item text editor: deleting or replacing a base implicit line now removes it instead of re-adding it on save
- Fix skipping node link #2113 
- Incarnation tree: Malicious Veins shows "+25% of Maximum life dealt as Arcane Damage"
- Incarnation tree: Vital Heart, First Aid, Rapid Mending and Vitalizing Charge are notables, as in game
- Crow's Whisper is 2-handed staff instead 1-handed wand
- C.Y.C.L.O.P.S. now pins gunner drones to 4 attacks/s
