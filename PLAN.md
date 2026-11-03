# trade-browse — plan

A plugin for [OpenAC](https://github.com/eriknihlen/openac-rust) that
browses a trade bot's stock: appraise every item, then sort and filter by
armor type, cantrip spells, item set, slot, covered area, and more — the
concept from the "Trade" screenshot.

It works with **any** trade bot that answers a `/tell` price query, not
SkunkTrader specifically. We use no bot data and ship no bot code, so this
is simply an MIT-licensed browser.

```
player ──/tradebrowse──▶ trade-browse plugin
                            │  /tell <bot> check <property-id>   (price)
                            │  appraisal                         (facts)
                            ▼
                    trade bot  +  OpenAC server
```

## The two kinds of information, and where each comes from

### 1. Item facts — from appraising (the client's job)

The only way to learn an item's properties is to **appraise it**. The SDK
answers a completed appraisal with everything the filters need:

| Need | Source |
|---|---|
| Property id (the "long negative number") | `WorldObject::weenie_class_id`, shown signed |
| Cantrip spells | `WorldObject::spell_ids` → `Host::spell(id)` for name, school, tier |
| Slot | `valid_locations` bits |
| Covered area | `coverage_mask` bits (head/chest/arms/legs/hands/feet) |
| Armor type / level | `ItemProperties::armor` |
| Weapon class / dam / crit | `ItemProperties::weapon` + wield difficulty |
| Item set | name prefix (Soldier, Adept, Dextrous, …) |
| Category | `item_type` bits → Weapons / Armor / Clothing / Jewelry / Cloaks / Summonons / Aetheria / Salvage / Other |
| Icon | `WorldObject::icon_id` |

Appraisals are answered **one at a time and rate-limited**, which is why
the concept's header reads "195 awaiting appraisal". The plugin keeps a
queue, asks for one per tick, and caches facts **by property id** — an
Acid Greataxe appraised once is never appraised again, at any bot.

### 2. Price — from the bot, over `/tell`

Clicking a row asks the bot what that item costs, by its **property id**:

```
/tell CyTrader check -20601
```

The bot's reply is parsed for the price. This is the only bot-specific
piece, and it is deliberately thin:

- **Send** with `Host::chat_submit("/tell <bot> check <id>")`.
- **Receive** through `Event::Chat(ChatLine)` where `kind == 3` (a tell),
  matching `sender` against the bot's name and reading `text`.
- The exact query word (`check`, `$check`, `price`) and reply shape vary
  per bot, so both live in one small table — `protocol.rs` — not scattered
  through the code. Adding a bot dialect is one row.

No bot inventory is scraped, no bot files are read, and no bot code is
bundled.

## Per-bot inventory tracking (CSV)

Every time the plugin sees a bot's stock, it upserts a CSV in plugin
storage (`storage_write`, one file per bot, e.g. `CyTrader.csv`):

| Column | Meaning |
|---|---|
| `PropertyId` | the signed weenie class id — the row's key |
| `Name` | item name |
| `Category` | Weapons / Armor / … |
| `Slot` | valid-location name(s) |
| `Coverage` | covered body parts |
| `ArmorType` | Plate / Chain / … |
| `ArmorLevel` | SL |
| `CreatureCantrips` | names, level-banded |
| `ItemCantrips` | names |
| `LifeCantrips` | names |
| `ItemSet` | Soldier / Adept / … / `No set` |
| `Stack` | stack size |
| `Price` | last price the bot quoted, blank if never asked |
| `LastSeen` | unix ms of the most recent sighting |

Rules:

- **Sighting** = the item appears in the bot's stock this visit → refresh
  its `LastSeen` (and any newly-appraised facts).
- **Pruning** = an item in the CSV that is *not* sighted during a complete
  stock read is **removed**. A partial read (a scan that stopped early)
  must not prune; the file records whether the last read was complete.
- The CSV is the source of the list, so a bot's stock opens instantly from
  cache and appraisals only fill gaps.

## Architecture

Follows the spell-research patterns exactly: a pure core unit-tested
natively, a thin `lib.rs` against the host, declarative markup with a
binding list that is parsed in a test so a typo fails the build.

```
trade-browse/
  plugin.json            # permissions: chat, listen
  src/
    lib.rs               # wiring: commands, tick, panel events, /tell replies
    view.rs              # markup, BINDINGS, row building (spell-research shape)
    facts.rs             # ItemFacts: the flat filterable view of one item
    appraisal.rs         # the one-at-a-time queue; facts by property id
    inventory.rs         # the CSV: parse, upsert, prune, complete-read flag
    price.rs             # /tell price requests, in flight, matched to replies
    protocol.rs          # bot dialects: query word + reply patterns
    filters.rs           # FilterState: every checkbox and band in the concept
    sets.rs              # item-set recognition by name
    cantrips.rs          # spell_ids → school + level band
  data/
    sets.tsv             # set name → slots/parts (transcribed, sourced)
  PLAN.md
  README.md
```

`inventory.rs`, `protocol.rs`, `filters.rs`, `sets.rs`, `cantrips.rs` and
`price.rs` are pure — no host calls — so the CSV round-trip, pruning,
reply parsing and filter logic are all tested without a running client.

### Commands

- `/tradebrowse` — summary of tracked bots and their stock counts.
- `/tradebrowse show` — open the window.
- `/tradebrowse <bot>` — browse that bot's stock.
- `/tradebrowse scan` — re-read the current bot's whole stock (the
  concept's **Deep scan**).

### Window

Same technique as spell-research: `<list>` with an icon column
(`iconkind="did"`, real item icons) and text columns with per-cell colours;
a tab row for the filter families so the concept's wall of checkboxes
stays usable at 700×480.

- **Tabs:** Category · Slot & Coverage · Cantrips · Set
- **Cantrip level band:** Minor (1–3) · Moderate (4) · Major (5) ·
  Epic (6) · Legendary (7–8)
- **Columns:** Item, Type, Slots, Creature Cantrips, Item Cantrips, Life
  Cantrips, Item Set, Price, Detail
- **Header:** bot name, `N of M items, K awaiting appraisal`
- **Row click** selects (status line shows the full recipe of filters) and
  fires the `/tell` price check for that property id; the Price cell fills
  in when the bot answers.

## Milestones

- **M0 — skeleton.** `plugin.json`, Cargo layout, empty window opens,
  `/tradebrowse` prints help. MIT LICENSE.
- **M1 — inventory + appraisal.** CSV read/write/upsert/prune with
  `LastSeen`; appraisal queue; `ItemFacts` cached by property id across
  sessions; list draws icon/name/type/slots/cantrips/set, unappraised rows
  greyed.
- **M2 — price over /tell.** `protocol.rs` dialect table; click → `/tell
  check <id>`; reply matched to the row and written back to the CSV.
- **M3 — filters + sorting.** `FilterState`, the four tabs, cantrip bands,
  set recognition, sortable columns, Reset filters.
- **M4 — polish.** Stat filters (Dam / Dam Resist / Crit / Heal Boost),
  ratings, Deep scan, payment-methods footer.

## Open questions

1. **Bot discovery.** Does the player name the bot (`/tradebrowse
   CyTrader`), or should the plugin notice any bot that answers a
   `points`/`check` style tell? v1: named by the player.
2. **Stock read.** How does the plugin learn a bot's *whole* stock without
   a bot-side change — does the bot list it on request, or does the player
   open the trade window and the plugin reads `TradeState::partner_items`?
   The trade-window route needs no bot cooperation at all and is the
   likely v1 answer; the `/tell` route is the fallback.
3. **Which OpenAC server build** for the spell table that defines cantrip
   tiers — generated from the target server's own table, as
   spell-research does.

## Licensing

MIT, © Liam Westray. No bot code, no bot data, no game content. Transcribed
community facts (item sets, cantrip bands) carry source URLs in comments.