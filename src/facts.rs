//! One item as the appraisal described it: the flat, filterable view.
//!
//! Facts are keyed by *property id* — the weenie class the server calls the
//! item's "original type", the long negative number players see — because
//! that is stable across trades, bots and sessions, while the client's
//! object id dies when the trade window closes.
//!
//! This module is pure: it turns the SDK's property bags into one small
//! struct, so the interesting mistakes (a coverage bit read as a slot, a
//! cantrip school off by one) are testable without a running client.

/// What one appraisal answered, kept forever under the item's property id.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ItemFacts {
    /// The property id (signed weenie class), e.g. -20601.
    pub property_id: i64,
    /// Its name, as the server spells it.
    pub name: String,
    /// Its icon's render surface; 0 until the client has described it.
    pub icon_id: u32,
    /// How many in a stack; 1 for a single item.
    pub stack_size: u32,
    /// The category row of the concept: Weapons, Armor, Clothing, Jewelry,
    /// Cloaks, Summonons, Aetheria, Salvage, Other.
    pub category: Category,
    /// The item type bits, kept raw for filters the category folds away.
    pub item_type: u32,
    /// Where it may be worn or wielded (the valid-location bits).
    pub valid_locations: u32,
    /// The body parts a worn piece covers (the coverage bits).
    pub coverage_mask: u32,
    /// Armor level, when it is armour.
    pub armor_level: Option<i32>,
    /// The cantrips on it, by spell id, as the appraisal listed them.
    pub spell_ids: Vec<u32>,
}

/// The concept window's first filter row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Category {
    Weapons,
    Armor,
    Clothing,
    Jewelry,
    Cloaks,
    Summonons,
    Aetheria,
    Salvage,
    #[default]
    Other,
}

impl Category {
    /// Its name, for the UI and the CSV.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Weapons => "Weapons",
            Self::Armor => "Armor",
            Self::Clothing => "Clothing",
            Self::Jewelry => "Jewelry",
            Self::Cloaks => "Cloaks",
            Self::Summonons => "Summonons",
            Self::Aetheria => "Aetheria",
            Self::Salvage => "Salvage",
            Self::Other => "Other",
        }
    }

    /// The category of an item with these `item_type` bits.
    ///
    /// The bits follow the retail `ItemType` flags the client reports
    /// verbatim. Order matters: a summonon gem is checked before the
    /// armour and weapon families because nothing else carries both bits.
    /// Aetheria reads as Summonons here — telling the two apart needs the
    /// item's cantrips, which the scan knows and this does not — and the
    /// scan upgrades a ring that can surge to `Aetheria`.
    #[must_use]
    pub fn from_item_type(bits: u32) -> Self {
        // Retail ItemType bits.
        const MELEE_WEAPON: u32 = 0x0001_0000;
        const MISSILE_WEAPON: u32 = 0x0002_0000;
        const CADDELL: u32 = 0x0004_0000; // a gem: summonon or aetheria
        const ARMOR: u32 = 0x0010_0000;
        const CLOTHING: u32 = 0x0020_0000;
        const JEWELRY: u32 = 0x0040_0000;
        const SHIELD: u32 = 0x0400_0000;
        const GEM: u32 = 0x0000_0040;

        if bits & (CADDELL | GEM) != 0 {
            return Self::Summonons;
        }
        if bits & (MELEE_WEAPON | MISSILE_WEAPON | SHIELD) != 0 {
            return Self::Weapons;
        }
        if bits & ARMOR != 0 {
            return Self::Armor;
        }
        if bits & CLOTHING != 0 {
            return Self::Clothing;
        }
        if bits & JEWELRY != 0 {
            return Self::Jewelry;
        }
        Self::Other
    }
}

#[cfg(test)]
mod tests {
    use super::Category;

    #[test]
    fn weapons_are_weapons() {
        assert_eq!(Category::from_item_type(0x0001_0000), Category::Weapons);
        assert_eq!(Category::from_item_type(0x0002_0000), Category::Weapons);
        assert_eq!(Category::from_item_type(0x0400_0000), Category::Weapons);
    }

    #[test]
    fn armour_clothing_and_jewellery_read_themselves() {
        assert_eq!(Category::from_item_type(0x0010_0000), Category::Armor);
        assert_eq!(Category::from_item_type(0x0020_0000), Category::Clothing);
        assert_eq!(Category::from_item_type(0x0040_0000), Category::Jewelry);
    }

    #[test]
    fn a_gem_is_a_summonon_until_a_surge_says_otherwise() {
        assert_eq!(Category::from_item_type(0x0004_0000), Category::Summonons);
        assert_eq!(Category::from_item_type(0x40), Category::Summonons);
    }

    #[test]
    fn nothing_recognised_is_other() {
        assert_eq!(Category::from_item_type(0), Category::Other);
        assert_eq!(Category::from_item_type(0x8000_0000), Category::Other);
    }
}
