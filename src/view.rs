//! The window's markup and the rows it shows.
//!
//! The concept's filter rows are drawn as real checkboxes — one bool
//! binding each, toggled by an `Action` handler that names the row and
//! index — above a list whose columns are the concept's columns. The
//! cantrip level band is a row of buttons whose captions carry a check
//! mark when selected (a button has no checked state; the caption is the
//! honest one). Icons are the items' own art (`iconkind="did"`), the
//! pattern proven in spell-research.
//!
//! The markup is parsed in a test against the same binding list the
//! plugin registers, so a misspelled element, a missing required
//! attribute or an undeclared binding fails the build instead of leaving
//! the window silently absent.

use crate::filters::{Categories, FilterState, Sets};
use crate::inventory::Row;

/// A row's text in full colour.
const COLOUR_HAVE: u32 = 0xFF_E8E8E8;
/// A row's text greyed: not yet appraised.
const COLOUR_PENDING: u32 = 0xFF_5A5A5A;
/// A price the bot quoted.
const COLOUR_PRICE: u32 = 0xFF_69B369;

/// A binding's type.
///
/// The SDK's declaration and the markup checker each use an enum of this
/// shape, and there is no conversion between them, so the list is spelled
/// once here and mapped both ways. That is what keeps the window's
/// bindings and its markup from drifting apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Int,
    Text,
    Bool,
    UintList,
    TextList,
    FloatList,
    /// A click.
    Action,
    /// A checkbox's `onchange`: `Action<bool>`; the new state arrives on
    /// the wire as `ActionValue::Int(1|0)`.
    BoolAction,
    /// A list row clicked.
    IntAction,
    /// A header clicked: (column, descending).
    IntBoolAction,
    /// Column widths changed.
    FloatListAction,
}

/// The category row's bindings, in `Categories::ALL` order.
pub const CATEGORY_BINDINGS: [&str; 9] = [
    "CatWeapons",
    "CatArmor",
    "CatClothing",
    "CatJewelry",
    "CatCloaks",
    "CatSummonons",
    "CatAetheria",
    "CatSalvage",
    "CatOther",
];

/// The weapon row's bindings, in `WEAPON_CHOICES` order.
pub const WEAPON_BINDINGS: [&str; 5] = [
    "WpnMissile",
    "WpnHeavy",
    "WpnFinesse",
    "WpnLight",
    "WpnShield",
];

/// The slot row's bindings, in `SLOT_CHOICES` order.
pub const SLOT_BINDINGS: [&str; 14] = [
    "SlotNecklace",
    "SlotBracelet",
    "SlotRing",
    "SlotTrinket",
    "SlotCloak",
    "SlotHead",
    "SlotChest",
    "SlotAbdomen",
    "SlotUpperArms",
    "SlotLowerArms",
    "SlotHands",
    "SlotUpperLegs",
    "SlotLowerLegs",
    "SlotFeet",
];

/// The cantrip row's bindings: three schools and "Regular only".
pub const CANTRIP_BINDINGS: [&str; 4] = ["CanCreature", "CanItem", "CanLife", "CanRegular"];

/// The band row's bindings, in order.
pub const BAND_BINDINGS: [&str; 5] = [
    "BandMinor",
    "BandModerate",
    "BandMajor",
    "BandEpic",
    "BandLegendary",
];

/// The categories behind `CATEGORY_BINDINGS`, same order.
pub const CATEGORIES: [crate::facts::Category; 9] = [
    crate::facts::Category::Weapons,
    crate::facts::Category::Armor,
    crate::facts::Category::Clothing,
    crate::facts::Category::Jewelry,
    crate::facts::Category::Cloaks,
    crate::facts::Category::Summonons,
    crate::facts::Category::Aetheria,
    crate::facts::Category::Salvage,
    crate::facts::Category::Other,
];

/// The set row's buttons, in the concept's order.
pub const SET_BINDINGS: [&str; 20] = [
    "SetSoldier",
    "SetAdept",
    "SetArcher",
    "SetDefender",
    "SetTinker",
    "SetCrafter",
    "SetHearty",
    "SetDextrous",
    "SetWise",
    "SetSwift",
    "SetHardened",
    "SetReinforced",
    "SetInterlocking",
    "SetFlameProof",
    "SetAcidProof",
    "SetColdProof",
    "SetLightningProof",
    "SetDedication",
    "SetNone",
    "SetOther",
];

