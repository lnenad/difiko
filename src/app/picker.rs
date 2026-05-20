use super::TextInput;

#[derive(Debug, Clone, Default)]
pub struct Picker {
    pub query: TextInput,
    pub items: Vec<String>,
    pub filtered: Vec<usize>,
    pub selected: usize,
}

impl Picker {
    pub fn new(items: Vec<String>) -> Self {
        let mut p = Picker {
            query: TextInput::new(""),
            items,
            filtered: Vec::new(),
            selected: 0,
        };
        p.refilter();
        p
    }

    /// Build a picker with the cursor positioned on `current`, when present.
    /// Used for branch pickers so re-opening them lands on the current value
    /// instead of the top of the list.
    pub fn with_selected(items: Vec<String>, current: Option<&str>) -> Self {
        let mut p = Self::new(items);
        if let Some(c) = current {
            if let Some(item_idx) = p.items.iter().position(|s| s == c) {
                if let Some(pos) = p.filtered.iter().position(|&i| i == item_idx) {
                    p.selected = pos;
                }
            }
        }
        p
    }

    pub fn refilter(&mut self) {
        let q = self.query.buffer.as_str();
        if q.is_empty() {
            self.filtered = (0..self.items.len()).collect();
        } else {
            use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
            use nucleo_matcher::{Matcher, Utf32Str};
            let mut matcher = Matcher::new(nucleo_matcher::Config::DEFAULT);
            let pattern = Pattern::parse(q, CaseMatching::Smart, Normalization::Smart);
            let mut buf: Vec<char> = Vec::new();
            let mut scored: Vec<(u32, usize)> = self
                .items
                .iter()
                .enumerate()
                .filter_map(|(i, s)| {
                    let haystack = Utf32Str::new(s, &mut buf);
                    pattern
                        .score(haystack, &mut matcher)
                        .map(|score| (score, i))
                })
                .collect();
            scored.sort_by_key(|b| std::cmp::Reverse(b.0));
            self.filtered = scored.into_iter().map(|(_, i)| i).collect();
        }
        if self.selected >= self.filtered.len() {
            self.selected = self.filtered.len().saturating_sub(1);
        }
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn move_down(&mut self) {
        if self.selected + 1 < self.filtered.len() {
            self.selected += 1;
        }
    }

    pub fn current(&self) -> Option<&String> {
        self.filtered
            .get(self.selected)
            .and_then(|i| self.items.get(*i))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picker_filters_and_selects() {
        let mut p = Picker::new(vec!["foo.rs".into(), "bar.rs".into(), "baz.txt".into()]);
        assert_eq!(p.filtered.len(), 3);
        for c in "ba".chars() {
            p.query.insert(c);
        }
        p.refilter();
        // "bar.rs" and "baz.txt" both match "ba"; "foo.rs" should not.
        assert_eq!(p.filtered.len(), 2);
        let cur = p.current().unwrap().clone();
        assert!(cur.starts_with("ba"));
        p.move_down();
        let cur2 = p.current().unwrap().clone();
        assert!(cur2.starts_with("ba"));
        assert_ne!(cur, cur2);
    }

    #[test]
    fn picker_with_selected_lands_on_current() {
        let p = Picker::with_selected(
            vec!["main".into(), "develop".into(), "feat/x".into()],
            Some("feat/x"),
        );
        assert_eq!(p.current().map(String::as_str), Some("feat/x"));
    }

    #[test]
    fn picker_with_selected_falls_back_when_missing() {
        let p = Picker::with_selected(vec!["main".into(), "develop".into()], Some("not-in-list"));
        // Falls back to first item, doesn't panic.
        assert_eq!(p.current().map(String::as_str), Some("main"));
    }

    #[test]
    fn picker_no_match_clears_selection() {
        let mut p = Picker::new(vec!["foo.rs".into()]);
        for c in "zzzz".chars() {
            p.query.insert(c);
        }
        p.refilter();
        assert!(p.filtered.is_empty());
        assert!(p.current().is_none());
    }
}
