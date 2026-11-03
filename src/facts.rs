//! One item as the appraisal described it: the flat, filterable view.
//!
//! Facts are keyed by *property id* — the signed weenie class, the long
//! negative number trade bots identify items by (`check -20601`) — because
//! that is stable across trades, bots and sessions, while the client's
//! object id dies when the trade window closes. The client reports the
//! weenie class unsigned; the bots' convention is its signed reading, and
//! that is what every id in this plugin carries.
//!
//! The bit tables here are the client's own (`ac-core`'s `ItemType` and
//! `EquipMask`), not guesses: the first build of this plugin invented them
//! and drew every weapon as "Other".

/// What one appraisal answered, kept forever under the item's property id.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ItemFacts {
    /// The property id, signed as the bots spell it: -20601.
    pub property_id: i64,
    /// Its name, as the server spells it.
    pub name: String,
    /// Its icon's render surface; 0 until the client has described it.
    pub icon_id: u32,
    /// How many in a stack; 1 for a single item.
    pub stack_size: u32,
    /// The category row of the concept.
    pub category: Category,
    /// The item type bits, kept raw for filters the category folds away.
    pub item_type: u32,
    /// Where it may be worn or wielded (the valid-location bits).
    pub valid_locations: u32,
    /// The body parts an armour piece covers (the retail CoverageMask
    /// bits the client sends with the object).
    pub coverage_mask: u32,
    /// Armor level, when it is armour.
    pub armor_level: Option<i32>,
    /// The cantrips on it, by spell id, as the appraisal listed them.
    pub spell_ids: Vec<u32>,
    /// The cantrip names by school, filled by the scan, which reads the
    /// spell table: each is `Name (Band)`, e.g. "Recklessness (Legendary)".
    pub creature_cantrips: Vec<String>,
    pub item_cantrips: Vec<String>,
    pub life_cantrips: Vec<String>,
    /// The item set named in its title ("Adept Gauntlets" → Adept), or
    /// empty.
    pub item_set: String,
    /// The weapon family, for the Missile/Heavy/Finesse/Light/Shield row.
    pub weapon: Weapon,
    /// Armor level and crit rating, as the Detail column reads them.
    pub crit_rating: Option<i32>,
    /// What the cantrips do for the buyer, as the Ratings column reads
    /// them: "Heal Boost 1, Crit Dam Resist 2". The scan fills it from
    /// the cantrip names, stripping each school's parenthetical band.
    pub ratings: String,
    /// The short appraisal line: "AL 312" for armour, "x24" for a stack.
    pub detail: String,
}

/// The ratings text the concept's column shows: the cantrip names
/// without their bands, joined. "Storm Ward (Legendary)" reads as
/// "Storm Ward"; the band belongs to the cantrip columns.
#[must_use]
pub fn ratings_of(facts: &ItemFacts) -> String {
    let mut parts: Vec<String> = Vec::new();
    for list in [
        &facts.creature_cantrips,
        &facts.item_cantrips,
        &facts.life_cantrips,
    ] {
        for cantrip in list {
            let bare = cantrip
                .strip_suffix(')')
                .and_then(|head| head.rsplit_once('('))
                .map_or_else(|| cantrip.clone(), |(name, _)| name.trim_end().to_owned());
            parts.push(bare);
        }
    }
    parts.join(", ")
}

/// The body parts an armour piece covers, as the concept's Covered
/// column names them. These are the retail `CoverageMask` bits the
/// client sends with the object.
pub mod coverage {
    pub const HEAD: u32 = 0x0000_0001;
    pub const CHEST: u32 = 0x0000_0002;
    pub const ABDOMEN: u32 = 0x0000_0004;
    pub const RIGHT_UPPER_ARM: u32 = 0x0000_0008;
    pub const RIGHT_LOWER_ARM: u32 = 0x0000_0010;
    pub const RIGHT_HAND: u32 = 0x0000_0020;
    pub const LEFT_UPPER_ARM: u32 = 0x0000_0040;
    pub const LEFT_LOWER_ARM: u32 = 0x0000_0080;
    pub const LEFT_HAND: u32 = 0x0000_0100;
    pub const RIGHT_UPPER_LEG: u32 = 0x0000_0200;
    pub const RIGHT_LOWER_LEG: u32 = 0x0000_0400;
    pub const RIGHT_FOOT: u32 = 0x0000_0800;
    pub const LEFT_UPPER_LEG: u32 = 0x0000_1000;
    pub const LEFT_LOWER_LEG: u32 = 0x0000_2000;
    pub const LEFT_FOOT: u32 = 0x0000_4000;
}