/// The set captions, same order.
pub const SET_LABELS: [&str; 20] = [
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
    "No set",
    "Other set",
];

/// Every binding the markup names, and its type.
pub const BINDINGS: &[(&str, Kind)] = &[
    ("Summary", Kind::Text),
    ("FilterLine", Kind::Text),
    ("Selected", Kind::Int),
    ("Select", Kind::IntAction),
    ("SortColumn", Kind::Int),
    ("SortDescending", Kind::Bool),
    ("Sort", Kind::IntBoolAction),
    ("Widths", Kind::FloatList),
    ("ColumnResize", Kind::FloatListAction),
    ("Icons", Kind::UintList),
    ("Names", Kind::TextList),
    ("NameColours", Kind::UintList),
    ("Types", Kind::TextList),
    ("Slots", Kind::TextList),
    ("Coverage", Kind::TextList),
    ("CreatureCantrips", Kind::TextList),
    ("ItemCantrips", Kind::TextList),
    ("LifeCantrips", Kind::TextList),
    ("Sets", Kind::TextList),
    ("Ratings", Kind::TextList),
    ("Details", Kind::TextList),
    ("Prices", Kind::TextList),
    ("PriceColours", Kind::UintList),
    ("Status", Kind::Text),
    ("Scan", Kind::Action),
    ("Reset", Kind::Action),
    ("CatWeaponsAction", Kind::BoolAction),
    ("CatArmorAction", Kind::BoolAction),
    ("CatClothingAction", Kind::BoolAction),
    ("CatJewelryAction", Kind::BoolAction),
    ("CatCloaksAction", Kind::BoolAction),
    ("CatSummononsAction", Kind::BoolAction),
    ("CatAetheriaAction", Kind::BoolAction),
    ("CatSalvageAction", Kind::BoolAction),
    ("CatOtherAction", Kind::BoolAction),
    ("WpnMissileAction", Kind::BoolAction),
    ("WpnHeavyAction", Kind::BoolAction),
    ("WpnFinesseAction", Kind::BoolAction),
    ("WpnLightAction", Kind::BoolAction),
    ("WpnShieldAction", Kind::BoolAction),
    ("SlotNecklaceAction", Kind::BoolAction),
    ("SlotBraceletAction", Kind::BoolAction),
    ("SlotRingAction", Kind::BoolAction),
    ("SlotTrinketAction", Kind::BoolAction),
    ("SlotCloakAction", Kind::BoolAction),
    ("SlotHeadAction", Kind::BoolAction),
    ("SlotChestAction", Kind::BoolAction),
    ("SlotAbdomenAction", Kind::BoolAction),
    ("SlotUpperArmsAction", Kind::BoolAction),
    ("SlotLowerArmsAction", Kind::BoolAction),
    ("SlotHandsAction", Kind::BoolAction),
    ("SlotUpperLegsAction", Kind::BoolAction),
    ("SlotLowerLegsAction", Kind::BoolAction),
    ("SlotFeetAction", Kind::BoolAction),
    ("CanCreatureAction", Kind::BoolAction),
    ("CanItemAction", Kind::BoolAction),
    ("CanLifeAction", Kind::BoolAction),
    ("CanRegularAction", Kind::BoolAction),
    ("UnenchentableAction", Kind::BoolAction),
    ("BandMinorAction", Kind::Action),
    ("BandModerateAction", Kind::Action),
    ("BandMajorAction", Kind::Action),
    ("BandEpicAction", Kind::Action),
    ("BandLegendaryAction", Kind::Action),
    ("SetSoldierAction", Kind::Action),
    ("SetAdeptAction", Kind::Action),
    ("SetArcherAction", Kind::Action),
    ("SetDefenderAction", Kind::Action),
    ("SetTinkerAction", Kind::Action),
    ("SetCrafterAction", Kind::Action),
    ("SetHeartyAction", Kind::Action),
    ("SetDextrousAction", Kind::Action),
    ("SetWiseAction", Kind::Action),
    ("SetSwiftAction", Kind::Action),
    ("SetHardenedAction", Kind::Action),
    ("SetReinforcedAction", Kind::Action),
    ("SetInterlockingAction", Kind::Action),
    ("SetFlameProofAction", Kind::Action),
    ("SetAcidProofAction", Kind::Action),
    ("SetColdProofAction", Kind::Action),
    ("SetLightningProofAction", Kind::Action),
    ("SetDedicationAction", Kind::Action),
    ("SetNoneAction", Kind::Action),
    ("SetOtherAction", Kind::Action),
    ("CatWeapons", Kind::Bool),
    ("CatArmor", Kind::Bool),
    ("CatClothing", Kind::Bool),
    ("CatJewelry", Kind::Bool),
    ("CatCloaks", Kind::Bool),
    ("CatSummonons", Kind::Bool),
    ("CatAetheria", Kind::Bool),
    ("CatSalvage", Kind::Bool),
    ("CatOther", Kind::Bool),
    ("Unenchentable", Kind::Bool),
    ("WpnMissile", Kind::Bool),
    ("WpnHeavy", Kind::Bool),
    ("WpnFinesse", Kind::Bool),
    ("WpnLight", Kind::Bool),
    ("WpnShield", Kind::Bool),
    ("SlotNecklace", Kind::Bool),
    ("SlotBracelet", Kind::Bool),
    ("SlotRing", Kind::Bool),
    ("SlotTrinket", Kind::Bool),
    ("SlotCloak", Kind::Bool),
    ("SlotHead", Kind::Bool),
    ("SlotChest", Kind::Bool),
    ("SlotAbdomen", Kind::Bool),
    ("SlotUpperArms", Kind::Bool),
    ("SlotLowerArms", Kind::Bool),
    ("SlotHands", Kind::Bool),
    ("SlotUpperLegs", Kind::Bool),
    ("SlotLowerLegs", Kind::Bool),
    ("SlotFeet", Kind::Bool),
    ("CanCreature", Kind::Bool),
    ("CanItem", Kind::Bool),
    ("CanLife", Kind::Bool),
    ("CanRegular", Kind::Bool),
    ("BandMinor", Kind::Text),
    ("BandModerate", Kind::Text),
    ("BandMajor", Kind::Text),
    ("BandEpic", Kind::Text),
    ("BandLegendary", Kind::Text),
    ("SetSoldier", Kind::Text),
    ("SetAdept", Kind::Text),
    ("SetArcher", Kind::Text),
    ("SetDefender", Kind::Text),
    ("SetTinker", Kind::Text),
    ("SetCrafter", Kind::Text),
    ("SetHearty", Kind::Text),
    ("SetDextrous", Kind::Text),
    ("SetWise", Kind::Text),
    ("SetSwift", Kind::Text),
    ("SetHardened", Kind::Text),
    ("SetReinforced", Kind::Text),
    ("SetInterlocking", Kind::Text),
    ("SetFlameProof", Kind::Text),
    ("SetAcidProof", Kind::Text),
    ("SetColdProof", Kind::Text),
    ("SetLightningProof", Kind::Text),
    ("SetDedication", Kind::Text),
    ("SetNone", Kind::Text),
    ("SetOther", Kind::Text),
];

