//! The per-bot inventory: one CSV per bot, in the plugin's own storage.
//!
//! The file is both cache and source of the list: a bot's stock opens
//! instantly from the last visit, and each new sighting refreshes it.
//! Rows carry a `LastSeen` stamp; a row the bot no longer offers after a
//! *complete* read is pruned, because the bot restocked. An incomplete
//! read (a trade closed mid-scan) must never prune — it did not see the
//! whole window, so absence proves nothing. That rule is the whole point
//! of `complete` and is tested as its own case.
//!
//! Pure over strings: `parse` and `write` are the file's grammar, so the
//! round-trip, the pruning and the quoting of names with commas are all
//! testable without a host.

use crate::facts::{Category, ItemFacts};

/// The columns, in file order. `PropertyId` is the row's key.
pub const HEADER: &str = "PropertyId,Name,Category,Slots,Coverage,\
                          ArmorLevel,CreatureCantrips,ItemCantrips,LifeCantrips,\
                          ItemSet,Stack,Price,LastSeen,IconId,Ratings,Detail,ObjectId";

/// One tracked item of one bot.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Row {
    /// The property id (signed weenie class): the key.
    pub property_id: i64,
    pub name: String,
    pub category: Category,
    /// The wear slots by name, joined with `; ` — the CSV is a human
    /// file, and the filters read facts, not this.
    pub slots: String,
    /// The covered body parts by name: "Chest, Upper Arms, Lower Arms".
    pub coverage: String,
    /// Armor level, blank when the item is not armour.
    pub armor_level: String,
    /// Cantrip names by school, joined with `; `; filled by the scan,
    /// which knows each spell's school.
    pub creature_cantrips: String,
    pub item_cantrips: String,
    pub life_cantrips: String,
    /// Soldier / Adept / … / `No set`.
    pub item_set: String,
    pub stack: u32,
    /// The last price the bot quoted, blank until asked.
    pub price: String,
    /// The item's guid, signed the way the bots spell it — always
    /// negative, e.g. -42637. Bots price by this number, so a row click
    /// queries it. It is written to the file as a hint only: a guid from
    /// a closed trade is dead, and the scan refreshes it on sighting.
    /// 0 means "never seen in an open window".
    pub object_id: i64,
    /// The item's icon surface; 0 until the client has described it.
    pub icon_id: u32,
    /// What the cantrips do for the buyer: "Heal Boost 1", "Crit Dam Resist 2".
    pub ratings: String,
    /// The short appraisal line: "AL 312", a stack count, a wield diff.
    pub detail: String,
    /// Unix milliseconds of the most recent sighting.
    pub last_seen: i64,
}

impl Row {
    /// A row for an item the trade window shows but the appraisal has not
    /// answered: the object record already carries name, icon and stack,
    /// which is enough to list it greyed and to price it. `partial` is
    /// what the object record alone can say (category, slots, coverage,
    /// set); the cantrip columns wait for the appraisal.
    #[must_use]
    pub fn from_sighting(
        object: &openac_plugin::world::WorldObject,
        partial: Option<&ItemFacts>,
        now: i64,
    ) -> Self {
        let guid = crate::facts::property_id(object.object_id);
        Self {
            property_id: crate::facts::property_id(object.weenie_class_id),
            name: object.name.clone(),
            category: partial.map_or(Category::Other, |facts| facts.category),
            slots: partial.map_or_else(String::new, |facts| facts.slots.join("; ")),
            coverage: partial.map_or_else(String::new, |facts| facts.covered.clone()),
            armor_level: String::new(),
            creature_cantrips: String::new(),
            item_cantrips: String::new(),
            life_cantrips: String::new(),
            item_set: partial.map_or_else(String::new, |facts| facts.item_set.clone()),
            stack: u32::try_from(object.stack_size.max(1)).unwrap_or(1),
            price: String::new(),
            object_id: guid,
            icon_id: object.icon_id,
            ratings: String::new(),
            detail: String::new(),
            last_seen: now,
        }
    }

