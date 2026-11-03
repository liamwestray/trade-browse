# trade-browse — plan

A plugin for [openac-rust](../openac-rust) that turns a SkunkTrader bot's
stock into a browsable, filterable appraisal window: sort and find items by
type, slot, covered area, cantrip spells, item set, ratings, and more — the
concept from the "Trade" screenshot, built as a first-class client plugin
instead of a Decal overlay.

```
SkunkTrader (Decal/SkunkWorks bot, Windows)  ──┐
                                               ├── trade-browse plugin (openac-rust, any OS)
openac-rust client ── appraisal/peer APIs ─────┘
```

## What the concept window must do

From the screenshot, in order:

1. **Header** — bot name ("CyTrader bot"), stock tally ("200 of 200 items,
   195 awaiting appraisal").
2. **Category row** — Weapons / Armor / Clothing / Jewelry / Cloaks /
   Summonons / Aetheria / Salvage / Other.
3. **Enchantability** — Regular / Unenchentable.
4. **Weapon subtypes** — Missile / Heavy / Finesse / Light / Shield.
5. **Slots** — Neck, Bracelet, Ring, Trinket, Cloak, Head, Chest, Abdomen,
   Upper Arms, Lower Arms, Hands, Upper Legs, Lower Legs, Feet.
6. **Stat filters** — Dam / Dam Resist / Crit Dam / Crit Dam Resist /
   Heal Boost.
7. **Item sets** — Soldier, Adept, Archer, Defender, Tinker, Crafter,
   Hearty, Dextrous, Wise, Swift, Hardened, Reinforced, Interlocking,
   Flame Proof, Acid Proof, Cold Proof, Lightning Proof, Dedication,
   No set.
8. **Cantrip school** — Creature / Item / Life cantrips, "Regular only",
   and a **cantrip level band** (Minor → Moderate → Major → Epic →
   Legendary).
9. **Actions** — Reset filters, Deep scan.
10. **Results list** — Item (icon + name), Type, Slots, Creature
    Cantrips, Item Cantrips, Life Cantrips, Item Set, Ratings, Detail;
    stack counts ("x24"); a payment-methods footer ("Points: Trade Note
    (250,000) [1], God Chest [2000], Durable Legendary Key [5]").

## Where the data comes from

Two sources, kept cleanly separated:

### A. The client (authoritative item truth)

The openac-rust plugin SDK already exposes everything the filters need —
no new client work for v1:

| Need | SDK source |
|---|---|
| Item in a trade/corpse/vendor | `Host::loot_contents`, `Host::vendor_item_properties` |
| Appraisal facts | `Host::item_properties(id)` → full `ItemProperties` (ints, floats, strings, data ids, instance ids) |
| Cantrips | `WorldObject::spell_ids` (the spells an appraisal listed on it) + `Host::spell(id)` for name/school/tier |
| Slot / valid locations | `InventoryItem::valid_locations`, `item_type` |
| Covered area | `InventoryItem::coverage_mask` (the body-part coverage bits) |
| Armor type / SL | `ItemProperties::armor` (`ArmorProfile`) |
| Weapon type / dam / crit | `ItemProperties::weapon` (`WeaponProfile`), wield difficulty + crit ints |
| Item set | Name prefix + creation-profile string fallback |
| Aetheria | `item_type` + `active_spell_ids` (surge spells) |
| Icons | `WorldObject::icon_id` (proven in spell-research) |

### B. SkunkTrader (stock list + prices)

SkunkTrader is a SkunkWorks script (`.swx`, C#) that runs under Decal on
Windows. It keeps the sell list and point values. trade-browse needs its
**inventory**: which items the bot has, how many, and at what price.

Options, in order of preference:

1. **File drop (simplest, zero bot changes).** SkunkTrader exports
   `stock.json`; the player points trade-browse at the file (plugin
   `storage_read`, or a path beside the plugin folder). Manual refresh,
   but it unblocks everything else and is the v1 default.
2. **Chat protocol (works with a stock SkunkTrader).** It already answers
   `$check <item>` in open chat and `check`/`points`/`total` by tell. A
   stock dump can be driven tell-by-tell and parsed. Workable for tens of
   items, fragile for hundreds.
3. **SkunkTrader patch (best UX).** Add one command — `browse` — that
   replies with one line per stock item: `wcid|stack|value|name`
   (SkunkWorks exposes the bot's full sell list). ~50 lines of C# in the
   bot's `src/`. The local copy is MIT; keep the upstream copyright
   notice on any fork.

The plugin treats B as *pluggable*: a `StockSource` trait with these
implementations, starting with (1) and growing into (3).

## Architecture

```
trade-browse/
  plugin.json            # permissions: chat, listen (stock protocol)
  src/
    lib.rs               # Plugin wiring: commands, tick, panel events
    stock.rs             # StockSource trait + file/chat/peer readers
    scan.rs              # appraisal queue: ask server, collect facts, rate-limit
    facts.rs             # ItemFacts: the flat, filterable view of one item
    filters.rs           # FilterState: every checkbox/band from the concept
    sets.rs              # item-set recognition (Soldier/Adept/…/Dedication)
    cantrips.rs          # spell_ids → school + level band classification
    ratings.rs           # simple 0–100 score: SL, cantrip levels, coverage
    view.rs              # markup + bindings + row building
  data/
    sets.tsv             # set name → required slots/parts (transcribed)
    cantrip-table.tsv    # generated: spell id → school, tier, name
  PLAN.md
  README.md
```

### The scan pipeline (the hard part)

The screenshot's "195 awaiting appraisal" is the honest state: the client
only learns an item's properties by appraising it, and the server answers
appraisals **one at a time, rate-limited**. Design:

1. `stock.rs` yields `StockItem { weenie_class_id, stack, price, name }`.
2. `scan.rs` keeps a queue; each tick it asks for at most one appraisal
   and records facts as answers arrive (`Event::ItemAppraisalCompleted`
   or polling `item_properties`).
3. Facts are **cached by weenie class id** via `storage_write` — a God
   Chest greatsword appraised once is never re-appraised; "Deep scan"
   re-validates only entries older than N days.
4. Until an item is appraised it still shows in the list, greyed, with
   whatever the name implies (type, set) — matching the concept's tally.

### Filters (`filters.rs`)

One bitset per filter family, OR within a family, AND across families —
standard auction-house semantics:

- `category` — from `item_type` bits: MeleeWeapon / MissileWeapon /
  Clothing / Jewelry / Gem (summonons) / Aetheria / Salvage / Other
- `enchantable` — ItemMana > 0 and no "unenchentable" marker
- `weapon_subtype` — wield difficulty + weapon skill →
  Missile / Heavy / Finesse / Light; shield type → Shield
- `slot` — `valid_locations` bits → the 14 concept slots
- `coverage` — `coverage_mask` bits → Head / Chest / Abdomen / UpperArms /
  LowerArms / Hands / UpperLegs / LowerLegs / Feet
- `stat` — Dam / Dam Resist / Crit Dam / Crit Dam Resist / Heal Boost
  (v2: needs the ratings pass over armor/crit ints)
- `set` — name-prefix table in `data/sets.tsv`
- `cantrip_school` + `cantrip_band` — `spell_ids` → school (31 creature /
  32 item / 33 life) and tier via the spell table; bands: Minor 1–3,
  Moderate 4, Major 5, Epic 6, Legendary 7–8

### Window

Same technique as spell-research: declarative markup, typed bindings, a
`<list>` with icon columns (real item icons via `iconkind="did"`, proven
this week), text columns with per-cell colors. The concept's wall of
checkboxes becomes **tabs** (Category / Slot & Coverage / Cantrips / Set)
so the window stays usable at 700×480, with the active filters summarised
in one line ("Armor · Chest+Legs · Legendary item cantrips").

## Milestones

- **M0 — skeleton (half a day).** Repo, plugin.json (`chat`, `listen`),
  empty window opens with `/tradebrowse show`, `/tradebrowse` prints
  help. Cargo layout above, MIT LICENSE.
- **M1 — facts + file stock source (1–2 days).** `StockSource::file`
  reading `stock.json`; appraisal queue; `ItemFacts` cached by wcid
  across sessions; list shows icon / name / type / slots / cantrips / set
  with "awaiting appraisal" greying. Useful with zero SkunkTrader
  changes.
- **M2 — filters + sorting (1–2 days).** `FilterState`, the four filter
  tabs, cantrip bands, set recognition, sortable columns, Reset filters.
- **M3 — SkunkTrader protocol (1–2 days + the .swx patch).** `browse`
  command in an MIT-preserving SkunkTrader fork, `StockSource::chat`, bot
  name + points footer in the header.
- **M4 — ratings + polish (ongoing).** `ratings.rs` score, stat filters,
  Deep scan, column layout matching the concept.

## Open questions (answer before M1 ends)

1. **Which server?** The target emulator's dat tables decide spell ids for
   cantrip bands. `data/cantrip-table.tsv` must be generated from the
   *server's* spell table, like spell-research's dump workflow.
2. **SkunkTrader fork vs. file drop?** M3 assumes we may patch the bot;
   if upstream won't take it, the fork lives beside this repo.
3. **Peer transport later?** openac-rust already has a plugin peer API
   (`peer_subscribe`, `broadcast_command` with capability tags). A future
   M5 could run a tiny companion plugin on the *bot side* and push stock
   over peers — no Decal, no chat scraping. The `StockSource` trait keeps
   a fourth variant open for it.

## Licensing guardrails

- Everything here is MIT (Liam Westray), matching the other AC repos.
- **Do not commit game data**: dat extracts, full spell dumps beyond the
  small generated cantrip table, live-play screenshots, or Turbine
  content. Transcribed community wiki facts (sets, bands) are fine with
  source URLs in comments, as spell-research does.
- SkunkTrader is MIT (upstream: gitlab.com/Cyprias/SkunkTrader); any
  patch we ship keeps its copyright notice intact.