impl Kind {
    /// The SDK's spelling, for registering the panel. The plain
    /// `BindingKind` has no `Action<bool>`; `bind` takes anything
    /// `Into<TypedBindingKind>`, which does, so the typed enum is the
    /// return type and the wire conversion happens in the SDK.
    #[must_use]
    pub fn abi(self) -> openac_plugin::ui::TypedBindingKind {
        use openac_plugin::ui::TypedBindingKind as K;
        match self {
            Self::Int => K::Int,
            Self::Text => K::Text,
            Self::Bool => K::Bool,
            Self::UintList => K::UintList,
            Self::TextList => K::TextList,
            Self::FloatList => K::FloatList,
            Self::Action => K::Action,
            Self::IntBoolAction => K::IntBoolAction,
            Self::FloatListAction => K::FloatListAction,
            Self::BoolAction => K::BoolAction,
            Self::IntAction => K::IntAction,
        }
    }
}

/// The window: the concept's filter rows above the results list.
pub const MARKUP: &str = markup();

/// Built at compile time by `const` concatenation would be nicer, but a
/// `const fn` returning `&str` cannot append; the literal is written out
/// by the same order the binding tables use, and the tests below fail if
/// the two ever drift.
const fn markup() -> &'static str {
    // Written as one literal so the parser test reads exactly what the
    // player's window is built from.
    r#"<panel w="760" h="620" theme="plugin" layout="column" padding="8" gap="4" resizable="true" minw="620" minh="420">
  <label text="{Summary}"/>
  <label text="{FilterLine}"/>
  <group layout="row" gap="6" align="center"><checkbox text="Weapons" checked="{CatWeapons}" onchange="{CatWeaponsAction}"/><checkbox text="Armor" checked="{CatArmor}" onchange="{CatArmorAction}"/><checkbox text="Clothing" checked="{CatClothing}" onchange="{CatClothingAction}"/><checkbox text="Jewelry" checked="{CatJewelry}" onchange="{CatJewelryAction}"/><checkbox text="Cloaks" checked="{CatCloaks}" onchange="{CatCloaksAction}"/><checkbox text="Summonons" checked="{CatSummonons}" onchange="{CatSummononsAction}"/><checkbox text="Aetheria" checked="{CatAetheria}" onchange="{CatAetheriaAction}"/><checkbox text="Salvage" checked="{CatSalvage}" onchange="{CatSalvageAction}"/><checkbox text="Other" checked="{CatOther}" onchange="{CatOtherAction}"/></group>
  <group layout="row" gap="6" align="center"><checkbox text="Unenchentable" checked="{Unenchentable}" onchange="{UnenchentableAction}"/></group>
  <group layout="row" gap="6" align="center"><checkbox text="Missile" checked="{WpnMissile}" onchange="{WpnMissileAction}"/><checkbox text="Heavy" checked="{WpnHeavy}" onchange="{WpnHeavyAction}"/><checkbox text="Finesse" checked="{WpnFinesse}" onchange="{WpnFinesseAction}"/><checkbox text="Light" checked="{WpnLight}" onchange="{WpnLightAction}"/><checkbox text="Shield" checked="{WpnShield}" onchange="{WpnShieldAction}"/></group>
  <group layout="row" gap="6" align="center"><checkbox text="Necklace" checked="{SlotNecklace}" onchange="{SlotNecklaceAction}"/><checkbox text="Bracelet" checked="{SlotBracelet}" onchange="{SlotBraceletAction}"/><checkbox text="Ring" checked="{SlotRing}" onchange="{SlotRingAction}"/><checkbox text="Trinket" checked="{SlotTrinket}" onchange="{SlotTrinketAction}"/><checkbox text="Cloak" checked="{SlotCloak}" onchange="{SlotCloakAction}"/><checkbox text="Head" checked="{SlotHead}" onchange="{SlotHeadAction}"/><checkbox text="Chest" checked="{SlotChest}" onchange="{SlotChestAction}"/><checkbox text="Abdomen" checked="{SlotAbdomen}" onchange="{SlotAbdomenAction}"/><checkbox text="Upper Arms" checked="{SlotUpperArms}" onchange="{SlotUpperArmsAction}"/><checkbox text="Lower Arms" checked="{SlotLowerArms}" onchange="{SlotLowerArmsAction}"/><checkbox text="Hands" checked="{SlotHands}" onchange="{SlotHandsAction}"/></group>
  <group layout="row" gap="6" align="center"><checkbox text="Upper Legs" checked="{SlotUpperLegs}" onchange="{SlotUpperLegsAction}"/><checkbox text="Lower Legs" checked="{SlotLowerLegs}" onchange="{SlotLowerLegsAction}"/><checkbox text="Feet" checked="{SlotFeet}" onchange="{SlotFeetAction}"/></group>
  <group layout="row" gap="6" align="center"><checkbox text="Creature cantrips" checked="{CanCreature}" onchange="{CanCreatureAction}"/><checkbox text="Item cantrips" checked="{CanItem}" onchange="{CanItemAction}"/><checkbox text="Life cantrips" checked="{CanLife}" onchange="{CanLifeAction}"/><checkbox text="Regular only" checked="{CanRegular}" onchange="{CanRegularAction}"/></group>
  <group layout="row" gap="4" align="center"><label text="Filter by cantrip level"/><button w="118" h="20" text="{BandMinor}" onclick="{BandMinorAction}"/><button w="118" h="20" text="{BandModerate}" onclick="{BandModerateAction}"/><button w="118" h="20" text="{BandMajor}" onclick="{BandMajorAction}"/><button w="118" h="20" text="{BandEpic}" onclick="{BandEpicAction}"/><button w="118" h="20" text="{BandLegendary}" onclick="{BandLegendaryAction}"/></group>
  <group layout="row" gap="4"><button w="70" h="20" text="{SetSoldier}" onclick="{SetSoldierAction}"/><button w="60" h="20" text="{SetAdept}" onclick="{SetAdeptAction}"/><button w="62" h="20" text="{SetArcher}" onclick="{SetArcherAction}"/><button w="72" h="20" text="{SetDefender}" onclick="{SetDefenderAction}"/><button w="58" h="20" text="{SetTinker}" onclick="{SetTinkerAction}"/><button w="62" h="20" text="{SetCrafter}" onclick="{SetCrafterAction}"/><button w="58" h="20" text="{SetHearty}" onclick="{SetHeartyAction}"/><button w="68" h="20" text="{SetDextrous}" onclick="{SetDextrousAction}"/><button w="50" h="20" text="{SetWise}" onclick="{SetWiseAction}"/><button w="50" h="20" text="{SetSwift}" onclick="{SetSwiftAction}"/><button w="72" h="20" text="{SetHardened}" onclick="{SetHardenedAction}"/></group>
  <group layout="row" gap="4"><button w="78" h="20" text="{SetReinforced}" onclick="{SetReinforcedAction}"/><button w="82" h="20" text="{SetInterlocking}" onclick="{SetInterlockingAction}"/><button w="80" h="20" text="{SetFlameProof}" onclick="{SetFlameProofAction}"/><button w="70" h="20" text="{SetAcidProof}" onclick="{SetAcidProofAction}"/><button w="72" h="20" text="{SetColdProof}" onclick="{SetColdProofAction}"/><button w="98" h="20" text="{SetLightningProof}" onclick="{SetLightningProofAction}"/><button w="76" h="20" text="{SetDedication}" onclick="{SetDedicationAction}"/><button w="56" h="20" text="{SetNone}" onclick="{SetNoneAction}"/><button w="78" h="20" text="{SetOther}" onclick="{SetOtherAction}"/></group>
  <button grow="1" h="20" text="Reset filters" onclick="{Reset}"/>
  <button grow="1" h="20" text="Deep scan" onclick="{Scan}"/>
  <list grow="1" rowheight="20" selected="{Selected}" onchange="{Select}" header="true" resizable="true" sortcolumn="{SortColumn}" sortdescending="{SortDescending}" onsort="{Sort}" widths="{Widths}" onresize="{ColumnResize}">
    <column type="icon" iconkind="did" values="{Icons}" onclick="{Select}" width="26"/>
    <column type="text" items="{Names}" colors="{NameColours}" header="Item" sort="text" width="190"/>
    <column type="text" items="{Types}" header="Type" sort="text" width="62"/>
    <column type="text" items="{Slots}" header="Slots" sort="text" width="70"/>
    <column type="text" items="{Coverage}" header="Covered" sort="text" width="90"/>
    <column type="text" items="{CreatureCantrips}" header="Creature Cantrips" sort="text" width="120"/>
    <column type="text" items="{ItemCantrips}" header="Item Cantrips" sort="text" width="110"/>
    <column type="text" items="{LifeCantrips}" header="Life Cantrips" sort="text" width="100"/>
    <column type="text" items="{Sets}" header="Item Set" sort="text" width="60"/>
    <column type="text" items="{Ratings}" header="Ratings" sort="text" width="90"/>
    <column type="text" items="{Details}" header="Detail" sort="text" width="80"/>
    <column type="text" items="{Prices}" colors="{PriceColours}" header="Price" sort="number" width="60"/>
  </list>
  <label text="{Status}"/>
