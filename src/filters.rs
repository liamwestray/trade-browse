//! The filter state: every checkbox row of the concept, as bits.
//!
//! Semantics are the auction-house ones: a row ORs within itself (any
//! checked category passes the category row) and the rows AND across
//! (a checked category *and* a checked slot must both pass). An all-empty
//! row means "no filter here" — including the cantrip rows, where the
//! concept's "Regular only" is its own switch: with no cantrip row
//! checked, everything passes; with "Regular only", an item with no
//! cantrips at all passes and one with them does not.
//!
//! Pure over [`ItemFacts`], so every rule is testable without a host.

use crate::facts::{Band, Category, ItemFacts, Weapon, slot};

/// The category row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Categories(u32);

impl Categories {
    /// The row's choices, in the concept's order.
    pub const ALL: [Category; 9] = [
        Category::Weapons,
        Category::Armor,
        Category::Clothing,
        Category::Jewelry,
        Category::Cloaks,
        Category::Summonons,
        Category::Aetheria,
        Category::Salvage,
        Category::Other,
    ];

    fn bit(category: Category) -> u32 {
        1 << Self::ALL.iter().position(|c| *c == category).unwrap_or(8)
    }

    /// Whether `category` is checked.
    #[must_use]
    pub fn has(self, category: Category) -> bool {
        self.0 & Self::bit(category) != 0
    }

    /// The row with `category` toggled.
    #[must_use]
    pub fn toggled(self, category: Category) -> Self {
        Self(self.0 ^ Self::bit(category))
    }

    /// Whether nothing is checked.
    #[must_use]
    pub fn is_empty(self) -> bool {
        self.0 == 0
    }
}

/// The slot row: the concept's 14 wear slots.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Slots(u32);

/// A slot choice: its label and the valid-location bits it matches.
pub const SLOT_CHOICES: [(&str, u32); 14] = [
    ("Necklace", slot::NECKLACE),
    ("Bracelet", slot::BRACELET_L | slot::BRACELET_R),
    ("Ring", slot::RING_L | slot::RING_R),
    ("Trinket", slot::TRINKET),
    ("Cloak", slot::CLOAK),
    ("Head", slot::HEAD),
    ("Chest", slot::CHEST),
    ("Abdomen", slot::ABDOMEN),
    ("Upper Arms", slot::UPPER_ARMS),
    ("Lower Arms", slot::LOWER_ARMS),
    ("Hands", slot::HANDS),
    ("Upper Legs", slot::UPPER_LEGS),
    ("Lower Legs", slot::LOWER_LEGS),
    ("Feet", slot::FEET),
];

impl Slots {
    /// Whether choice `index` is checked.
    #[must_use]
    pub fn has(self, index: usize) -> bool {
        self.0 & (1 << index) != 0
    }

    /// The row with choice `index` toggled.
    #[must_use]
    pub fn toggled(self, index: usize) -> Self {
        Self(self.0 ^ (1 << index))
    }

    /// Whether nothing is checked.
    #[must_use]
    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Whether `facts` may be worn in any checked slot.
    #[must_use]
    pub fn matches(self, facts: &ItemFacts) -> bool {
        self.is_empty()
            || SLOT_CHOICES
                .iter()
                .enumerate()
                .any(|(index, (_, bits))| self.has(index) && facts.valid_locations & bits != 0)
    }
}

/// The weapon row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Weapons(u32);

pub const WEAPON_CHOICES: [(&str, Weapon); 5] = [
    ("Missile", Weapon::Missile),
    ("Heavy", Weapon::Heavy),
    ("Finesse", Weapon::Finesse),
    ("Light", Weapon::Light),
    ("Shield", Weapon::Shield),
];

impl Weapons {
    #[must_use]
    pub fn has(self, index: usize) -> bool {
        self.0 & (1 << index) != 0
    }

    #[must_use]
    pub fn toggled(self, index: usize) -> Self {
        Self(self.0 ^ (1 << index))
    }

    #[must_use]
    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Whether `facts` is wielded as one of the checked families. An item
    /// that is not a weapon fails a checked row — the row is about
    /// weapons.
    #[must_use]
    pub fn matches(self, facts: &ItemFacts) -> bool {
        self.is_empty()
            || WEAPON_CHOICES
                .iter()
                .enumerate()
                .any(|(index, (_, weapon))| self.has(index) && facts.weapon == *weapon)
    }
}

/// The cantrip row: which schools may appear, and "Regular only".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Cantrips {
    /// Creature cantrips allowed.
    pub creature: bool,
    /// Item cantrips allowed.
    pub item: bool,
    /// Life cantrips allowed.
    pub life: bool,
    /// Only items with no cantrips at all.
    pub regular_only: bool,
}

impl Cantrips {
    /// Whether any school is checked.
    #[must_use]
    pub fn any_school(self) -> bool {
        self.creature || self.item || self.life
    }

