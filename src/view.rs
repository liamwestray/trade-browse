//! The window's markup and the rows it shows.
//!
//! The concept's wall of checkboxes becomes a tab row — Category, Slot &
//! Coverage, Cantrips, Set — so the window stays usable at 700×480; the
//! filter tabs themselves arrive with M3, and M0 draws the list they will
//! scope. One column per fact, an icon column for the item's real art
//! (`iconkind="did"`, the pattern proven in spell-research), and a status
//! line that names what the last scan did.

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
    UintList,
    TextList,
    /// A click.
    Action,
    /// A list row clicked.
    IntAction,
}

/// Every binding the markup names, and its type.
pub const BINDINGS: &[(&str, Kind)] = &[
    ("Summary", Kind::Text),
    ("Selected", Kind::Int),
    ("Select", Kind::IntAction),
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
    ("Prices", Kind::TextList),
    ("PriceColours", Kind::UintList),
    ("Status", Kind::Text),
    ("Scan", Kind::Action),
    ("Reset", Kind::Action),
];

impl Kind {
    /// The SDK's spelling, for registering the panel.
    #[must_use]
    pub fn abi(self) -> openac_plugin::ui::BindingKind {
        use openac_plugin::ui::BindingKind as K;
        match self {
            Self::Int => K::Int,
            Self::Text => K::Text,
            Self::UintList => K::UintList,
            Self::TextList => K::TextList,
            Self::Action => K::Action,
            Self::IntAction => K::IntAction,
        }
    }
}

/// The window.
pub const MARKUP: &str = r#"<panel w="700" h="480">
  <label x="10" y="8" text="{Summary}"/>
  <list x="10" y="28" w="680" h="392" rowheight="24" selected="{Selected}" onchange="{Select}">
    <column type="icon" iconkind="did" values="{Icons}" onclick="{Select}" width="30"/>
    <column type="text" items="{Names}" colors="{NameColours}" header="Item" sort="text" width="170"/>
    <column type="text" items="{Types}" header="Type" width="70"/>
    <column type="text" items="{Slots}" header="Slots" width="60"/>
    <column type="text" items="{Coverage}" header="Covered" width="60"/>
    <column type="text" items="{CreatureCantrips}" header="Creature" width="70"/>
    <column type="text" items="{ItemCantrips}" header="Item" width="70"/>
    <column type="text" items="{LifeCantrips}" header="Life" width="60"/>
    <column type="text" items="{Sets}" header="Set" width="60"/>
    <column type="text" items="{Prices}" colors="{PriceColours}" header="Price" sort="number" width="60"/>
  </list>
  <label x="10" y="426" w="680" h="18" text="{Status}"/>
  <button x="10" y="448" w="90" h="24" text="Deep scan" onclick="{Scan}"/>
  <button x="106" y="448" w="90" h="24" text="Reset filters" onclick="{Reset}"/>
</panel>"#;

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
            // A row with no cantrips and no coverage recorded is one the
            // appraisal has not answered yet: it draws greyed.
            let appraised = !row.valid_locations.is_empty() || !row.coverage.is_empty();
            rows.property_ids.push(row.property_id);
            rows.icons.push(0);
            rows.names.push(display_name(row));
            rows.name_colours.push(if appraised {
                COLOUR_HAVE
            } else {
                COLOUR_PENDING
            });
            rows.types.push(row.category.label().to_owned());
            rows.slots.push(row.valid_locations.clone());
            rows.coverage.push(row.coverage.clone());
            rows.creature_cantrips.push(row.creature_cantrips.clone());
            rows.item_cantrips.push(row.item_cantrips.clone());
            rows.life_cantrips.push(row.life_cantrips.clone());
            rows.sets.push(row.item_set.clone());
            rows.prices.push(row.price.clone());
            rows.price_colours.push(if row.price.is_empty() {
                COLOUR_HAVE
            } else {
                COLOUR_PRICE
            });
        }
        rows
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
            self.prices.len(),
            self.price_colours.len(),
        ]
        .iter()
        .all(|len| *len == height)
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inventory::Inventory;

    #[test]
    fn the_window_markup_parses_with_the_declared_bindings() {
        use ac_plugin_ui::BindingKind as K;
        use ac_plugin_ui::BindingSchema;

        let ui_kind = |kind: Kind| match kind {
            Kind::Int => K::Int,
            Kind::Text => K::Text,
            Kind::UintList => K::UintList,
            Kind::TextList => K::TextList,
            Kind::Action => K::Action,
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
    fn rows_sort_by_name_and_stay_square() {
        let mut inv = Inventory::default();
        inv.rows.push(crate::inventory::Row {
            property_id: -1,
            name: "Zephyr Bow".to_owned(),
            ..Default::default()
        });
        inv.rows.push(crate::inventory::Row {
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
        inv.rows.push(crate::inventory::Row {
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
        let text = summary("CyTrader", 200, 195);
        assert!(text.contains("CyTrader"), "{text}");
        assert!(text.contains("195 awaiting"), "{text}");
        assert!(summary("CyTrader", 200, 0).contains("200 items"));
        assert!(summary("", 0, 0).contains("no bot"));
    }
}