</panel>"#
}

/// The list's authored column widths, in markup order. The player can
/// drag them; this is what the widths binding starts from.
pub const AUTHORED_WIDTHS: [f32; 12] = [
    26.0, 190.0, 62.0, 70.0, 90.0, 120.0, 110.0, 100.0, 60.0, 90.0, 80.0, 60.0,
];

/// What the rows hold, one entry per list column.
#[derive(Debug, Default, Clone)]
pub struct Rows {
    /// The property id of each row, so a row index finds its item.
    pub property_ids: Vec<i64>,
    pub icons: Vec<u32>,
    pub names: Vec<String>,
    pub name_colours: Vec<u32>,
    pub types: Vec<String>,
    pub slots: Vec<String>,
    pub coverage: Vec<String>,
    pub creature_cantrips: Vec<String>,
    pub item_cantrips: Vec<String>,
    pub life_cantrips: Vec<String>,
    pub sets: Vec<String>,
    pub ratings: Vec<String>,
    pub details: Vec<String>,
    pub prices: Vec<String>,
    pub price_colours: Vec<u32>,
}

impl Rows {
    /// The rows for `inventory`, in name order.
    #[must_use]
    pub fn build(inventory: &crate::inventory::Inventory) -> Self {
        let mut rows = Self::default();
        let mut ordered: Vec<&Row> = inventory.rows.iter().collect();
        ordered.sort_by_key(|row| row.name.to_lowercase());
        for row in ordered {
            // A row the appraisal has answered carries at least a slot
            // (armour and jewelry) or a category beyond Other (weapons,
            // gems); a sighting alone leaves both blank.
            let appraised = !row.slots.is_empty()
                || !row.coverage.is_empty()
                || !row.creature_cantrips.is_empty()
                || !row.item_cantrips.is_empty()
                || !row.life_cantrips.is_empty()
                || row.category != crate::facts::Category::Other;
            rows.property_ids.push(row.property_id);
            rows.icons.push(row.icon_id);
            rows.names.push(display_name(row));
            rows.name_colours.push(if appraised {
                COLOUR_HAVE
            } else {
                COLOUR_PENDING
            });
            rows.types.push(row.category.label().to_owned());
            rows.slots.push(row.slots.clone());
            rows.coverage.push(row.coverage.clone());
            rows.creature_cantrips.push(row.creature_cantrips.clone());
            rows.item_cantrips.push(row.item_cantrips.clone());
            rows.life_cantrips.push(row.life_cantrips.clone());
            rows.sets.push(row.item_set.clone());
            rows.ratings.push(row.ratings.clone());
            rows.details.push(row.detail.clone());
            rows.prices.push(row.price.clone());
            rows.price_colours.push(if row.price.is_empty() {
                COLOUR_HAVE
            } else {
                COLOUR_PRICE
            });
        }
        rows
    }

