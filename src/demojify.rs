use std::{collections::BTreeMap, sync::LazyLock};

use serde_json::from_slice;

static TABLE: LazyLock<BTreeMap<char, &str>> = LazyLock::new(|| {
    from_slice::<BTreeMap<&str, &str>>(include_bytes!("../assets/emoji_reverse.json"))
        .unwrap()
        .into_iter()
        .map(|(emoji, name)| (emoji.chars().next().unwrap(), name))
        .collect()
});

const VARIATION_SELECTOR_16: char = '\u{FE0F}';

pub trait Demojify {
    fn demojify(&self) -> String;
}

impl<T> Demojify for T
where
    T: AsRef<str>,
{
    fn demojify(&self) -> String {
        let s = self.as_ref();
        let mut new_text = String::with_capacity(s.len());
        let mut chars = s.chars().peekable();

        while let Some(c) = chars.next() {
            match TABLE.get(&c) {
                Some(name) => {
                    new_text.push_str(name);
                    // the table stores emoji without the variation selector; consume one
                    // following the emoji in the input
                    if chars.peek() == Some(&VARIATION_SELECTOR_16) {
                        chars.next();
                    }
                }
                None => new_text.push(c),
            }
        }
        new_text
    }
}
