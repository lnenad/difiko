use crate::app::App;
use crate::model::{DiffLine, FileChange};
use crate::ui::{syntax, word_diff};
use std::collections::HashMap;

/// Word ranges keyed by `diff_lines` index. Both old- and new-side ranges
/// land here — each is byte-offset into its own line's text.
pub type WordPairings = HashMap<usize, Vec<(usize, usize)>>;

/// Pair adjacent runs of Del/Add lines and compute word-level changed
/// byte ranges within each line. Position-paired: the k-th Del in a run
/// pairs with the k-th Add. Unpaired lines (excess on either side) get
/// no entry — the renderer treats them as fully changed.
pub fn compute_word_pairings(diff_lines: &[DiffLine]) -> WordPairings {
    let mut out: WordPairings = HashMap::new();
    let mut i = 0;
    while i < diff_lines.len() {
        let mut dels: Vec<(usize, String)> = Vec::new();
        let mut adds: Vec<(usize, String)> = Vec::new();
        let start = i;
        while i < diff_lines.len() {
            match &diff_lines[i] {
                DiffLine::Del(t) => dels.push((i, t.clone())),
                DiffLine::Add(t) => adds.push((i, t.clone())),
                _ => break,
            }
            i += 1;
        }
        if i == start {
            i += 1;
            continue;
        }
        let n = dels.len().min(adds.len());
        for k in 0..n {
            let (di, dt) = &dels[k];
            let (ai, at) = &adds[k];
            let (or, nr) = word_diff::word_ranges(dt, at);
            out.insert(*di, or);
            out.insert(*ai, nr);
        }
    }
    out
}

/// Make sure `app.syntax_cache` has an entry for `file.path` when syntax
/// highlighting is enabled. Cheap no-op when already cached or disabled.
pub fn ensure_syntax_cache(app: &App, file: &FileChange) {
    if !app.config.syntax_highlight {
        return;
    }
    if app.syntax_cache.borrow().contains_key(&file.path) {
        return;
    }
    let highlights = syntax::highlight_file(file);
    app.syntax_cache
        .borrow_mut()
        .insert(file.path.clone(), highlights);
}