    /// Reorder every column by `column` (1 = Item; 0 is the icon column,
    /// which never sorts), descending on request. The list shows the
    /// rows as given, so the sort is the plugin's job.
    pub fn sort_by(&mut self, column: i32, descending: bool) {
        let Some(column) = usize::try_from(column).ok().filter(|c| *c >= 1) else {
            return;
        };
        // Zip every column onto the row, order by the chosen key, then
        // pull them apart again; every field moves with its row.
        let height = self.property_ids.len();
        let mut index: Vec<usize> = (0..height).collect();
        let keys: Vec<&str> = match column {
            1 => self.names.iter().map(String::as_str).collect::<Vec<&str>>(),
            2 => self.types.iter().map(String::as_str).collect::<Vec<&str>>(),
            3 => self.slots.iter().map(String::as_str).collect::<Vec<&str>>(),
            4 => self
                .coverage
                .iter()
                .map(String::as_str)
                .collect::<Vec<&str>>(),
            5 => self
                .creature_cantrips
                .iter()
                .map(String::as_str)
                .collect::<Vec<&str>>(),
            6 => self
                .item_cantrips
                .iter()
                .map(String::as_str)
                .collect::<Vec<&str>>(),
            7 => self
                .life_cantrips
                .iter()
                .map(String::as_str)
                .collect::<Vec<&str>>(),
            8 => self.sets.iter().map(String::as_str).collect::<Vec<&str>>(),
            9 => self
                .ratings
                .iter()
                .map(String::as_str)
                .collect::<Vec<&str>>(),
            10 => self
                .details
                .iter()
                .map(String::as_str)
                .collect::<Vec<&str>>(),
            11 => self
                .prices
                .iter()
                .map(String::as_str)
                .collect::<Vec<&str>>(),
            _ => return,
        };
        let numeric = column == 11;
        index.sort_by(|a, b| {
            let order = if numeric {
                number(keys[*a])
                    .partial_cmp(&number(keys[*b]))
                    .unwrap_or(std::cmp::Ordering::Equal)
            } else {
                keys[*a].to_lowercase().cmp(&keys[*b].to_lowercase())
            };
            if descending { order.reverse() } else { order }
        });
        let take = |column: &Vec<String>| {
            index
                .iter()
                .map(|i| column[*i].clone())
                .collect::<Vec<String>>()
        };
        let numbers = |column: &Vec<u32>| index.iter().map(|i| column[*i]).collect::<Vec<u32>>();
        let ids = |column: &Vec<i64>| index.iter().map(|i| column[*i]).collect::<Vec<i64>>();
        *self = Self {
            property_ids: ids(&self.property_ids),
            icons: numbers(&self.icons),
            names: take(&self.names),
            name_colours: numbers(&self.name_colours),
            types: take(&self.types),
            slots: take(&self.slots),
            coverage: take(&self.coverage),
            creature_cantrips: take(&self.creature_cantrips),
            item_cantrips: take(&self.item_cantrips),
            life_cantrips: take(&self.life_cantrips),
            sets: take(&self.sets),
            ratings: take(&self.ratings),
            details: take(&self.details),
            prices: take(&self.prices),
            price_colours: numbers(&self.price_colours),
        };
    }

