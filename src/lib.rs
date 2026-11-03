//! Trade Browse — browse and filter a trade bot's stock.
//!
//! Trade a bot and the plugin reads the trade window: every item offered
//! is queued for appraisal (one per tick, the server's pace), its facts
//! cached by *property id* — the signed weenie class players call the
//! property — and the window fills in as answers arrive. Clicking a row
//! asks the bot its price over `/tell`; the reply lands in the row.
//!
//! Each bot's stock is tracked in its own CSV in the plugin's storage,
//! with a `LastSeen` column; items the bot no longer offers after a
//! complete read are pruned. The cache is the list's source, so a
//! returning bot opens fully populated and only new lines await appraisal.
//!
//! M0 is the skeleton: the window, the commands, the trade watcher, and
//! the pure modules (inventory, price, facts) that carry the design. The
//! appraisal queue and the filter tabs grow in M1 and M3.
//!
//! `/tradebrowse` reports in chat; `/tradebrowse show` opens the window;
//! `/tradebrowse scan` re-appraises the open trade's items.

// `export_plugin!` wires the entry point only for wasm32, so a host build
// (which exists to run the unit tests) has nothing that reaches the
// plugin's own code and reports all of it dead.
#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]

mod facts;
mod inventory;
mod price;
mod view;

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::facts::ItemFacts;
use crate::inventory::{Inventory, Row};
use crate::view::Rows;
use openac_plugin::storage::{StoragePlace, StorageRoot};
use openac_plugin::ui::{ActionValue, Panel, PanelSpec, TypedValue, UiEvent};
use openac_plugin::{Event, Host, Plugin, StartInfo};

/// Ticks between re-reads while idle (a tick is 15 ms).
const IDLE_TICKS: u32 = 100;
/// … and while a scan is running, so the next appraisal starts promptly.
const BUSY_TICKS: u32 = 4;

/// The `ValidLocations` int property (retail `PropertyInt.ValidLocations`).
const VALID_LOCATIONS: u32 = 9;
/// The `ArmorLevel` int property (retail `PropertyInt.ArmorLevel`).
const ARMOR_LEVEL: u32 = 28;

#[derive(Default)]
struct TradeBrowse {
    panel: Option<Panel>,
    /// The bot of the last (or current) trade, by name.
    bot: String,
    /// Its tracked stock, once loaded.
    inventory: Option<Inventory>,
    /// Object ids in the open trade window still awaiting an appraisal
    /// answer. Object ids die with the trade; the answer maps them to
    /// their property id.
    queue: VecDeque<u32>,
    /// The appraisal in flight, if any: the object we asked about.
    asked: Option<u32>,
    /// Facts learned this session, keyed by property id.
    facts: BTreeMap<i64, ItemFacts>,
    /// The property id whose price we last asked, awaiting the reply.
    price_asked: Option<i64>,
    /// The rows currently shown, which the UI's row indexes refer to.
    rows: Rows,
    ticks: u32,
    status: String,
}

impl Plugin for TradeBrowse {
    fn enable(&mut self, host: &mut Host<'_>, _info: &StartInfo) -> Result<(), String> {
        host.register_command(
            "tradebrowse",
            "Browse a trade bot's stock: [show] opens the window, [scan] re-appraises",
        )?;
        let spec = view::BINDINGS.iter().fold(
            PanelSpec::new("main", "Trade Browse", view::MARKUP),
            |spec, (name, kind)| spec.bind(name, kind.abi()),
        );
        self.panel = Some(host.register_panel(&spec)?);
        Ok(())
    }

    fn tick(&mut self, host: &mut Host<'_>) {
        if !host.item_state().is_available {
            return;
        }
        self.ticks = self.ticks.saturating_add(1);
        let cadence = if self.queue.is_empty() {
            IDLE_TICKS
        } else {
            BUSY_TICKS
        };
        if self.ticks < cadence {
            return;
        }
        self.ticks = 0;
        self.begin_scan(host, false);
        self.pump_appraisals(host);
        self.refresh(host);
    }

    fn on_event(&mut self, host: &mut Host<'_>, event: Event) {
        match event {
            Event::Command(call) if call.verb == "tradebrowse" => {
                self.handle_command(host, call.arguments.trim());
            }
            Event::Trade(opened) => {
                // A trade opening or a new item offered is a stock read;
                // a close stops the scan where it stands (an incomplete
                // read prunes nothing).
                match opened {
                    openac_plugin::trade::TradeEvent::Opened(_)
                    | openac_plugin::trade::TradeEvent::ItemAdded(_) => {
                        self.begin_scan(host, false);
                    }
                    openac_plugin::trade::TradeEvent::Closed => {
                        self.queue.clear();
                        self.asked = None;
                        self.save(host);
                    }
                    openac_plugin::trade::TradeEvent::PartnerAccepted(_) => {}
                }
            }
            Event::ChatReceived(line) if line.kind == 3 => {
                self.hear_tell(&line.sender, &line.text);
            }
            Event::Ui(UiEvent::Action(action)) => {
                self.handle_ui(host, &action.handler, action.value);
            }
            _ => {}
        }
    }
}

