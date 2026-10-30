## Interface

- Fix version badge to match with version stream: Stable for stable stream, opt-in beta for beta stream and dev for local
## Calculations

- "+X to Melee, Ranged, Area of Effect and Orbit Skills" on items now raise the ranks of skills with that tag; before, they did nothing.
- Mana Costs Decreased on items and the Manaleak–Manahunger affixes now reduce mana costs instead of doing nothing or adding flat Mana.
- Gurag's Soulgem gives Life Replenish; its stat was misnamed and did nothing.
- Large negative damage stats (Belt of Skulls' -666% Magic Skill Damage, Lucifer's Crown's damage per resistance point with -300% all resistances) bring a hit down to zero instead of making damage and DPS negative.
- Satanic crystals on satanic, heroic, angelic and unholy charms now count; before, only crystals on equipment did, so a Flat Life crystal on a charm left Life unchanged. Potions no longer offer a crystal slot, as they cannot take one (#214).
- Powerfunnel and Powerfunneled Lifespan convert 1% and 7.5% of the Attack Damage stat to Increased Life; they used to read the weapon's damage after Enhanced Damage and came out about seven times too strong (#215).
- Citadel's Increased Life Replenish per Colossus stack adds to the same pool as other Increased Life Replenish (e.g. Angel's Vibrance Aura) instead of multiplying it again (#216).
- Jagged Gauntlet (St. Ahto's Diamond Hands) works for spells and attacks and grows with its rank: each hit has its chance (4% per rank) to deal extra damage equal to your critical damage, raised by Jagged Gauntlet Damage (10% per rank), while your critical chance is halved and critical damage is capped at 40% + 20% per rank. Before, it only added critical chance, which spells never read. The proc formula follows the ability text, not traced game code (#217).
- Roll the Dice (High Roller's Deathcrown) turns on by itself once the crown's six rolled stats add up to more than 777, stars and pinned rolls included; the Config switch for it is gone (#218).
- Scorching Aura deals damage in the calculation. Before, it was never counted because it is an aura, not an active skill. As in the game, it pulses every 61 frames (about once a second) whatever your cast speed, hitting each enemy once per pulse. Rapid Burning shortens the pulse instead of adding cast rate (16 frames at rank 5). Increased Damaging Aura Effectiveness multiplies the pulse. Sixt Degree Burns and Searing Burst raise its subtree multiplier instead of fire skill damage, and Burning Soul adds 6% per rank for every 100 maximum life to that multiplier. With Scorching Aura as your aura, its DPS adds to your skills' Combined DPS, it is listed under Active skills in the sidebar (× clears the aura slot) and it has its own card among the per-skill DPS. A build that plays only Scorching Aura, with no active skill picked, shows its DPS and leads the Stats view with it.
- Runeword stats with a range can be set like item rolls: each ranged stat of the socketed runeword has its own slider under Stat Rolls, the calculation uses the chosen value instead of the whole range, and the item tooltip shows it. Changing the runeword clears the old values.
- Text edit shows the item's runeword and its stats under "Runeword:"; writing a single value with [custom] on a runeword stat sets that roll.
