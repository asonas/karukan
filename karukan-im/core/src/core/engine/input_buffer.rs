//! InputBuffer: a recorded element array plus a caret, with every view
//! derived by evaluation.
//!
//! **The record** is the single source of truth: one element per display
//! character plus `cursor`, the caret as an element index. Typing `kyo` records
//! `[Romaji(k), Romaji(y), Romaji(o)]`, which evaluation re-records as
//! `[Converted(き), Converted(ょ)]` — elements and displayed characters
//! always correspond one to one, so the record can never disagree with
//! what is shown, and the caret is simply an index into both.
//!
//! - [`Element::Romaji`]: one keystroke not yet consumed by a rule (`y`,
//!   `k`, a lone `n`). Shown verbatim; evaluation may later consume it.
//! - [`Element::Converted`]: one settled character — a fired rule's kana,
//!   a passthrough like `1`, or direct input (alphabet/emoji mode). Opaque
//!   to evaluation; it never reverts.
//!
//! **Evaluation** derives everything else: the display, the conversion
//! reading, and the aux romaji tail. After a romaji keystroke is recorded,
//! the Romaji run ending at the cursor is evaluated through the converter:
//! keystrokes a rule consumed are re-recorded as its output. Elements
//! right of the cursor are never touched, so nothing combines across the
//! caret, and the caret moves without settling anything — `[Romaji(k),
//! Romaji(y), Converted(K)]` plus `o` typed before the `K` evaluates to
//! 「きょK」.
//!
//! Every record edit ends with an evaluation. Typing evaluates the run
//! ending at the caret; backspace/delete remove exactly one element and
//! then evaluate the run the removal joined, so the result always equals
//! typing the remaining keystrokes fresh: removing こ from `ytko`
//! re-exposes the live elements (`o` → 「yと」, again 「よ」), and
//! removing the `1` from `yt1t` evaluates `ytt` → 「yっt」.

use karukan_engine::RomajiConverter;

/// One display character of the composition.
#[derive(Clone)]
enum Element {
    /// A keystroke not yet consumed by a conversion rule
    Romaji(char),
    /// A settled character: fired rule output (`ko` → こ), passthrough
    /// (`1`), or direct input — excluded from romaji evaluation
    Converted {
        ch: char,
        /// The keystrokes that produced this character, kept so F9/F10 can
        /// commit what was typed without a lossy kana→romaji reverse
        /// conversion. A rule's keystrokes are attributed to its first
        /// output character, so the later characters of a multi-kana rule
        /// (`kya` → きゃ) carry none and the buffer's keystrokes still
        /// concatenate in order.
        raw: String,
    },
}

impl Element {
    fn converted(ch: char, raw: impl Into<String>) -> Self {
        Element::Converted {
            ch,
            raw: raw.into(),
        }
    }

    fn ch(&self) -> char {
        match self {
            Element::Romaji(ch) | Element::Converted { ch, .. } => *ch,
        }
    }

    fn is_romaji(&self) -> bool {
        matches!(self, Element::Romaji(_))
    }
}

/// The recorded composition: elements plus the caret index.
pub(super) struct InputBuffer {
    elements: Vec<Element>,
    /// Caret: a boundary index into `elements`, which — with one element
    /// per display character — is also the display position.
    ///
    /// ```text
    /// elements: [Romaji(k), Romaji(y), Converted(1), Converted(K)]
    /// boundary: 0         1          2             3             4
    ///                                ↑ cursor = 2 (between y and 1)
    /// ```
    cursor: usize,
}

impl InputBuffer {
    pub fn new() -> Self {
        Self {
            elements: Vec::new(),
            cursor: 0,
        }
    }

    pub fn clear(&mut self) {
        self.elements.clear();
        self.cursor = 0;
    }

    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    // --- Record edits -----------------------------------------------------

    /// Record a kana-mode keystroke at the caret, then evaluate the active
    /// run it now ends.
    pub fn push_romaji(&mut self, ch: char, romaji: &RomajiConverter) {
        self.elements
            .insert(self.cursor, Element::Romaji(ch.to_ascii_lowercase()));
        self.cursor += 1;
        self.evaluate_active_run(romaji);
    }

    /// Record a direct-input keystroke (alphabet/emoji mode) at the caret,
    /// settled as-is.
    pub fn push_direct(&mut self, ch: char) {
        self.push_direct_raw(ch, ch.to_string());
    }

    /// Record a settled character whose keystroke differs from what is
    /// displayed — Ctrl+Space shows `　` but was typed as a space, so F10
    /// commits the half-width form.
    pub fn push_direct_raw(&mut self, ch: char, raw: impl Into<String>) {
        self.elements
            .insert(self.cursor, Element::converted(ch, raw));
        self.cursor += 1;
    }

    /// Record settled text at the caret. Test setup only — production
    /// code always goes through the typed-key paths.
    #[cfg(test)]
    pub fn insert(&mut self, text: &str) {
        let count = text.chars().count();
        self.elements.splice(
            self.cursor..self.cursor,
            text.chars()
                .map(|ch| Element::converted(ch, ch.to_string())),
        );
        self.cursor += count;
    }