impl TradeBrowse {
    /// The CSV's storage place for the current bot: one folder per bot, so
    /// `storage_list` can enumerate the tracked ones later.
    fn file(bot: &str) -> StoragePlace {
        StoragePlace::character(StorageRoot::PluginData, &format!("bots/{bot}"))
    }

    /// Load the bot's CSV, if we have one.
    fn load(&mut self, host: &mut Host<'_>, bot: &str) {
        if self.bot == bot && self.inventory.is_some() {
            return;
        }
        let text = host
            .storage_read_text(&Self::file(bot), "stock.csv")
            .ok()
            .flatten()
            .unwrap_or_default();
        self.inventory = Some(Inventory::parse(&text));
        self.bot = bot.to_owned();
    }

    /// Write the bot's CSV back.
    fn save(&self, host: &mut Host<'_>) {
        let Some(inventory) = &self.inventory else {
            return;
        };
        if let Err(why) =
            host.storage_write_text(&Self::file(&self.bot), "stock.csv", &inventory.write())
        {
            host.warn(&format!("could not save the stock file: {why}"));
        }
    }

    /// Notice a trade opening or its items changing: queue whatever is
    /// new. `deep` re-queues even cached items (the concept's Deep scan).
    fn begin_scan(&mut self, host: &mut Host<'_>, deep: bool) {
        let trade = host.trade();
        if !trade.is_open {
            return;
        }
        self.load(host, &trade.partner_name);
        let now = unix_ms(host);
        let offered: Vec<u32> = trade.partner_items.to_vec();
        let mut sighted = BTreeSet::new();
        let mut unseen = 0usize;
        for object in offered.clone() {
            // The object's record names its property id; the appraisal
            // fills the rest. An object the client cannot describe yet is
            // not unseen — it is unread — and says nothing about pruning.
            match host.object(object) {
                Some(object) => {
                    sighted.insert(object.weenie_class_id as i32 as i64);
                }
                None => unseen += 1,
            }
        }
        {
            let Some(inventory) = &mut self.inventory else {
                return;
            };
            for object in offered {
                let Some(object) = host.object(object) else {
                    continue;
                };
                let property = object.weenie_class_id as i32 as i64;
                let cached = self.facts.contains_key(&property);
                if (deep || !cached) && !self.queue.contains(&object.object_id) {
                    self.queue.push_back(object.object_id);
                }
                if cached {
                    inventory.sight(Row::from_facts(&self.facts[&property], now));
                }
            }
            // A read that saw every object may prune what the bot no
            // longer offers; a partial one must not.
            if unseen == 0 {
                inventory.prune_to(&sighted);
            } else {
                inventory.complete = false;
            }
        }
        self.save(host);
        self.status = format!(
            "{}: {} item(s) in the window, {} to appraise{}.",
            trade.partner_name,
            sighted.len(),
            self.queue.len(),
            if unseen == 0 {
                String::new()
            } else {
                format!(", {unseen} not yet described")
            }
        );
    }

    /// Ask for one appraisal per tick, and take in its answer.
    fn pump_appraisals(&mut self, host: &mut Host<'_>) {
        if let Some(asked) = self.asked {
            // The answer arrives as the object's record fills in; poll it.
            let answered = host
                .object(asked)
                .filter(|object| object.has_appraisal_data);
            if let Some(object) = answered {
                let property = object.weenie_class_id as i32 as i64;
                let facts = ItemFacts {
                    property_id: property,
                    name: object.name.clone(),
                    icon_id: object.icon_id,
                    stack_size: u32::try_from(object.stack_size.max(1)).unwrap_or(1),
                    category: facts::Category::from_item_type(object.item_type),
                    item_type: object.item_type,
                    valid_locations: host
                        .int_property(object.object_id, VALID_LOCATIONS)
                        .unwrap_or(0) as u32,
                    coverage_mask: object.coverage_mask,
                    armor_level: host.int_property(object.object_id, ARMOR_LEVEL),
                    spell_ids: object.spell_ids.clone(),
                };
                self.facts.insert(property, facts.clone());
                let now = unix_ms(host);
                if let Some(inventory) = &mut self.inventory {
                    // The sighting carries the icon the row draws: the
                    // property id is known, so the cached row (if any) is
                    // refreshed with the facts the appraisal answered.
                    let mut row = Row::from_facts(&facts, now);
                    if let Some(existing) = inventory.find(property) {
                        row.price = existing.price.clone();
                    }
                    inventory.sight(row);
                }
                self.asked = None;
            }
            return;
        }
        let Some(object) = self.queue.pop_front() else {
            return;
        };
        if host.identify(object).accepted() {
            self.asked = Some(object);
        } else {
            // Busy or gone: try it again later rather than dropping it.
            self.queue.push_back(object);
        }
    }

