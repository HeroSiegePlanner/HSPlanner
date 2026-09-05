# Season 10 item refresh — 2026-09-05

Source: [Hero Siege Helper items](https://hero-siege-helper.vercel.app/items),
exported at `2026-09-05T18:33:06.635Z` from deployment
`dpl_6QxshWt8hpbfWizTjhTnBKvmowzR`.

- Item asset: `2k_8wgndfiuuw.js`, SHA-256 `f4c3d972032771a900f41e0b3c3826b580ac650b21bbcab9242099b1b7c1a420`.
- Stat dictionary: `32gwha2lys19e.js`, SHA-256 `147da54d0a2d9301aefa83421bd1224270da8139c97d6a1b5d3c960a18776d72`.
- Local source export: `outputs/01a071e5-9991-7162-81f0-e2c54cb0f4ba/hero-siege-items-2026-09-05.json`.

`items.patch.json` updates 1,096 existing non-Common records and adds 9 missing
equipment records. All 156 Common records remain identical to the base data.
Existing item IDs and elemental variant IDs are preserved so saved builds keep
their item references. Relic statistics, runewords, gems and runes are unchanged.

For non-Common items, missing socket information means `sockets: 0` and
`maxSockets: 0`. A source range beginning at zero still permits sockets up to its
positive maximum. An explicit zero maximum also prevents a forged socket bonus;
legacy sockets on such items are cleared when loading a build and ignored by the
calculation engine. Common socket rules are unchanged.

Numeric stats use the planner's existing stat keys and retain source ranges.
Class skill bonuses, elemental alternatives, unholy affixes and gem transforms
use the existing item model. Effects without an existing numeric stat remain in
tooltip text. Proc chance and level ranges are retained in descriptions; the
existing scalar proc calculation uses the source minimum chance and cast level.
This import does not introduce formulas for unsupported effects.

`game-config.patch.json` adds the missing label for `all_skills_bard`.

Checks: unchanged Common records and stable IDs, socket ranges and prohibition
with forging, player/mercenary legacy build loading, elemental stat mapping,
shared season patch loading, frontend tests, Rust unit tests and parity fixtures.