/// The covered areas an item's coverage bits name, joined for the cell:
/// left and right read as one ("Arms", "Legs", "Feet", "Hands").
#[must_use]
pub fn coverage_names(mask: u32) -> String {
    let mut parts: Vec<&str> = Vec::new();
    let both = |a: u32, b: u32| mask & (a | b) != 0;
    if mask & coverage::HEAD != 0 {
        parts.push("Head");
    }
    if mask & coverage::CHEST != 0 {
        parts.push("Chest");
    }
    if mask & coverage::ABDOMEN != 0 {
        parts.push("Abdomen");
    }
    if both(coverage::RIGHT_UPPER_ARM, coverage::LEFT_UPPER_ARM) {
        parts.push("Upper Arms");
    }
    if both(coverage::RIGHT_LOWER_ARM, coverage::LEFT_LOWER_ARM) {
        parts.push("Lower Arms");
    }
    if both(coverage::RIGHT_HAND, coverage::LEFT_HAND) {
        parts.push("Hands");
    }
    if both(coverage::RIGHT_UPPER_LEG, coverage::LEFT_UPPER_LEG) {
        parts.push("Upper Legs");
    }
    if both(coverage::RIGHT_LOWER_LEG, coverage::LEFT_LOWER_LEG) {
        parts.push("Lower Legs");
    }
    if both(coverage::RIGHT_FOOT, coverage::LEFT_FOOT) {
        parts.push("Feet");
    }
    parts.join(", ")
}

/// The signed reading of a weenie class: the bots' property id.
#[must_use]
pub const fn property_id(weenie_class_id: u32) -> i64 {
    weenie_class_id as i32 as i64
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
}

/// `ItemType` bits, as `ac-core` defines them.
pub mod item_type {
    pub const MELEE_WEAPON: u32 = 0x0000_0001;
    pub const ARMOR: u32 = 0x0000_0002;
    pub const CLOTHING: u32 = 0x0000_0004;
    pub const JEWELRY: u32 = 0x0000_0008;
    pub const MISSILE_WEAPON: u32 = 0x0000_0100;
    pub const GEM: u32 = 0x0000_0800;
    pub const CASTER: u32 = 0x0000_8000;
    pub const TINKERING_MATERIAL: u32 = 0x4000_0000;
}

/// `EquipMask` bits (valid locations), as `ac-core` defines them.
pub mod slot {
    pub const HEAD: u32 = 0x0000_0001;
    pub const CHEST: u32 = 0x0000_0002;
    pub const ABDOMEN: u32 = 0x0000_0004;
    pub const UPPER_ARMS: u32 = 0x0000_0008;
    pub const LOWER_ARMS: u32 = 0x0000_0010;
    pub const HANDS: u32 = 0x0000_0020;
    pub const UPPER_LEGS: u32 = 0x0000_0040;
    pub const LOWER_LEGS: u32 = 0x0000_0080;
    pub const FEET: u32 = 0x0000_0100;
    pub const NECKLACE: u32 = 0x0000_8000;
    pub const BRACELET_L: u32 = 0x0001_0000;
    pub const BRACELET_R: u32 = 0x0002_0000;
    pub const RING_L: u32 = 0x0004_0000;
    pub const RING_R: u32 = 0x0008_0000;
    pub const MELEE_LOCATION: u32 = 0x0010_0000;
    pub const SHIELD: u32 = 0x0020_0000;
    pub const MISSILE_LOCATION: u32 = 0x0040_0000;
    pub const TWO_HANDED: u32 = 0x0200_0000;
    pub const TRINKET: u32 = 0x0400_0000;
    pub const CLOAK: u32 = 0x0800_0000;
}

/// The weapon row of the concept: what family a wielded weapon belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Weapon {
    #[default]
    None,
    Missile,
    Heavy,
    Finesse,
    Light,
    Shield,
}

/// The category of an item, from its `ItemType` bits and its slots.
///
/// Order matters where the game's own families overlap: a cloak is
/// clothing that wears the cloak slot, so the slot decides first; a
/// jewelry bit with a caddee-style gem is Aetheria only when it surges
/// (which needs its cantrips — the scan upgrades those), and reads as
/// Summonons for plain gems.
#[must_use]
pub fn category_of(item_type: u32, valid_locations: u32) -> Category {
    use crate::facts::item_type as t;
    if valid_locations & slot::CLOAK != 0 {
        return Category::Cloaks;
    }
    if valid_locations & slot::TRINKET != 0 {
        return Category::Jewelry;
    }
    if item_type & t::GEM != 0 {
        return Category::Summonons;
    }
    if item_type & (t::MELEE_WEAPON | t::MISSILE_WEAPON | t::CASTER) != 0
        || valid_locations & (slot::SHIELD) != 0
    {
        return Category::Weapons;
    }
    if item_type & t::ARMOR != 0 {
        return Category::Armor;
    }
    if item_type & t::CLOTHING != 0 {
        return Category::Clothing;
    }
    if item_type & t::JEWELRY != 0 {
        return Category::Jewelry;
    }
    if item_type & t::TINKERING_MATERIAL != 0 {
        return Category::Salvage;
    }
    Category::Other
}