    /// Match a tell from the trading bot to the price we asked for.
    fn hear_tell(&mut self, sender: &str, text: &str) {
        let Some(asked) = self.price_asked else {
            return;
        };
        if !sender.eq_ignore_ascii_case(&self.bot) {
            return;
        }
        match price::parse_reply(text, asked) {
            price::Reply::Price { property_id, price } => {
                if let Some(inventory) = &mut self.inventory {
                    inventory.set_price(property_id, &price);
                }
                self.price_asked = None;
                self.status = format!("Price: {price}.");
            }
            price::Reply::Unknown { .. } => {
                self.price_asked = None;
                self.status = "The bot says it has no such item.".to_owned();
            }
            price::Reply::Other => {}
        }
    }

    /// Rebuild the rows and push them to the window.
    fn refresh(&mut self, host: &mut Host<'_>) {
        let Some(panel) = self.panel.as_mut() else {
            return;
        };
        let empty = Inventory::default();
        let inventory = self.inventory.as_ref().unwrap_or(&empty);
        let rows = Rows::build(inventory);
        let pending = self.queue.len() + usize::from(self.asked.is_some());
        panel.set(
            "Summary",
            view::summary(&self.bot, inventory.rows.len(), pending),
        );
        panel.set("Icons", TypedValue::UintList(rows.icons.clone()));
        panel.set("Names", TypedValue::TextList(rows.names.clone()));
        panel.set(
            "NameColours",
            TypedValue::UintList(rows.name_colours.clone()),
        );
        panel.set("Types", TypedValue::TextList(rows.types.clone()));
        panel.set("Slots", TypedValue::TextList(rows.slots.clone()));
        panel.set("Coverage", TypedValue::TextList(rows.coverage.clone()));
        panel.set(
            "CreatureCantrips",
            TypedValue::TextList(rows.creature_cantrips.clone()),
        );
        panel.set(
            "ItemCantrips",
            TypedValue::TextList(rows.item_cantrips.clone()),
        );
        panel.set(
            "LifeCantrips",
            TypedValue::TextList(rows.life_cantrips.clone()),
        );
        panel.set("Sets", TypedValue::TextList(rows.sets.clone()));
        panel.set("Prices", TypedValue::TextList(rows.prices.clone()));
        panel.set(
            "PriceColours",
            TypedValue::UintList(rows.price_colours.clone()),
        );
        panel.set("Status", self.status.clone());
        debug_assert!(rows.is_square(), "the list's columns must stay aligned");
        self.rows = rows;
        if let Err(why) = panel.flush(host) {
            self.status = format!("The window could not be updated: {why}");
        }
    }

    fn handle_command(&mut self, host: &mut Host<'_>, argument: &str) {
        match argument {
            "show" => {
                if let Some(panel) = self.panel.as_ref() {
                    panel.show(host, true);
                }
            }
            "scan" => self.begin_scan(host, true),
            _ => {
                let bots = self
                    .inventory
                    .as_ref()
                    .map_or(0, |inventory| inventory.rows.len());
                host.chat_post(&format!(
                    "Trade Browse: {} ({bots} tracked item(s)). \
                     Trade a bot to scan its stock; `/tradebrowse show` opens the window.",
                    if self.bot.is_empty() {
                        "no bot yet".to_owned()
                    } else {
                        self.bot.clone()
                    }
                ));
            }
        }
    }

    fn handle_ui(&mut self, host: &mut Host<'_>, handler: &str, value: ActionValue) {
        let row = match &value {
            ActionValue::Int(index) => usize::try_from(*index).ok(),
            _ => None,
        };
        match handler {
            "Select" => {
                // Clicking a row asks the bot its price, by property id.
                if let Some(id) = row.and_then(|index| self.rows.property_ids.get(index).copied()) {
                    if self.bot.is_empty() {
                        self.status = "No bot to ask.".to_owned();
                    } else {
                        host.chat_submit(&price::query(&price::DEFAULT, &self.bot, id));
                        self.price_asked = Some(id);
                        self.status = format!("Asked {} for the price…", self.bot);
                    }
                }
            }
            "Scan" => self.begin_scan(host, true),
            "Reset" => {
                self.status = "Filters reset.".to_owned();
            }
            _ => {}
        }
        if matches!(handler, "Select" | "Reset") {
            self.refresh(host);
        }
    }
}

/// UTC milliseconds, from the client's clock.
fn unix_ms(host: &mut Host<'_>) -> i64 {
    host.clock().utc_unix_ms
}

openac_plugin::export_plugin!(TradeBrowse);

#[cfg(test)]
mod tests {
    use super::*;

    /// Start the plugin the way the client does, through the SDK's fake
    /// host, which applies the same verb and panel rules the real one
    /// does. A misspelled verb or a bad binding fails here, not in game.
    #[test]
    fn the_plugin_starts_and_registers_its_window() {
        let mut host = openac_plugin::testing::FakeHost::new("liamwestray.trade-browse");
        let mut plugin = TradeBrowse::default();
        host.enable(&mut plugin)
            .expect("the plugin starts: its verb and panel must satisfy the host");
        assert!(plugin.panel.is_some(), "the window was not registered");
    }
}