    /// A row for a freshly-appraised fact, sighted at `now`.
    #[must_use]
    pub fn from_facts(facts: &ItemFacts, now: i64) -> Self {
        Self {
            property_id: facts.property_id,
            name: facts.name.clone(),
            category: facts.category,
            slots: crate::facts::slot_names(facts.valid_locations).join("; "),
            coverage: crate::facts::coverage_names(facts.coverage_mask),
            armor_level: facts
                .armor_level
                .map_or_else(String::new, |level| level.to_string()),
            creature_cantrips: facts.creature_cantrips.join("; "),
            item_cantrips: facts.item_cantrips.join("; "),
            life_cantrips: facts.life_cantrips.join("; "),
            item_set: facts.item_set.clone(),
            stack: facts.stack_size.max(1),
            price: String::new(),
            object_id: 0,
            icon_id: facts.icon_id,
            ratings: facts.ratings.clone(),
            detail: facts.detail.clone(),
            last_seen: now,
        }
    }
}

/// A bot's whole tracked inventory.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Inventory {
    /// Rows, in file order.
    pub rows: Vec<Row>,
    /// Whether the last read of this bot's stock saw the whole window.
    /// Only a complete read may prune.
    pub complete: bool,
}

impl Inventory {
    /// The file's rows, and whether its complete-read marker was set.
    ///
    /// A malformed line is skipped rather than failing the load: one bad
    /// row must not cost the player their whole cache.
    #[must_use]
    pub fn parse(text: &str) -> Self {
        let mut rows = Vec::new();
        let mut complete = false;
        for (index, line) in text.lines().enumerate() {
            let line = line.trim_end();
            if line.is_empty() {
                continue;
            }
            if index == 0 {
                // The header, or a marker line beside it.
                if line.starts_with("PropertyId,") {
                    complete = line.ends_with(",complete");
                    continue;
                }
            }
            if let Some(flag) = line.strip_prefix("#complete ") {
                complete = flag.trim() == "1";
                continue;
            }
            let fields = split(line);
            if fields.len() < 16 {
                continue;
            }
            let Ok(property_id) = fields[0].parse::<i64>() else {
                continue;
            };
            let Ok(stack) = fields[10].parse::<u32>() else {
                continue;
            };
            let Ok(last_seen) = fields[12].parse::<i64>() else {
                continue;
            };
            rows.push(Row {
                property_id,
                name: fields[1].clone(),
                category: category_of(&fields[2]),
                slots: fields[3].clone(),
                coverage: fields[4].clone(),
                armor_level: fields[5].clone(),
                creature_cantrips: fields[6].clone(),
                item_cantrips: fields[7].clone(),
                life_cantrips: fields[8].clone(),
                item_set: fields[9].clone(),
                stack,
                price: fields[11].clone(),
                last_seen,
                object_id: fields.get(16).and_then(|v| v.parse().ok()).unwrap_or(0),
                icon_id: fields[13].parse().unwrap_or(0),
                ratings: fields[14].clone(),
                detail: fields[15].clone(),
            });
        }
        Self { rows, complete }
    }

    /// The file's text: header, a complete-read marker, then the rows.
    #[must_use]
    pub fn write(&self) -> String {
        let mut out = String::from(HEADER);
        out.push_str(&format!("\n#complete {}\n", u8::from(self.complete)));
        for row in &self.rows {
            out.push_str(&join(row));
            out.push('\n');
        }
        out
    }

    /// Sight `row` (or refresh the matching one), stamping `last_seen`.
    /// The newest facts win; a price already quoted survives a re-scan.
    pub fn sight(&mut self, row: Row) {
        if let Some(existing) = self
            .rows
            .iter_mut()
            .find(|existing| existing.property_id == row.property_id)
        {
            existing.last_seen = row.last_seen;
            existing.stack = row.stack;
            existing.object_id = row.object_id;
            // The sighting's facts win wherever they are known; a price
            // already quoted survives, and so does any richer cached
            // field the new row left blank (cantrips filled by a later
            // pass, a set named by the scan).
            merge(&mut existing.name, &row.name);
            merge(&mut existing.slots, &row.slots);
            merge(&mut existing.coverage, &row.coverage);
            merge(&mut existing.armor_level, &row.armor_level);
            merge(&mut existing.creature_cantrips, &row.creature_cantrips);
            merge(&mut existing.item_cantrips, &row.item_cantrips);
            merge(&mut existing.life_cantrips, &row.life_cantrips);
            merge(&mut existing.item_set, &row.item_set);
            merge(&mut existing.price, &row.price);
            merge(&mut existing.ratings, &row.ratings);
            merge(&mut existing.detail, &row.detail);
            if row.icon_id != 0 {
                existing.icon_id = row.icon_id;
            }
            existing.category = row.category;
        } else {
            self.rows.push(row);
        }
    }