    /// Whether `facts` passes. A school row asks for items carrying that
    /// school; "Regular only" asks for items carrying none. Both at once
    /// is the player's problem, not the filter's: schools win.
    #[must_use]
    pub fn matches(self, facts: &ItemFacts) -> bool {
        if self.regular_only && !self.any_school() {
            return facts.creature_cantrips.is_empty()
                && facts.item_cantrips.is_empty()
                && facts.life_cantrips.is_empty();
        }
        if !self.any_school() {
            return true;
        }
        (self.creature && !facts.creature_cantrips.is_empty())
            || (self.item && !facts.item_cantrips.is_empty())
            || (self.life && !facts.life_cantrips.is_empty())
    }
}

/// The cantrip level band row: the highest band an item's cantrips reach
/// must be the checked one (the concept draws it as a segmented bar).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Bands {
    /// The checked band, if any.
    pub checked: Option<Band>,
}

impl Bands {
    /// Whether `facts`' cantrips reach exactly `band` at their highest.
    /// An item with no cantrips passes an unchecked row and fails a
    /// checked one.
    #[must_use]
    pub fn matches(self, facts: &ItemFacts) -> bool {
        let Some(band) = self.checked else {
            return true;
        };
        facts.highest_band() == Some(band)
    }
}

/// The set row: the concept's 18 sets plus "No set" and "Other set".
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Sets {
    /// Checked set names; empty means no filter.
    pub checked: Vec<String>,
    /// "No set": items with no set name.
    pub none: bool,
    /// "Other set": a set the row does not name.
    pub other: bool,
}

impl Sets {
    /// Whether `facts` belongs to a checked set, or to the none/other
    /// switches.
    #[must_use]
    pub fn matches(&self, facts: &ItemFacts) -> bool {
        if self.checked.is_empty() && !self.none && !self.other {
            return true;
        }
        if facts.item_set.is_empty() {
            return self.none;
        }
        if self.checked.iter().any(|set| set == &facts.item_set) {
            return true;
        }
        self.other
    }
}

/// The whole filter state: one field per concept row.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FilterState {
    pub categories: Categories,
    /// Regular / Unenchentable: whether the item may carry cantrips.
    pub unenchentable_only: bool,
    pub weapons: Weapons,
    pub slots: Slots,
    pub cantrips: Cantrips,
    pub bands: Bands,
    pub sets: Sets,
}

impl FilterState {
    /// Whether `facts` passes every checked row.
    #[must_use]
    pub fn matches(&self, facts: &ItemFacts) -> bool {
        (self.categories.is_empty() || self.categories.has(facts.category))
            && self.weapons.matches(facts)
            && self.slots.matches(facts)
            && self.cantrips.matches(facts)
            && self.bands.matches(facts)
            && self.sets.matches(facts)
    }

    /// The concept's "Reset filters": every row empty.
    #[must_use]
    pub fn reset(&self) -> Self {
        Self::default()
    }

    /// The rows' summary line: what is checked, in words.
    #[must_use]
    pub fn describe(&self) -> String {
        let mut parts: Vec<String> = Vec::new();
        if !self.categories.is_empty() {
            let names: Vec<&str> = Categories::ALL
                .into_iter()
                .filter(|c| self.categories.has(*c))
                .map(Category::label)
                .collect();
            parts.push(names.join("/"));
        }
        if self.unenchentable_only {
            parts.push("Unenchentable".to_owned());
        }
        if !self.weapons.is_empty() {
            let names: Vec<&str> = WEAPON_CHOICES
                .iter()
                .enumerate()
                .filter(|(index, _)| self.weapons.has(*index))
                .map(|(_, (name, _))| *name)
                .collect();
            parts.push(names.join("/"));
        }
        if !self.slots.is_empty() {
            let names: Vec<&str> = SLOT_CHOICES
                .iter()
                .enumerate()
                .filter(|(index, _)| self.slots.has(*index))
                .map(|(_, (name, _))| *name)
                .collect();
            parts.push(names.join("/"));
        }
        if self.cantrips.regular_only {
            parts.push("Regular only".to_owned());
        } else if self.cantrips.any_school() {
            let mut schools = Vec::new();
            if self.cantrips.creature {
                schools.push("Creature");
            }
            if self.cantrips.item {
                schools.push("Item");
            }
            if self.cantrips.life {
                schools.push("Life");
            }
            parts.push(schools.join("/"));
        }
        if let Some(band) = self.bands.checked {
            parts.push(format!("{} cantrips", band.label()));
        }
        if !self.sets.checked.is_empty() {
            parts.push(self.sets.checked.join("/"));
        }
        if self.sets.none {
            parts.push("No set".to_owned());
        }
        if self.sets.other {
            parts.push("Other set".to_owned());
        }
        if parts.is_empty() {
            "No filters".to_owned()
        } else {
            parts.join(" · ")
        }
    }
}

