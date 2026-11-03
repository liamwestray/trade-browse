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
mod filters;
mod inventory;
mod price;
mod view;

use std::collections::{BTreeMap, BTreeSet};

use crate::facts::ItemFacts;
use crate::inventory::{Inventory, Row};
use crate::view::{
    BAND_BINDINGS, CANTRIP_BINDINGS, CATEGORY_BINDINGS, Rows, SET_BINDINGS, SLOT_BINDINGS,
    WEAPON_BINDINGS,
};
use openac_plugin::storage::{StoragePlace, StorageRoot};
use openac_plugin::ui::{ActionValue, Panel, PanelSpec, TypedValue, UiEvent};
use openac_plugin::{Event, Host, Plugin, StartInfo};

/// Ticks between re-reads while idle (a tick is 15 ms).
const IDLE_TICKS: u32 = 100;
/// … and while a scan is running, so the next appraisal starts promptly.
const BUSY_TICKS: u32 = 4;

/// One object in the trade window, remembered with the property id its
/// record carried when it was sighted. The bots price by the *object
/// guid* — the number their own client sees — so the row keeps it; the
/// signed weenie class stays the key of the facts cache and the CSV.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ScanItem {
    /// The client's object id, unsigned: what `Host::object` answers.
    object_id: u32,
    /// The signed weenie class: the facts cache and CSV key.
    property_id: i64,
}

/// The `ValidLocations` int property (retail `PropertyInt.ValidLocations`).
const VALID_LOCATIONS: u32 = 9;
/// The `ArmorLevel` int property (retail `PropertyInt.ArmorLevel`).
const ARMOR_LEVEL: u32 = 28;
/// The `WieldDifficulty` int property (retail `PropertyInt.WieldDifficulty`).
const WIELD_DIFFICULTY: u32 = 160;
/// The `CritRating` int property (retail `PropertyInt.CritRating`).
const CRIT_RATING: u32 = 313;

#[derive(Default)]
struct TradeBrowse {
    panel: Option<Panel>,
    /// The bot of the last (or current) trade, by name.
    bot: String,
    /// Its tracked stock, once loaded.
    inventory: Option<Inventory>,
    /// The open trade's objects still awaiting an appraisal answer.
    /// Object ids die with the trade; the answer maps them to their
    /// property id.
    queue: Vec<ScanItem>,
    /// The appraisal in flight, if any.
    asked: Option<ScanItem>,
    /// Facts learned this session, keyed by property id.
    facts: BTreeMap<i64, ItemFacts>,
    /// The (object guid we asked about, property id to write the answer
    /// to) of the outstanding price query. The guid is signed, as the
    /// bots spell it.
    price_asked: Option<(i64, i64)>,
    /// The rows currently shown, which the UI's row indexes refer to.
    rows: Rows,
    ticks: u32,
    /// The raw-appraisal dump gathered during `/tradebrowse dump`: one
    /// CSV line per item, every number the client holds for it. Written
    /// to `dump/appraisals.csv` in plugin data when the scan drains.
    dump: Vec<String>,
    /// Whether the next scans should record raw properties.
    dumping: bool,
    /// The filter rows, as the concept draws them.
    filters: filters::FilterState,
    /// The list's sort: column index (1 = the Item name; 0 is the icon
    /// column, which never sorts) and direction.
    sort_column: i32,
    sort_descending: bool,
    /// The list's column widths, as the player dragged them (empty: the
    /// markup's authored widths).
    widths: Vec<f32>,
    status: String,
}