    /// Drop every row whose property id is not in `sighted`, and mark the
    /// read complete. Call only after the whole window has been seen.
    pub fn prune_to(&mut self, sighted: &std::collections::BTreeSet<i64>) {
        self.rows.retain(|row| sighted.contains(&row.property_id));
        self.complete = true;
    }

    /// The row for `property_id`, if tracked.
    #[must_use]
    pub fn find(&self, property_id: i64) -> Option<&Row> {
        self.rows.iter().find(|row| row.property_id == property_id)
    }

    /// Record a price the bot quoted; false when the item is not tracked.
    pub fn set_price(&mut self, property_id: i64, price: &str) -> bool {
        match self
            .rows
            .iter_mut()
            .find(|row| row.property_id == property_id)
        {
            Some(row) => {
                row.price = price.to_owned();
                true
            }
            None => false,
        }
    }
}

/// Bits as hex text, so a human can read the file.
fn bits(value: u32) -> String {
    if value == 0 {
        String::new()
    } else {
        format!("0x{value:X}")
    }
}

/// Take `incoming` when it says something `held` does not.
fn merge(held: &mut String, incoming: &str) {
    if !incoming.is_empty() {
        *held = incoming.to_owned();
    }
}

/// The category a label names; unknown labels read as Other.
fn category_of(label: &str) -> Category {
    [
        Category::Weapons,
        Category::Armor,
        Category::Clothing,
        Category::Jewelry,
        Category::Cloaks,
        Category::Summonons,
        Category::Aetheria,
        Category::Salvage,
        Category::Other,
    ]
    .into_iter()
    .find(|category| category.label() == label)
    .unwrap_or(Category::Other)
}

/// One row as a CSV line. Fields with commas or quotes are quoted.
fn join(row: &Row) -> String {
    [
        row.property_id.to_string(),
        quote(&row.name),
        row.category.label().to_owned(),
        quote(&row.slots),
        row.coverage.clone(),
        row.armor_level.clone(),
        quote(&row.creature_cantrips),
        quote(&row.item_cantrips),
        quote(&row.life_cantrips),
        quote(&row.item_set),
        row.stack.to_string(),
        quote(&row.price),
        row.last_seen.to_string(),
        row.icon_id.to_string(),
        quote(&row.ratings),
        quote(&row.detail),
        row.object_id.to_string(),
    ]
    .join(",")
}

/// Quote a field if it needs it.
fn quote(field: &str) -> String {
    if field.contains(',') || field.contains('"') {
        format!("\"{}\"", field.replace('"', "\"\""))
    } else {
        field.to_owned()
    }
}