    /// Remove the element before the caret, then evaluate the Romaji run
    /// the removal joined, so the result matches typing the remaining
    /// keystrokes fresh (`yt1t` minus the `1` → 「yっt」). Returns false
    /// when the caret is at the start.
    pub fn backspace(&mut self, romaji: &RomajiConverter) -> bool {
        if self.cursor == 0 {
            return false;
        }
        self.cursor -= 1;
        self.elements.remove(self.cursor);
        self.evaluate_joined_run(romaji);
        true
    }

    /// Remove the element at the caret (delete key), then evaluate the
    /// Romaji run the removal joined. Returns false when the caret is at
    /// the end.
    pub fn delete_at_cursor(&mut self, romaji: &RomajiConverter) -> bool {
        if self.cursor == self.elements.len() {
            return false;
        }
        self.elements.remove(self.cursor);
        self.evaluate_joined_run(romaji);
        true
    }

    /// Evaluate the active run (the Romaji run ending at the cursor),
    /// re-recording keystrokes a rule consumed as its output. Typing never
    /// combines across the caret, so this stops there.
    fn evaluate_active_run(&mut self, romaji: &RomajiConverter) {
        let range = self.active_run();
        let evaluated_len = self.evaluate_range(range.clone(), romaji);
        self.cursor = range.start + evaluated_len;
    }

    /// Evaluate the Romaji run containing the caret — both sides of a
    /// deletion point. The caret keeps its offset from the run start,
    /// clamped to the evaluated length.
    fn evaluate_joined_run(&mut self, romaji: &RomajiConverter) {
        let start = self.elements[..self.cursor]
            .iter()
            .rposition(|e| !e.is_romaji())
            .map(|i| i + 1)
            .unwrap_or(0);
        let end = self.cursor
            + self.elements[self.cursor..]
                .iter()
                .position(|e| !e.is_romaji())
                .unwrap_or(self.elements.len() - self.cursor);
        let offset = self.cursor - start;
        let evaluated_len = self.evaluate_range(start..end, romaji);
        self.cursor = start + offset.min(evaluated_len);
    }

    /// Replace a Romaji range with its evaluation; returns the new length.
    fn evaluate_range(&mut self, range: std::ops::Range<usize>, romaji: &RomajiConverter) -> usize {
        if range.is_empty() {
            return 0;
        }
        let run: String = self.elements[range.clone()]
            .iter()
            .map(Element::ch)
            .collect();
        let evaluated = evaluate_run(&run, romaji);
        let len = evaluated.len();
        self.elements.splice(range, evaluated);
        len
    }

    /// The reading as it would settle: Romaji runs force-converted in
    /// place, everything else as displayed. The non-destructive
    /// counterpart of [`Self::settle_romaji`] — used when the composition
    /// must stay editable (starting a conversion that Escape can undo).
    pub fn settled_reading(&self, romaji: &RomajiConverter) -> String {
        let mut reading = String::new();
        let mut run = String::new();
        for element in &self.elements {
            match element {
                Element::Romaji(ch) => run.push(*ch),
                Element::Converted { ch, .. } => {
                    if !run.is_empty() {
                        reading.push_str(&romaji.convert_flush(&run));
                        run.clear();
                    }
                    reading.push(*ch);
                }
            }
        }
        if !run.is_empty() {
            reading.push_str(&romaji.convert_flush(&run));
        }
        reading
    }

    /// Settle all Romaji keystrokes in place (`ltu` → っ; unmatched
    /// consonants pass through literally). Called before conversion,
    /// commit, and katakana baking. The caret keeps its distance from the
    /// end, so an end-of-composition caret stays at the end.
    pub fn settle_romaji(&mut self, romaji: &RomajiConverter) {
        if !self.elements.iter().any(Element::is_romaji) {
            return;
        }
        let from_end = self.elements.len() - self.cursor;
        let mut settled: Vec<Element> = Vec::with_capacity(self.elements.len());
        let mut run = String::new();
        for element in self.elements.drain(..) {
            match element {
                Element::Romaji(ch) => run.push(ch),
                other => {
                    flush_run(&mut settled, &mut run, romaji);
                    settled.push(other);
                }
            }
        }
        flush_run(&mut settled, &mut run, romaji);
        self.elements = settled;
        self.cursor = self.elements.len().saturating_sub(from_end);
    }

    /// Convert every settled element to katakana permanently. Called when
    /// leaving katakana mode so the preedit doesn't revert.
    pub fn bake_katakana(&mut self) {
        for element in &mut self.elements {
            if let Element::Converted { ch, .. } = element {
                let katakana = karukan_engine::hiragana_to_katakana(&ch.to_string());
                *ch = katakana.chars().next().unwrap_or(*ch);
            }
        }
    }

    /// Move the caret to a display position (also its element index).
    pub fn set_cursor(&mut self, pos: usize) {
        self.cursor = pos.min(self.elements.len());
    }

    // --- Evaluation: views derived from the record ------------------------

    /// Display caret position (== the element index of the caret).
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// Full composition display.
    pub fn display(&self) -> String {
        self.elements.iter().map(Element::ch).collect()
    }

