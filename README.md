# Trade Browse

Browse and filter a trade bot's stock for [OpenAC](https://github.com/eriknihlen/openac-rust).
Trade a bot, and the plugin reads the trade window: every item offered is
appraised, its facts cached by *property id*, and the window fills in as
answers arrive. Clicking a row asks the bot its price over `/tell`.

A WebAssembly plugin; the client runs it on the session's own thread.

```
cargo test                      # the inventory, price and filter logic, natively
scripts/build.sh --test         # test, build wasm, install into the client
```

In game: `/plugin reload all`, then trade a bot — the window fills on its
own. `/tradebrowse show` opens it, `/tradebrowse scan` re-appraises.

## How it works

```
trade opens ──▶ partner_items ──▶ appraisal queue (one per tick)
                                        │
                     facts by property id ◀── Host::object / int_property
                                        │
                    per-bot CSV (LastSeen, prune on a complete read)
                                        │
                                   the window
row click ──▶ /tell <bot> check <property-id> ──▶ reply ──▶ Price cell
```

- **Property id is the key.** The client's object id dies when the trade
  closes; the signed weenie class ("the long negative number") is stable
  across bots and sessions, so facts and the price query both use it.
- **Appraisal is the only way to know an item.** The server answers one at
  a time, so the scan queues them; a returning bot opens fully populated
  from cache and only new lines show "awaiting appraisal".
- **Each bot's stock is a CSV** in the plugin's data folder, with a
  `LastSeen` column. An item not sighted during a *complete* read is
  pruned — the bot restocked. A partial read never prunes.
- **Prices come from the bot over `/tell`**, keyed by property id. Bot
  dialects (the query word, the reply shape) live in one table in
  `price.rs`; nothing else assumes a particular bot.

## Status: M0 skeleton

The window, the trade watcher, the appraisal queue, the CSV and the price
round-trip are wired; the filter tabs (Category / Slot & Coverage /
Cantrips / Set) and cantrip level bands arrive in M3. See
[PLAN.md](./PLAN.md) for the milestones and the design decisions.

24 unit tests, clippy clean, and the window's markup is parsed in a test
against the same binding list the plugin registers — so a misspelled
element or an undeclared binding fails the build instead of leaving the
window silently absent.

## Layout

| File | Holds |
|---|---|
| `facts.rs` | `ItemFacts`: the flat filterable view of one appraisal |
| `inventory.rs` | the per-bot CSV: parse, upsert, prune, `LastSeen` |
| `price.rs` | the `/tell` query and reply parsing; the bot dialect table |
| `view.rs` | the markup, the binding list, and the rows |
| `lib.rs` | wiring: trade watcher, appraisal queue, panel, commands |

## Notes

- Permissions: `chat` (the `/tell` price query and reports) and `listen`
  (hearing the bot's reply). Reading items and appraising is not gated.
- No bot code and no bot data: this is an MIT browser that talks to any
  bot answering a `/tell check <property-id>`.