/// Split one CSV line, honouring quotes.
fn split(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut field = String::new();
    let mut quoted = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' if quoted && chars.peek() == Some(&'"') => {
                field.push('"');
                chars.next();
            }
            '"' => quoted = !quoted,
            ',' if !quoted => {
                fields.push(std::mem::take(&mut field));
            }
            other => field.push(other),
        }
    }
    fields.push(field);
    fields
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(id: i64, name: &str, seen: i64) -> Row {
        Row {
            property_id: id,
            name: name.to_owned(),
            category: Category::Weapons,
            slots: "Hands".to_owned(),
            coverage: "0x20".to_owned(),
            armor_level: String::new(),
            creature_cantrips: String::new(),
            item_cantrips: "Inferno's Edge".to_owned(),
            life_cantrips: String::new(),
            item_set: "No set".to_owned(),
            stack: 1,
            price: String::new(),
            last_seen: seen,
            object_id: 0,
            icon_id: 0,
            ratings: String::new(),
            detail: String::new(),
        }
    }

    #[test]
    fn a_file_round_trips() {
        let mut inv = Inventory::default();
        inv.rows.push(row(-20601, "Acid Greataxe", 1000));
        inv.rows.push(row(-20632, "Blessed Longsword", 1000));
        inv.complete = true;
        let text = inv.write();
        let back = Inventory::parse(&text);
        assert_eq!(back, inv, "write then parse is the identity");
    }

    #[test]
    fn a_name_with_a_comma_survives() {
        let mut inv = Inventory::default();
        let mut r = row(-1, "Amulet, Greater", 5);
        r.item_cantrips = "A, B; C".to_owned();
        inv.rows.push(r);
        let back = Inventory::parse(&inv.write());
        assert_eq!(back.rows[0].name, "Amulet, Greater");
        assert_eq!(back.rows[0].item_cantrips, "A, B; C");
    }

    #[test]
    fn a_bad_line_is_skipped_not_fatal() {
        let text = format!(
            "{HEADER}\n#complete 1\nnot,a,short,line\n{},\n",
            join(&row(-7, "Good", 1))
        );
        let inv = Inventory::parse(&text);
        assert_eq!(inv.rows.len(), 1, "the good row survives");
        assert_eq!(inv.rows[0].property_id, -7);
        assert!(inv.complete, "the marker was read");
    }

    #[test]
    fn sighting_refreshes_and_keeps_the_price() {
        let mut inv = Inventory::default();
        let mut first = row(-20601, "Acid Greataxe", 1000);
        first.price = "1,200 pts".to_owned();
        inv.rows.push(first);
        inv.sight(row(-20601, "Acid Greataxe", 2000));
        assert_eq!(inv.rows.len(), 1, "one row per property id");
        assert_eq!(inv.rows[0].last_seen, 2000, "the sighting stamped");
        assert_eq!(inv.rows[0].price, "1,200 pts", "the price survived");
    }

    #[test]
    fn a_sighting_fills_blanks_and_keeps_the_price() {
        let mut inv = Inventory::default();
        let mut cached = row(-20601, "Acid Greataxe", 1000);
        cached.price = "1200".to_owned();
        cached.item_set = "Soldier".to_owned();
        inv.rows.push(cached);
        // A later appraisal that knows the set anew but nothing else must
        // not blank the cached fields, and the price survives.
        let mut fresh = row(-20601, "Acid Greataxe", 2000);
        fresh.item_set = String::new();
        inv.sight(fresh);
        assert_eq!(inv.rows[0].price, "1200");
        assert_eq!(inv.rows[0].item_set, "Soldier", "a blank does not erase");
        assert_eq!(inv.rows[0].last_seen, 2000);
    }

    #[test]
    fn a_complete_read_prunes_what_was_not_sighted() {
        let mut inv = Inventory::default();
        inv.rows.push(row(-1, "Still There", 1000));
        inv.rows.push(row(-2, "Sold Away", 1000));
        let mut sighted = std::collections::BTreeSet::new();
        sighted.insert(-1);
        inv.prune_to(&sighted);
        assert_eq!(inv.rows.len(), 1);
        assert_eq!(inv.rows[0].property_id, -1);
        assert!(inv.complete);
    }

    #[test]
    fn an_incomplete_read_never_prunes() {
        // The guard lives in the caller, but the flag is the file's
        // memory of it: a parse of an unfinished scan says so.
        let mut inv = Inventory::default();
        inv.rows.push(row(-1, "Maybe Gone", 1000));
        let back = Inventory::parse(&inv.write());
        assert!(!back.complete, "no prune may run on this file");
    }

    #[test]
    fn a_price_lands_on_its_row() {
        let mut inv = Inventory::default();
        inv.rows.push(row(-20601, "Acid Greataxe", 1));
        assert!(inv.set_price(-20601, "98765 pts"));
        assert_eq!(inv.find(-20601).unwrap().price, "98765 pts");
        assert!(!inv.set_price(-999, "0"), "an untracked id is refused");
    }
}