    pub fn char_count(&self) -> usize {
        self.elements.len()
    }

    /// The keystrokes that produced the composition, in buffer order —
    /// what F9/F10 commit. Reconstructed from the record rather than by
    /// reverse-converting the kana, so cursor movement and mid-buffer
    /// edits stay faithful (`aiueo`, caret back two, `ka` → `aiukaeo`).
    pub fn raw_input(&self) -> String {
        let mut raw = String::new();
        for element in &self.elements {
            match element {
                Element::Romaji(ch) => raw.push(*ch),
                Element::Converted { raw: keys, .. } => raw.push_str(keys),
            }
        }
        raw
    }

    /// Element indices of the active run: the maximal Romaji run ending at
    /// the cursor — the keystrokes currently being typed. Empty when the
    /// element left of the cursor is settled (a stranded consonant elsewhere
    /// is NOT active; it stays part of the reading at its position).
    fn active_run(&self) -> std::ops::Range<usize> {
        let start = self.elements[..self.cursor]
            .iter()
            .rposition(|e| !e.is_romaji())
            .map(|i| i + 1)
            .unwrap_or(0);
        start..self.cursor
    }

    /// Keystrokes of the active run (shown as the aux romaji tail).
    pub fn pending(&self) -> String {
        self.elements[self.active_run()]
            .iter()
            .map(Element::ch)
            .collect()
    }

    /// Conversion reading: everything except the active run. A Romaji
    /// keystroke stranded away from the caret counts as a literal
    /// character at its position, so `y1` + `ka` reads 「y1か」.
    pub fn reading(&self) -> String {
        let active = self.active_run();
        self.elements
            .iter()
            .enumerate()
            .filter(|(i, _)| !active.contains(i))
            .map(|(_, e)| e.ch())
            .collect()
    }

    /// Caret position within [`Self::reading`]. The active run sits just
    /// before the cursor and is excluded from the reading, so this is the
    /// caret minus the active run's length.
    pub fn reading_cursor(&self) -> usize {
        self.cursor - self.active_run().len()
    }
}

/// Settle one Romaji run into `out` and clear it.
fn flush_run(out: &mut Vec<Element>, run: &mut String, romaji: &RomajiConverter) {
    if run.is_empty() {
        return;
    }
    let settled = romaji.convert_flush(run);
    out.extend(
        attribute_keys(&settled, run)
            .into_iter()
            .map(|(ch, raw)| Element::converted(romaji.width().apply(ch), raw)),
    );
    run.clear();
}

/// Attribute `keys` to the characters of `text`. An ASCII output character
/// passed through the converter unchanged, so it owns exactly its own
/// keystroke; a run of rule outputs owns the keystrokes between its ASCII
/// neighbours, all attributed to the run's first character because a
/// multi-kana rule (`kya` → きゃ) cannot be split any further. The
/// attributions therefore concatenate back to `keys`.
fn attribute_keys(text: &str, keys: &str) -> Vec<(char, String)> {
    let keys: Vec<char> = keys.chars().collect();
    let text: Vec<char> = text.chars().collect();
    let mut out = Vec::with_capacity(text.len());
    let mut pos = 0;
    let mut i = 0;
    while i < text.len() {
        if text[i].is_ascii() {
            out.push((text[i], text[i].to_string()));
            pos += 1;
            i += 1;
            continue;
        }
        let start = i;
        while i < text.len() && !text[i].is_ascii() {
            i += 1;
        }
        // Every remaining ASCII output still owes one keystroke, so the
        // rest of `keys` belongs to this run of rule outputs.
        let end = keys
            .len()
            .saturating_sub(text[i..].iter().filter(|c| c.is_ascii()).count());
        out.push((text[start], keys[pos.min(end)..end].iter().collect()));
        out.extend(text[start + 1..i].iter().map(|ch| (*ch, String::new())));
        pos = end;
    }
    out
}

/// Evaluate a run of romaji keystrokes: convert the whole run and record
/// one element per output character.
///
/// Rule outputs never contain ASCII (see the converter's contract), so an
/// ASCII character in the output is a keystroke that passed through: it
/// stays live (`Romaji`) if it can still begin a rule (`ykt` → BS → `o`
/// → 「yこ」) and settles otherwise (`1`). Everything else is a fired
/// rule's output, settled for good. The trailing pending stays `Romaji`
/// per keystroke.
///
/// Settling is where the configured width applies, after the classification
/// above: a character settles at the width in force when it was typed, so
/// switching to alphabet input mid-word (`（` then Shift+A) leaves what is
/// already settled alone.
fn evaluate_run(run: &str, romaji: &RomajiConverter) -> Vec<Element> {
    let converted = romaji.convert(run);
    let consumed: String = run
        .strip_suffix(converted.pending.as_str())
        .unwrap_or(run)
        .to_string();
    attribute_keys(&converted.text, &consumed)
        .into_iter()
        .map(|(ch, raw)| {
            if romaji.starts_rule(ch) {
                Element::Romaji(ch)
            } else {
                Element::converted(romaji.width().apply(ch), raw)
            }
        })
        .chain(converted.pending.chars().map(Element::Romaji))
        .collect()
}