    /// One row per column: the UI cannot be handed rows whose columns
    /// disagree.
    #[must_use]
    pub fn is_square(&self) -> bool {
        let height = self.property_ids.len();
        [
            self.icons.len(),
            self.names.len(),
            self.name_colours.len(),
            self.types.len(),
            self.slots.len(),
            self.coverage.len(),
            self.creature_cantrips.len(),
            self.item_cantrips.len(),
            self.life_cantrips.len(),
            self.sets.len(),
            self.ratings.len(),
            self.details.len(),
            self.prices.len(),
            self.price_colours.len(),
        ]
        .iter()
        .all(|len| *len == height)
    }
}

/// The numeric key of a price cell: digits with commas removed, up to
/// the first non-numeric word. "1,200 pts" reads as 1200; blank reads as
/// the smallest, so unpriced rows sort together.
fn number(cell: &str) -> f64 {
    let cleaned: String = cell
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == ',' || *c == '-' || *c == '.')
        .filter(|c| *c != ',')
        .collect();
    cleaned.parse().unwrap_or(f64::NEG_INFINITY)
}

/// A row's name, with its stack count when it stacks: "Artist's Crystal
/// x24", as the concept draws it.
fn display_name(row: &Row) -> String {
    if row.stack > 1 {
        format!("{} x{}", row.name, row.stack)
    } else {
        row.name.clone()
    }
}