/// The weapon family, from the wielded slot and the wield difficulty.
///
/// The concept's split is the player's shopping question — which skill
/// the item needs. Missile reads the slot; the melee families split on
/// difficulty, the same line the game draws: light weapons cap at 250,
/// finesse at 300, and heavier is heavy. A shield is its own family.
#[must_use]
pub fn weapon_of(valid_locations: u32, wield_difficulty: Option<i32>) -> Weapon {
    if valid_locations & slot::SHIELD != 0 {
        return Weapon::Shield;
    }
    if valid_locations & slot::MISSILE_LOCATION != 0 {
        return Weapon::Missile;
    }
    if valid_locations & (slot::MELEE_LOCATION | slot::TWO_HANDED) == 0 {
        return Weapon::None;
    }
    match wield_difficulty.unwrap_or(0) {
        0..=250 => Weapon::Light,
        251..=300 => Weapon::Finesse,
        _ => Weapon::Heavy,
    }
}

/// The slot names the concept's Slots column shows, in its order. A bit
/// list becomes "Necklace", "Hands", "Ring" (left and right read as one),
/// "Upper Arms", and so on.
#[must_use]
pub fn slot_names(valid_locations: u32) -> Vec<&'static str> {
    let mut out = Vec::new();
    macro_rules! push {
        ($bits:expr, $name:expr) => {
            if valid_locations & $bits != 0 {
                out.push($name);
            }
        };
    }
    push!(slot::NECKLACE, "Necklace");
    if valid_locations & (slot::BRACELET_L | slot::BRACELET_R) != 0 {
        out.push("Bracelet");
    }
    if valid_locations & (slot::RING_L | slot::RING_R) != 0 {
        out.push("Ring");
    }
    push!(slot::TRINKET, "Trinket");
    push!(slot::CLOAK, "Cloak");
    push!(slot::HEAD, "Head");
    push!(slot::CHEST, "Chest");
    push!(slot::ABDOMEN, "Abdomen");
    push!(slot::UPPER_ARMS, "Upper Arms");
    push!(slot::LOWER_ARMS, "Lower Arms");
    push!(slot::HANDS, "Hands");
    push!(slot::UPPER_LEGS, "Upper Legs");
    push!(slot::LOWER_LEGS, "Lower Legs");
    push!(slot::FEET, "Feet");
    out
}

/// The cantrip level bands, as the concept names them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Band {
    Minor,
    Moderate,
    Major,
    Epic,
    Legendary,
}

impl ItemFacts {
    /// The highest band any of the item's cantrips reaches, from the
    /// `(Band)` suffix the scan writes into each name.
    #[must_use]
    pub fn highest_band(&self) -> Option<Band> {
        self.creature_cantrips
            .iter()
            .chain(&self.item_cantrips)
            .chain(&self.life_cantrips)
            .filter_map(|cantrip| {
                let tail = cantrip.rsplit('(').next()?.strip_suffix(')')?;
                Some(match tail {
                    "Minor" => Band::Minor,
                    "Moderate" => Band::Moderate,
                    "Major" => Band::Major,
                    "Epic" => Band::Epic,
                    "Legendary" => Band::Legendary,
                    _ => return None,
                })
            })
            .max_by_key(|band| band.rank())
    }
}

impl Band {
    /// The spell tier's band: 1–3 Minor, 4 Moderate, 5 Major, 6 Epic,
    /// 7–8 Legendary.
    #[must_use]
    pub fn of_tier(tier: i32) -> Self {
        match tier {
            1..=3 => Self::Minor,
            4 => Self::Moderate,
            5 => Self::Major,
            6 => Self::Epic,
            _ => Self::Legendary,
        }
    }

    /// Its place in the row, so the highest cantrip wins.
    #[must_use]
    pub const fn rank(self) -> u8 {
        match self {
            Self::Minor => 0,
            Self::Moderate => 1,
            Self::Major => 2,
            Self::Epic => 3,
            Self::Legendary => 4,
        }
    }

    /// Its name, for the cell text.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Minor => "Minor",
            Self::Moderate => "Moderate",
            Self::Major => "Major",
            Self::Epic => "Epic",
            Self::Legendary => "Legendary",
        }
    }
}