impl Plugin for TradeBrowse {
    fn enable(&mut self, host: &mut Host<'_>, _info: &StartInfo) -> Result<(), String> {
        host.register_command(
            "tradebrowse",
            "Browse a trade bot's stock: [show] opens the window, [scan] re-appraises",
        )?;
        let spec = view::BINDINGS.iter().fold(
            // The dock slot reads "TB" rather than the derived "TR".
            PanelSpec::new("main", "Trade Browse", view::MARKUP).icon_text("TB"),
            |spec, (name, kind)| spec.bind_typed(name, kind.abi()),
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
                        self.price_asked = None;
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
        let offered: BTreeSet<u32> = trade.partner_items.iter().copied().collect();

        let mut sighted: BTreeSet<i64> = BTreeSet::new();
        let mut unread = 0usize;
        // Pass one: read every offered object and queue what needs an
        // appraisal. Facts are computed here (they need &self and host);
        // the inventory is only touched in pass two, so the two borrows
        // never overlap.
        let mut sightings: Vec<(i64, Row)> = Vec::new();
        for object_id in offered {
            let Some(object) = host.object(object_id) else {
                // Not described yet: the server's create packet is still
                // on its way. It is neither sighted nor unseen.
                unread += 1;
                continue;
            };
            let property = facts::property_id(object.weenie_class_id);
            sighted.insert(property);
            let item = ScanItem {
                object_id,
                property_id: property,
            };
            let cached = self.facts.contains_key(&property);
            if (deep || !cached) && !self.queue.contains(&item) {
                self.queue.push(item);
            }
            // The live guid is what a price click needs, and the object
            // record carries it whether or not the appraisal has
            // answered. A row left holding a previous trade's guid (or 0)
            // would silently refuse to ask.
            let guid = facts::property_id(object_id);
            if cached {
                let mut fresh = Row::from_facts(&self.facts[&property], now);
                fresh.object_id = guid;
                sightings.push((property, fresh));
            } else {
                // Name, icon, slots and covered areas arrive with the
                // object record itself, long before the appraisal
                // answers. Cache them as partial facts so the filters can
                // read this row now.
                if let Some(partial) = self.facts_from_object(host, object_id) {
                    self.facts.insert(property, partial);
                }
                sightings.push((property, Row::from_sighting(&object, now)));
            }
        }
        if let Some(inventory) = &mut self.inventory {
            for (property, mut row) in sightings {
                if let Some(existing) = inventory.find(property) {
                    row.price = existing.price.clone();
                }
                inventory.sight(row);
            }
            // A read that saw every object may prune what the bot no
            // longer offers; a partial one must not.
            if unread == 0 {
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
            if unread == 0 {
                String::new()
            } else {
                format!(", {unread} not yet described")
            }
        );
    }

    /// The facts an object record already carries, before any appraisal:
    /// name, icon, stack, category, slots and covered areas. The slot and
    /// weapon filters read these, so a row filters correctly the moment
    /// it appears rather than only after its appraisal lands.
    fn facts_from_object(&self, host: &mut Host<'_>, object_id: u32) -> Option<ItemFacts> {
        let object = host.object(object_id)?;
        let valid_locations = host.int_property(object_id, VALID_LOCATIONS).unwrap_or(0) as u32;
        let wield = host.int_property(object_id, WIELD_DIFFICULTY);
        Some(ItemFacts {
            property_id: facts::property_id(object.weenie_class_id),
            name: object.name.clone(),
            icon_id: object.icon_id,
            stack_size: u32::try_from(object.stack_size.max(1)).unwrap_or(1),
            category: facts::category_of(object.item_type, valid_locations),
            item_type: object.item_type,
            valid_locations,
            coverage_mask: object.coverage_mask,
            slots: facts::slot_names(valid_locations),
            covered: facts::coverage_names(object.coverage_mask),
            weapon: facts::weapon_of(valid_locations, wield),
            item_set: facts::set_of(&object.name).to_owned(),
            ..Default::default()
        })
    }

    /// Ask for one appraisal per tick, and take in its answer.
    fn pump_appraisals(&mut self, host: &mut Host<'_>) {
        if let Some(asked) = self.asked {
            // The answer arrives as the object's record fills in; poll it.
            let answered = host
                .object(asked.object_id)
                .filter(|object| object.has_appraisal_data);
            if let Some(object) = answered {
                let property = asked.property_id;
                let mut facts = self
                    .facts_from_object(host, object.object_id)
                    .unwrap_or_default();
                facts.property_id = property;
                facts.armor_level = host.int_property(object.object_id, ARMOR_LEVEL);
                facts.crit_rating = host.int_property(object.object_id, CRIT_RATING);
                facts.spell_ids = object.spell_ids.clone();
                self.classify_cantrips(&mut facts, host);
                facts.ratings = facts::ratings_of(&facts);
                facts.detail = detail_of(&facts);
                self.facts.insert(property, facts.clone());
                if self.dumping {
                    let line = self.dump_line(host, object.object_id, &facts);
                    self.dump.push(line);
                }
                let now = unix_ms(host);
                if let Some(inventory) = &mut self.inventory {
                    let mut row = Row::from_facts(&facts, now);
                    row.object_id = facts::property_id(asked.object_id);
                    if let Some(existing) = inventory.find(property) {
                        row.price = existing.price.clone();
                    }
                    inventory.sight(row);
                    self.save(host);
                }
                self.asked = None;
            }
            return;
        }
        if self.dumping && self.queue.is_empty() {
            self.finish_dump(host);
        }
        let Some(item) = (!self.queue.is_empty()).then(|| self.queue.remove(0)) else {
            return;
        };
        // The item left the window while its appraisal waited; nothing
        // to appraise.
        let Some(object) = host.object(item.object_id) else {
            return;
        };
        let item = ScanItem {
            object_id: object.object_id,
            property_id: facts::property_id(object.weenie_class_id),
        };
        if host.identify(object.object_id).accepted() {
            self.asked = Some(item);
        } else {
            // Busy: try it again later rather than dropping it.
            if !self.queue.contains(&item) {
                self.queue.push(item);
            }
        }
    }

    /// Sort an item's cantrips into the concept's three school columns,
    /// each named with its level band: "Recklessness (Legendary)".
    fn classify_cantrips(&mut self, facts: &mut ItemFacts, host: &mut Host<'_>) {
        for id in facts.spell_ids.clone() {
            let Some(spell) = host.spell(id) else {
                continue;
            };
            let cell = format!(
                "{} ({})",
                spell.name,
                facts::Band::of_tier(spell.tier).label()
            );
            match spell.school {
                31 => facts.creature_cantrips.push(cell),
                32 => facts.item_cantrips.push(cell),
                33 => facts.life_cantrips.push(cell),
                // War, void and anything else are not cantrip schools a
                // buyer filters by; the Detail line still lists them.
                _ => {}
            }
        }
    }

    /// Match a tell from the trading bot to the price we asked for.
    fn hear_tell(&mut self, sender: &str, text: &str) {
        let Some((guid, property)) = self.price_asked else {
            return;
        };
        if !sender.eq_ignore_ascii_case(&self.bot) {
            return;
        }
        match price::parse_reply(text, guid) {
            price::Reply::Price { price, .. } => {
                if let Some(inventory) = &mut self.inventory {
                    inventory.set_price(property, &price);
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
        // The filters scope the list. Rows whose facts are cached match
        // against them; a row with no facts yet passes only an empty
        // filter set, so a fresh scan is never hidden behind a box.
        let shown = if self.filters == filters::FilterState::default() {
            inventory.clone()
        } else {
            Inventory {
                rows: inventory
                    .rows
                    .iter()
                    .filter(|row| match self.facts.get(&row.property_id) {
                        Some(facts) => self.filters.matches(facts),
                        // A row the client has never described cannot be
                        // judged; hiding it behind a filter would make it
                        // vanish forever, so it shows until facts arrive.
                        None => true,
                    })
                    .cloned()
                    .collect(),
                complete: inventory.complete,
            }
        };
        let mut rows = Rows::build(&shown);
        rows.sort_by(self.sort_column, self.sort_descending);
        let pending = self.queue.len() + usize::from(self.asked.is_some());
        panel.set(
            "Summary",
            view::summary(&self.bot, inventory.rows.len(), pending),
        );
        view::set_filters(panel, &self.filters);
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
        panel.set("Ratings", TypedValue::TextList(rows.ratings.clone()));
        panel.set("Details", TypedValue::TextList(rows.details.clone()));
        panel.set("Prices", TypedValue::TextList(rows.prices.clone()));
        panel.set(
            "PriceColours",
            TypedValue::UintList(rows.price_colours.clone()),
        );
        panel.set("Status", self.status.clone());
        panel.set("SortColumn", TypedValue::Int(self.sort_column));
        panel.set("SortDescending", TypedValue::Bool(self.sort_descending));
        panel.set(
            "Widths",
            TypedValue::FloatList(if self.widths.is_empty() {
                view::AUTHORED_WIDTHS.to_vec()
            } else {
                self.widths.clone()
            }),
        );
        debug_assert!(rows.is_square(), "the list's columns must stay aligned");
        self.rows = rows;
        if let Err(why) = panel.flush(host) {
            self.status = format!("The window could not be updated: {why}");
        }
    }

    /// One raw-appraisal line: the ids we key by, then every number the
    /// client holds for the object, keyed by its property id.
    fn dump_line(&mut self, host: &mut Host<'_>, object_id: u32, facts: &ItemFacts) -> String {
        let mut fields = vec![
            format!("guid {}", facts::property_id(object_id)),
            format!("property {}", facts.property_id),
            format!("name {}", facts.name),
            format!("icon {}", facts.icon_id),
            format!("stack {}", facts.stack_size),
            format!("item_type {:#X}", facts.item_type),
            format!("valid_locations {:#X}", facts.valid_locations),
            format!("coverage {:#X}", facts.coverage_mask),
            format!(
                "spells {}",
                facts
                    .spell_ids
                    .iter()
                    .map(|id| id.to_string())
                    .collect::<Vec<_>>()
                    .join(" ")
            ),
        ];
        if let Some(properties) = host.properties(object_id) {
            for (id, value) in &properties.ints {
                fields.push(format!("int{id} {value}"));
            }
            for (id, value) in &properties.floats {
                fields.push(format!("float{id} {value}"));
            }
            for (id, value) in &properties.bools {
                fields.push(format!("bool{id} {}", u8::from(*value)));
            }
            for (id, value) in &properties.data_ids {
                fields.push(format!("data{id} {value}"));
            }
            for (id, value) in &properties.instance_ids {
                fields.push(format!("instance{id} {value}"));
            }
            if let Some(weapon) = &properties.weapon {
                fields.push(format!(
                    "weapon {} {} {} {} {} {} {} {}",
                    weapon.damage_type,
                    weapon.weapon_time,
                    weapon.weapon_skill,
                    weapon.damage,
                    weapon.damage_variance,
                    weapon.damage_mod,
                    weapon.max_velocity,
                    weapon.weapon_offense,
                ));
            }
            if let Some(armor) = &properties.armor {
                fields.push(format!(
                    "armor {} {} {} {} {} {} {} {}",
                    armor.armor_level,
                    armor.slash_mod,
                    armor.pierce_mod,
                    armor.bludgeon_mod,
                    armor.cold_mod,
                    armor.fire_mod,
                    armor.acid_mod,
                    armor.nether_mod,
                ));
            }
        }
        fields.join(",")
    }

    /// Write the recorded dump and stop recording.
    fn finish_dump(&mut self, host: &mut Host<'_>) {
        self.dumping = false;
        let place = StoragePlace::global(StorageRoot::PluginData);
        let text = format!(
            "recorded {} item(s)\n{}",
            self.dump.len(),
            self.dump.join("\n")
        );
        match host.storage_write_text(&place, "dump/appraisals.csv", &text) {
            Ok(()) => {
                self.status = format!(
                    "Dumped {} appraisal(s) to the plugin's data folder.",
                    self.dump.len()
                );
                host.chat_post(&format!(
                    "Trade Browse: recorded {} appraisal(s).",
                    self.dump.len()
                ));
            }
            Err(why) => {
                self.status = format!("The dump could not be written: {why}");
            }
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
            "dump" => {
                self.dump.clear();
                self.dumping = true;
                self.begin_scan(host, true);
                host.chat_post(
                    "Trade Browse: recording every appraisal. The file lands in the plugin's data folder when the scan drains.",
                );
            }
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
                // Clicking a row asks the bot its price, by the item's
                // *object guid* — the number the bot's own client sees.
                // The open trade window is the live source; the row's
                // cached guid (from the CSV) is the fallback for a bot
                // traded earlier. The query always goes out — a stale id
                // costs one "could not find" reply, which is more honest
                // than a silent no-op.
                let Some(index) = row else { return };
                let Some(property) = self.rows.property_ids.get(index).copied() else {
                    return;
                };
                if self.bot.is_empty() {
                    self.status = "No bot to ask.".to_owned();
                } else {
                    // The row's guid is stamped from the live window on
                    // every sighting, so it is the freshest number the
                    // plugin holds for this item.
                    let guid = self
                        .inventory
                        .as_ref()
                        .and_then(|inventory| inventory.find(property))
                        .map(|row| row.object_id)
                        .unwrap_or(0);
                    if guid == 0 {
                        self.status = "Trade the bot again to ask its price.".to_owned();
                    } else {
                        host.chat_submit(&price::query(&price::DEFAULT, &self.bot, guid));
                        self.price_asked = Some((guid, property));
                        self.status = format!("Asked {} for {guid}…", self.bot);
                    }
                }
            }
            "Sort" => {
                // The header click arrives as Action<int, bool>: the
                // column and its new direction.
                if let Some((column, descending)) = int_bool(&value) {
                    self.sort_column = column.max(1);
                    self.sort_descending = descending;
                    self.refresh(host);
                }
            }
            "ColumnResize" => {
                // The wire packs a float list as comma-joined text.
                if let ActionValue::Text(joined) = &value {
                    self.widths = joined
                        .split(',')
                        .filter_map(|word| word.trim().parse().ok())
                        .collect();
                }
            }
            "Scan" => self.begin_scan(host, true),
            "Reset" => {
                self.filters = filters::FilterState::default();
                self.status = "Filters reset.".to_owned();
            }
            // Every filter control names its own handler, so a click says
            // exactly which box it toggled. A checkbox arrives as
            // ActionValue::Int(1|0): the new state, which is what we set.
            "UnenchentableAction" => {
                self.filters.unenchentable_only = bool_value(&value);
            }
            handler if handler.ends_with("Action") => {
                let name = handler.trim_end_matches("Action");
                if let Some(index) = CATEGORY_BINDINGS.iter().position(|b| *b == name) {
                    let category = view::CATEGORIES[index];
                    if self.filters.categories.has(category) != bool_value(&value) {
                        self.filters.categories = self.filters.categories.toggled(category);
                    }
                } else if let Some(index) = WEAPON_BINDINGS.iter().position(|b| *b == name) {
                    if self.filters.weapons.has(index) != bool_value(&value) {
                        self.filters.weapons = self.filters.weapons.toggled(index);
                    }
                } else if let Some(index) = SLOT_BINDINGS.iter().position(|b| *b == name) {
                    if self.filters.slots.has(index) != bool_value(&value) {
                        self.filters.slots = self.filters.slots.toggled(index);
                    }
                } else if let Some(index) = CANTRIP_BINDINGS.iter().position(|b| *b == name) {
                    let c = &mut self.filters.cantrips;
                    match index {
                        0 => c.creature = bool_value(&value),
                        1 => c.item = bool_value(&value),
                        2 => c.life = bool_value(&value),
                        _ => c.regular_only = bool_value(&value),
                    }
                } else if let Some(index) = BAND_BINDINGS.iter().position(|b| *b == name) {
                    // A band button carries no state: clicking the checked
                    // band clears it, any other selects it.
                    use facts::Band;
                    let chosen = [
                        Band::Minor,
                        Band::Moderate,
                        Band::Major,
                        Band::Epic,
                        Band::Legendary,
                    ][index];
                    self.filters.bands.checked = if self.filters.bands.checked == Some(chosen) {
                        None
                    } else {
                        Some(chosen)
                    };
                } else if let Some(index) = SET_BINDINGS.iter().position(|b| *b == name) {
                    let sets = &mut self.filters.sets;
                    match index {
                        18 => sets.none = !sets.none,
                        19 => sets.other = !sets.other,
                        _ => {
                            let label = view::SET_LABELS[index];
                            match sets.checked.iter().position(|s| s == label) {
                                Some(at) => {
                                    sets.checked.remove(at);
                                }
                                None => sets.checked.push(label.to_owned()),
                            }
                        }
                    }
                }
            }
            _ => {}
        }
        // Every filter control names its own handler (`CatWeaponsAction`,
        // `BandMinorAction`, …); a toggle must redraw the list. The old
        // shared `Toggle*` names are gone — matching on them here left
        // every checkbox updating state without ever refreshing.
        if handler.ends_with("Action") || matches!(handler, "Select" | "Reset") {
            self.refresh(host);
        }
    }
}

/// An `Action<int, bool>` argument: the wire packs it as text, a flag
/// byte then the number (OpenAC `ActionValue::from_int_bool`).
fn int_bool(value: &ActionValue) -> Option<(i32, bool)> {
    let ActionValue::Text(packed) = value else {
        return None;
    };
    let (flag, number) = packed.split_at(1);
    Some((number.parse().ok()?, flag.as_bytes().first()? != &0))
}

/// A checkbox's new state, as the wire carries it (`Int(1)` / `Int(0)`).
fn bool_value(value: &ActionValue) -> bool {
    matches!(value, ActionValue::Int(1))
}

/// UTC milliseconds, from the client's clock.
fn unix_ms(host: &mut Host<'_>) -> i64 {
    host.clock().utc_unix_ms
}

/// The Detail column: armour shows its level, a stack its count.
fn detail_of(facts: &ItemFacts) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(level) = facts.armor_level.filter(|l| *l > 0) {
        parts.push(format!("AL {level}"));
    }
    if facts.stack_size > 1 {
        parts.push(format!("x{}", facts.stack_size));
    }
    parts.join(" ")
}

openac_plugin::export_plugin!(TradeBrowse);

#[cfg(test)]
mod tests {
    use super::*;

    /// Start the plugin the way the client does, through the SDK's fake
    /// host, which applies the same verb and panel rules the real one
    /// does. A misspelled verb or a bad binding fails here, not in game.
    /// A checkbox click must both update the filter state and redraw the
    /// window. The per-control handler names (`CatWeaponsAction`, …) once
    /// left the redraw gate matching the old `Toggle*` prefix, so every
    /// box silently did nothing.
    #[test]
    fn a_checkbox_click_updates_the_filter_and_asks_for_a_redraw() {
        // The state half: the click carries the new value, not a flip.
        let value = ActionValue::Int(1);
        assert!(bool_value(&value));
        let mut filters = filters::FilterState::default();
        filters.categories = filters.categories.toggled(facts::Category::Armor);
        assert!(filters.categories.has(facts::Category::Armor));
        // The redraw half: every per-control handler name ends in
        // "Action", which is what the gate now matches.
        for name in [
            "CatWeaponsAction",
            "WpnMissileAction",
            "SlotNecklaceAction",
            "CanCreatureAction",
            "UnenchentableAction",
        ] {
            assert!(name.ends_with("Action"), "{name} would not redraw the list");
        }
    }

    #[test]
    fn the_plugin_starts_and_registers_its_window() {
        let mut host = openac_plugin::testing::FakeHost::new("liamwestray.trade-browse");
        let mut plugin = TradeBrowse::default();
        host.enable(&mut plugin)
            .expect("the plugin starts: its verb and panel must satisfy the host");
        assert!(plugin.panel.is_some(), "the window was not registered");
    }
}