/// The header line: whose stock, how much, how much is still unread.
#[must_use]
pub fn summary(bot: &str, total: usize, pending: usize) -> String {
    if bot.is_empty() {
        return "Trade Browse: no bot traded yet.".to_owned();
    }
    if pending == 0 {
        format!("{bot}: {total} items.")
    } else {
        format!("{bot}: {total} items, {pending} awaiting appraisal.")
    }
}

/// The band row's captions: the checked one carries a tick.
#[must_use]
pub fn band_captions(checked: Option<crate::facts::Band>) -> [String; 5] {
    use crate::facts::Band;
    [
        Band::Minor,
        Band::Moderate,
        Band::Major,
        Band::Epic,
        Band::Legendary,
    ]
    .map(|band| {
        let label = band.label().to_owned();
        if checked == Some(band) {
            format!("✓ {label}")
        } else {
            label
        }
    })
}

/// The set row's captions: the checked ones carry a tick.
#[must_use]
pub fn set_captions(sets: &Sets) -> [String; 20] {
    let mut out = SET_LABELS.map(str::to_owned);
    for (index, label) in SET_LABELS.iter().enumerate() {
        let on = match index {
            18 => sets.none,
            19 => sets.other,
            _ => sets.checked.iter().any(|set| set == label),
        };
        if on {
            out[index] = format!("✓ {label}");
        }
    }
    out
}