/// The item set named in an item's title. The game's armour and jewelry
/// sets all read "<Set> <Slot>", and the weapon and gem sets share the
/// list; anything else has no set.
#[must_use]
pub fn set_of(name: &str) -> &'static str {
    const SETS: &[&str] = &[
        "Soldier",
        "Adept",
        "Archer",
        "Defender",
        "Tinker",
        "Crafter",
        "Hearty",
        "Dextrous",
        "Wise",
        "Swift",
        "Hardened",
        "Reinforced",
        "Interlocking",
        "Flame Proof",
        "Acid Proof",
        "Cold Proof",
        "Lightning Proof",
        "Dedication",
    ];
    let lower = name.to_lowercase();
    SETS.iter()
        .find(|set| lower.starts_with(&set.to_lowercase()))
        .map_or("", |set| set.as_ref())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::facts::item_type as t;
    use crate::facts::slot as s;

    #[test]
    fn property_ids_read_signed_as_the_bots_spell_them() {
        // 0xFFFF_AF87 is the client's unsigned reading of -20601.
        assert_eq!(property_id(0xFFFF_AF87), -20601);
        assert_eq!(property_id(20601), 20601, "a small id stays positive");
    }

    #[test]
    fn a_dagger_is_a_weapon_not_other() {
        // The bug the first build shipped: invented bits made every item
        // Other.
        assert_eq!(
            category_of(t::MELEE_WEAPON, s::MELEE_LOCATION),
            Category::Weapons
        );
        assert_eq!(
            category_of(t::MISSILE_WEAPON, s::MISSILE_LOCATION),
            Category::Weapons
        );
        assert_eq!(category_of(t::CASTER, s::MELEE_LOCATION), Category::Weapons);
    }

    #[test]
    fn armour_clothing_jewelry_gems_and_salvage_read_themselves() {
        assert_eq!(category_of(t::ARMOR, s::CHEST), Category::Armor);
        assert_eq!(category_of(t::CLOTHING, s::HEAD), Category::Clothing);
        assert_eq!(category_of(t::JEWELRY, s::RING_L), Category::Jewelry);
        assert_eq!(category_of(t::GEM, 0), Category::Summonons);
        assert_eq!(category_of(t::TINKERING_MATERIAL, 0), Category::Salvage);
    }

    #[test]
    fn a_cloak_or_trinket_reads_by_its_slot() {
        assert_eq!(category_of(t::CLOTHING, s::CLOAK), Category::Cloaks);
        assert_eq!(category_of(t::JEWELRY, s::TRINKET), Category::Jewelry);
    }

    #[test]
    fn weapon_families_split_on_the_wield_line() {
        assert_eq!(weapon_of(s::SHIELD, Some(0)), Weapon::Shield);
        assert_eq!(weapon_of(s::MISSILE_LOCATION, Some(150)), Weapon::Missile);
        assert_eq!(weapon_of(s::MELEE_LOCATION, Some(250)), Weapon::Light);
        assert_eq!(weapon_of(s::MELEE_LOCATION, Some(275)), Weapon::Finesse);
        assert_eq!(weapon_of(s::TWO_HANDED, Some(325)), Weapon::Heavy);
        assert_eq!(weapon_of(s::NECKLACE, None), Weapon::None);
    }

    #[test]
    fn slots_name_themselves_in_the_concept_order() {
        assert_eq!(slot_names(s::NECKLACE), ["Necklace"]);
        assert_eq!(
            slot_names(s::BRACELET_L | s::BRACELET_R),
            ["Bracelet"],
            "left and right read as one"
        );
        assert_eq!(
            slot_names(s::CHEST | s::ABDOMEN | s::UPPER_ARMS),
            ["Chest", "Abdomen", "Upper Arms"]
        );
        assert!(slot_names(0).is_empty());
    }

    #[test]
    fn bands_follow_the_concept_row() {
        assert_eq!(Band::of_tier(1), Band::Minor);
        assert_eq!(Band::of_tier(3), Band::Minor);
        assert_eq!(Band::of_tier(4), Band::Moderate);
        assert_eq!(Band::of_tier(5), Band::Major);
        assert_eq!(Band::of_tier(6), Band::Epic);
        assert_eq!(Band::of_tier(7), Band::Legendary);
        assert_eq!(Band::of_tier(8), Band::Legendary);
    }

    #[test]
    fn sets_read_their_name_from_the_title() {
        assert_eq!(set_of("Adept Gauntlets"), "Adept");
        assert_eq!(set_of("Soldier Bow"), "Soldier");
        assert_eq!(set_of("Flame Proof Brogues"), "Flame Proof");
        assert_eq!(set_of("Acid Greataxe"), "");
        assert_eq!(set_of("Dagger"), "");
    }
}