/// Whether an item can carry cantrips: the concept's Regular /
/// Unenchentable row. An unenchentable item has no cantrips and no room
/// for them; the client's honest signal is the cantrip list itself, so
/// the row reads "this item has cantrips" as enchantable.
#[must_use]
pub fn is_unenchentable(facts: &ItemFacts) -> bool {
    facts.spell_ids.is_empty() && facts.category != Category::Other && !facts.name.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::facts::ItemFacts;

    fn greataxe() -> ItemFacts {
        ItemFacts {
            property_id: -20601,
            name: "Acid Greataxe".to_owned(),
            category: Category::Weapons,
            item_type: crate::facts::item_type::MELEE_WEAPON,
            valid_locations: slot::TWO_HANDED,
            weapon: Weapon::Heavy,
            item_cantrips: vec!["Inferno's Edge (Legendary)".to_owned()],
            ..Default::default()
        }
    }

    fn gauntlets() -> ItemFacts {
        ItemFacts {
            property_id: -100,
            name: "Adept Gauntlets".to_owned(),
            category: Category::Armor,
            item_type: crate::facts::item_type::ARMOR,
            valid_locations: slot::HANDS,
            item_set: "Adept".to_owned(),
            creature_cantrips: vec!["Recklessness (Legendary)".to_owned()],
            ..Default::default()
        }
    }

    #[test]
    fn an_empty_state_passes_everything() {
        let f = FilterState::default();
        assert!(f.matches(&greataxe()));
        assert!(f.matches(&gauntlets()));
        assert_eq!(f.describe(), "No filters");
    }

    #[test]
    fn categories_or_within_the_row() {
        let f = FilterState {
            categories: Categories::default()
                .toggled(Category::Weapons)
                .toggled(Category::Armor),
            ..Default::default()
        };
        assert!(f.matches(&greataxe()));
        assert!(f.matches(&gauntlets()));
        let only_armor = FilterState {
            categories: Categories::default().toggled(Category::Armor),
            ..Default::default()
        };
        assert!(!only_armor.matches(&greataxe()));
    }

    #[test]
    fn rows_and_across_each_other() {
        let f = FilterState {
            categories: Categories::default().toggled(Category::Weapons),
            slots: Slots(1 << 10), // Hands
            ..Default::default()
        };
        assert!(!f.matches(&greataxe()), "a weapon wears no hands");
    }

    #[test]
    fn slots_match_their_bits() {
        let hands = SLOT_CHOICES
            .iter()
            .position(|(name, _)| *name == "Hands")
            .unwrap();
        let f = FilterState {
            slots: Slots(1 << hands),
            ..Default::default()
        };
        assert!(f.matches(&gauntlets()));
        assert!(!f.matches(&greataxe()));
    }

    #[test]
    fn weapon_row_splits_the_families() {
        let heavy = WEAPON_CHOICES
            .iter()
            .position(|(_, w)| *w == Weapon::Heavy)
            .unwrap();
        let f = FilterState {
            weapons: Weapons(1 << heavy),
            ..Default::default()
        };
        assert!(f.matches(&greataxe()));
        assert!(!f.matches(&gauntlets()));
    }

    #[test]
    fn regular_only_wants_the_cantripless() {
        let f = FilterState {
            cantrips: Cantrips {
                regular_only: true,
                ..Default::default()
            },
            ..Default::default()
        };
        assert!(!f.matches(&greataxe()), "the axe has a cantrip");
        let plain = ItemFacts {
            name: "Dagger".to_owned(),
            ..Default::default()
        };
        assert!(f.matches(&plain));
    }

    #[test]
    fn a_school_row_wants_that_school() {
        let f = FilterState {
            cantrips: Cantrips {
                creature: true,
                ..Default::default()
            },
            ..Default::default()
        };
        assert!(f.matches(&gauntlets()));
        assert!(!f.matches(&greataxe()));
    }

    #[test]
    fn a_band_row_reads_the_highest_cantrip() {
        let f = FilterState {
            bands: Bands {
                checked: Some(Band::Legendary),
            },
            ..Default::default()
        };
        assert!(f.matches(&greataxe()));
        assert!(f.matches(&gauntlets()));
        let epic = FilterState {
            bands: Bands {
                checked: Some(Band::Epic),
            },
            ..Default::default()
        };
        assert!(!epic.matches(&greataxe()));
    }

    #[test]
    fn sets_match_by_name_or_by_being_setless() {
        let adept = FilterState {
            sets: Sets {
                checked: vec!["Adept".to_owned()],
                ..Default::default()
            },
            ..Default::default()
        };
        assert!(adept.matches(&gauntlets()));
        assert!(!adept.matches(&greataxe()));
        let none = FilterState {
            sets: Sets {
                none: true,
                ..Default::default()
            },
            ..Default::default()
        };
        assert!(none.matches(&greataxe()));
        assert!(!none.matches(&gauntlets()));
    }

    #[test]
    fn the_summary_names_what_is_checked() {
        let f = FilterState {
            categories: Categories::default().toggled(Category::Armor),
            cantrips: Cantrips {
                life: true,
                ..Default::default()
            },
            bands: Bands {
                checked: Some(Band::Legendary),
            },
            ..Default::default()
        };
        let text = f.describe();
        assert!(text.contains("Armor"), "{text}");
        assert!(text.contains("Life"), "{text}");
        assert!(text.contains("Legendary"), "{text}");
    }
}
