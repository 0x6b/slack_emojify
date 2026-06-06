#[cfg(test)]
mod tests {
    use slack_emojify::{Demojify, Emojify};

    #[test]
    fn test_emojify() {
        assert_eq!(
            ":hiking_boot: :anger::canned_food: :wavy_dash: :motorway: I kicked the can down the road on my other two in-progress tasks.".emojify(),
            "🥾 💢🥫 〰 🛣 I kicked the can down the road on my other two in-progress tasks."
        );
    }

    #[test]
    fn test_demojify() {
        assert_eq!(
            "🥾 💢🥫 〰 🛣 I kicked the can down the road on my other two in-progress tasks."
                .demojify(),
            ":hiking_boot: :anger::canned_food: :wavy_dash: :motorway: I kicked the can down the road on my other two in-progress tasks."
        );
    }

    #[test]
    fn test_demojify_variation_selector() {
        // the table stores emoji without U+FE0F; input with it converts the same
        assert_eq!("✈️ and ✈".demojify(), ":airplane: and :airplane:");
    }

    #[test]
    fn test_demojify_canonical_alias() {
        // 👍 has aliases :+1: and :thumbsup:; the canonical short_name wins
        assert_eq!("👍".demojify(), ":+1:");
    }

    #[test]
    fn test_demojify_passthrough() {
        // keycap components (#, *, digits) and unmapped text are left as-is
        assert_eq!("plain text, #2 *star*".demojify(), "plain text, #2 *star*");
    }

    #[test]
    fn test_round_trip() {
        let s = ":hiking_boot: :anger: :+1:";
        assert_eq!(s.emojify().demojify(), s);
    }
}