/// Push every filter binding: the checkbox bools and the caption texts.
pub fn set_filters(panel: &mut openac_plugin::ui::Panel, state: &FilterState) {
    use openac_plugin::ui::TypedValue;
    for (index, name) in CATEGORY_BINDINGS.iter().enumerate() {
        panel.set(
            name,
            TypedValue::Bool(state.categories.has(Categories::ALL[index])),
        );
    }
    panel.set("Unenchentable", TypedValue::Bool(state.unenchentable_only));
    for (index, name) in WEAPON_BINDINGS.iter().enumerate() {
        panel.set(name, TypedValue::Bool(state.weapons.has(index)));
    }
    for (index, name) in SLOT_BINDINGS.iter().enumerate() {
        panel.set(name, TypedValue::Bool(state.slots.has(index)));
    }
    panel.set("CanCreature", TypedValue::Bool(state.cantrips.creature));
    panel.set("CanItem", TypedValue::Bool(state.cantrips.item));
    panel.set("CanLife", TypedValue::Bool(state.cantrips.life));
    panel.set("CanRegular", TypedValue::Bool(state.cantrips.regular_only));
    for (band, name) in band_captions(state.bands.checked).into_iter().zip([
        "BandMinor",
        "BandModerate",
        "BandMajor",
        "BandEpic",
        "BandLegendary",
    ]) {
        panel.set(name, band);
    }
    for (caption, name) in set_captions(&state.sets).into_iter().zip(SET_BINDINGS) {
        panel.set(name, caption);
    }
    panel.set("FilterLine", state.describe());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filters::{SLOT_CHOICES, WEAPON_CHOICES};
    use crate::inventory::Inventory;

    #[test]
    fn the_window_markup_parses_with_the_declared_bindings() {
        use ac_plugin_ui::BindingKind as K;
        use ac_plugin_ui::BindingSchema;

        let ui_kind = |kind: Kind| match kind {
            Kind::Int => K::Int,
            Kind::Text => K::Text,
            Kind::Bool => K::Bool,
            Kind::UintList => K::UintList,
            Kind::TextList => K::TextList,
            Kind::FloatList => K::FloatList,
            Kind::Action => K::Action,
            Kind::IntBoolAction => K::IntBoolAction,
            Kind::FloatListAction => K::FloatListAction,
            Kind::BoolAction => K::BoolAction,
            Kind::IntAction => K::IntAction,
        };

        let mut schema = BindingSchema::new("TradeBrowse");
        for (name, kind) in BINDINGS {
            schema.declare(*name, ui_kind(*kind));
        }
        if let Err(error) = ac_plugin_ui::markup::parse(MARKUP, &schema) {
            panic!("the window's markup does not parse: {}", error.0);
        }
    }

    #[test]
    fn every_binding_is_used_by_the_markup() {
        for (name, _) in BINDINGS {
            assert!(
                MARKUP.contains(&format!("{{{name}}}")),
                "{name} is declared but the markup never names it"
            );
        }
    }

    #[test]
    fn the_binding_tables_match_their_markup_order() {
        // The handlers index these tables; the markup lists the same
        // bindings in the same order, and a drift would toggle the wrong
        // box.
        assert_eq!(CATEGORY_BINDINGS.len(), Categories::ALL.len());
        assert_eq!(WEAPON_BINDINGS.len(), WEAPON_CHOICES.len());
        assert_eq!(SLOT_BINDINGS.len(), SLOT_CHOICES.len());
        assert_eq!(SET_BINDINGS.len(), SET_LABELS.len());
        assert_eq!(SET_BINDINGS.len(), 20, "18 sets plus none and other");
    }

    #[test]
    fn rows_sort_by_name_and_stay_square() {
        let mut inv = Inventory::default();
        inv.rows.push(Row {
            property_id: -1,
            name: "Zephyr Bow".to_owned(),
            ..Default::default()
        });
        inv.rows.push(Row {
            property_id: -2,
            name: "acid greataxe".to_owned(),
            ..Default::default()
        });
        let rows = Rows::build(&inv);
        assert!(rows.is_square());
        assert_eq!(rows.names[0], "acid greataxe", "case-insensitive");
        assert_eq!(rows.names[1], "Zephyr Bow");
    }

    #[test]
    fn a_stack_shows_its_count() {
        let mut inv = Inventory::default();
        inv.rows.push(Row {
            property_id: -3,
            name: "Artist's Crystal".to_owned(),
            stack: 24,
            ..Default::default()
        });
        let rows = Rows::build(&inv);
        assert_eq!(rows.names[0], "Artist's Crystal x24");
    }

    #[test]
    fn the_summary_counts_what_is_still_unread() {
        let text = summary("Mortarian", 200, 49);
        assert!(text.contains("Mortarian"), "{text}");
        assert!(text.contains("49 awaiting"), "{text}");
        assert!(summary("Mortarian", 200, 0).contains("200 items"));
        assert!(summary("", 0, 0).contains("no bot"));
    }

    #[test]
    fn a_checked_band_ticks_its_caption() {
        let captions = band_captions(Some(crate::facts::Band::Legendary));
        assert!(captions[4].starts_with('✓'));
        assert!(!captions[0].starts_with('✓'));
        assert!(band_captions(None).iter().all(|c| !c.starts_with('✓')));
    }

    #[test]
    fn a_checked_set_ticks_its_caption() {
        let sets = Sets {
            checked: vec!["Adept".to_owned()],
            none: true,
            ..Default::default()
        };
        let captions = set_captions(&sets);
        assert!(captions[1].starts_with('✓'), "Adept");
        assert!(captions[18].starts_with('✓'), "No set");
        assert!(!captions[0].starts_with('✓'), "Soldier unchecked");
    }
}
