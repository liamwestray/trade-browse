//! Asking a bot for a price over `/tell`, and hearing its answer.
//!
//! The query is keyed by *property id* — the signed weenie class players
//! call the item's property — because that is the number trade bots match
//! on: `check -20601` asks for the item whose id is -20601, and a bot that
//! has it answers with its price. The reply arrives as an ordinary tell
//! (`ChatLine::kind == 3`) from the bot, so the plugin listens, matches
//! the id back to the row it asked about, and reports the price.
//!
//! Bots differ in wording, so the query verb and the reply shapes live in
//! one table. Everything here is pure — the tell is *rendered* to a string
//! and a reply line is *parsed* from one — so the matching rules are
//! testable without a client.

/// One bot's dialect: how to ask, and how to read the answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Dialect {
    /// The verb the bot answers a price query with.
    pub verb: &'static str,
}

/// The dialect assumed until a bot proves otherwise. SkunkTrader-family
/// bots answer `check <property-id>`; the reply names the item and its
/// cost.
pub const DEFAULT: Dialect = Dialect { verb: "check" };

/// The `/tell` line that asks `bot` for the price of `property_id`.
#[must_use]
pub fn query(dialect: &Dialect, bot: &str, property_id: i64) -> String {
    format!("/tell {bot} {} {property_id}", dialect.verb)
}

/// What a reply line said, once read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reply {
    /// The bot quoted this price for this property id.
    Price { property_id: i64, price: String },
    /// The bot says it has no such item.
    Unknown { property_id: i64 },
    /// A tell from the bot that is not about a price.
    Other,
}

/// Read one tell from `bot` about `asked` (the id we queried).
///
/// Bots answer in loose prose — "Acid Greataxe: 1200 pts", "I don't have
/// that" — so the rule is: find the property id we asked about in the
/// line, then the first number that is not that id is the price. A line
/// that names the id and no other number is a refusal.
#[must_use]
pub fn parse_reply(text: &str, asked: i64) -> Reply {
    let numbers = numbers(text);
    if !numbers.contains(&asked) {
        // Not an answer to our query (bots answer many players at once).
        return Reply::Other;
    }
    match numbers.iter().find(|n| **n != asked) {
        Some(price) => Reply::Price {
            property_id: asked,
            price: price.to_string(),
        },
        None => Reply::Unknown { property_id: asked },
    }
}

/// Every signed number in a line, in order.
///
/// Bots wrap ids and prices in prose punctuation — "(-20601):", "1,200
/// pts" — so each word is scanned for numeric runs: a `-` directly before
/// a digit starts a negative number, and digit runs (with commas between
/// digits, so "1,200" reads whole) are the positives.
fn numbers(text: &str) -> Vec<i64> {
    let chars: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut index = 0;
    while index < chars.len() {
        let c = chars[index];
        let negative = c == '-' && index + 1 < chars.len() && chars[index + 1].is_ascii_digit();
        let start = if negative { index + 1 } else { index };
        if !chars[start].is_ascii_digit() {
            index += 1;
            continue;
        }
        let mut digits = String::new();
        let mut end = start;
        while end < chars.len() {
            let d = chars[end];
            if d.is_ascii_digit() {
                digits.push(d);
                end += 1;
            } else if d == ',' && end + 1 < chars.len() && chars[end + 1].is_ascii_digit() {
                // A digit-group comma inside a number: "1,200". A comma
                // between two numbers ("costs 5, 10 each") reads as one
                // number and the price parser's caller tolerates it: the
                // first non-id number wins, and a bot that quotes two
                // numbers in one line is outside the dialect table.
                end += 1;
            } else {
                break;
            }
        }
        let value = digits.parse::<i64>().unwrap_or(0);
        out.push(if negative { -value } else { value });
        index = end;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_query_carries_the_property_id() {
        assert_eq!(
            query(&DEFAULT, "CyTrader", -20601),
            "/tell CyTrader check -20601"
        );
    }

    #[test]
    fn a_price_reply_names_the_id_then_the_cost() {
        let reply = parse_reply("Acid Greataxe (-20601): 1200 pts", -20601);
        assert_eq!(
            reply,
            Reply::Price {
                property_id: -20601,
                price: "1200".to_owned()
            }
        );
    }

    #[test]
    fn a_refusal_names_only_the_id() {
        assert_eq!(
            parse_reply("I have nothing with property -20601", -20601),
            Reply::Unknown {
                property_id: -20601
            }
        );
    }

    #[test]
    fn an_answer_about_another_item_is_not_ours() {
        // The bot is answering someone else's query in the same breath.
        assert_eq!(
            parse_reply("Steel Salvage (-100): 5 pts", -20601),
            Reply::Other
        );
    }

    #[test]
    fn a_plain_number_is_read_as_signed() {
        let reply = parse_reply("-20601 costs -500", -20601);
        assert_eq!(
            reply,
            Reply::Price {
                property_id: -20601,
                price: "-500".to_owned()
            },
            "a negative price is odd but is what the bot said"
        );
    }

    #[test]
    fn a_grouped_price_reads_as_one_number() {
        let reply = parse_reply("Acid Greataxe (-20601): 1,200 pts", -20601);
        assert_eq!(
            reply,
            Reply::Price {
                property_id: -20601,
                price: "1200".to_owned()
            }
        );
    }
}
