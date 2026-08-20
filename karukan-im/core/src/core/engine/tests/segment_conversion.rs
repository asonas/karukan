use super::*;
use crate::core::preedit::AttributeType;
use karukan_engine::{LearningCache, LearningConfig};

/// Extract the committed text from an `EngineResult`, if any.
fn commit_text_of(result: &EngineResult) -> Option<String> {
    result.actions.iter().find_map(|a| match a {
        EngineAction::Commit(t) => Some(t.clone()),
        _ => None,
    })
}

/// Type "あいうえお" (5 kana via romaji a,i,u,e,o), move the cursor to
/// position 2, then press Space — entering Conversion with reading "あい"
/// and tail "うえお".
///
/// Seeds the learning cache with exact-match entries for both readings
/// mapped to themselves. Learning candidates are always inserted first
/// (see `build_conversion_candidates`'s "Learning → ... " priority), so the
/// default-selected candidate is guaranteed to equal the reading regardless
/// of whether a real kanji model happens to be loaded on the machine running
/// the test (unlike `engine.converters.kanji = None`, which only skips the
/// *initial* load — `build_conversion_candidates` re-triggers
/// `init_kanji_converter` on every call and will pick up a cached model if
/// one is available locally, making plain hiragana-fallback assumptions
/// non-deterministic).
fn engine_in_partial_conversion() -> InputMethodEngine {
    let mut engine = InputMethodEngine::new();
    let mut cache = LearningCache::new(LearningConfig::default());
    cache.record("あい", "あい");
    cache.record("うえお", "うえお");
    engine.learning = Some(cache);

    for ch in ['a', 'i', 'u', 'e', 'o'] {
        engine.process_key(&press(ch));
    }
    engine.process_key(&press_key(Keysym::LEFT));
    engine.process_key(&press_key(Keysym::LEFT));
    engine.process_key(&press_key(Keysym::LEFT));
    engine.process_key(&press_key(Keysym::SPACE));
    assert!(matches!(engine.state(), InputState::Conversion { .. }));
    assert_eq!(
        engine.candidates().and_then(|c| c.selected_text()),
        Some("あい"),
        "test setup: default candidate for \"あい\" must be deterministic"
    );
    engine
}

/// Like `engine_in_partial_conversion`, but the seeded learning surfaces
/// differ from the readings ("あい"→"藍", "うえお"→"ウエオ") so tests can
/// tell converted text apart from raw kana.
fn engine_in_partial_conversion_with_kanji() -> InputMethodEngine {
    let mut engine = InputMethodEngine::new();
    let mut cache = LearningCache::new(LearningConfig::default());
    cache.record("あい", "藍");
    cache.record("うえお", "ウエオ");
    engine.learning = Some(cache);

    for ch in ['a', 'i', 'u', 'e', 'o'] {
        engine.process_key(&press(ch));
    }
    engine.process_key(&press_key(Keysym::LEFT));
    engine.process_key(&press_key(Keysym::LEFT));
    engine.process_key(&press_key(Keysym::LEFT));
    engine.process_key(&press_key(Keysym::SPACE));
    assert!(matches!(engine.state(), InputState::Conversion { .. }));
    assert_eq!(
        engine.candidates().and_then(|c| c.selected_text()),
        Some("藍"),
        "test setup: default candidate for \"あい\" must be deterministic"
    );
    engine
}

#[test]
fn test_left_keeps_next_segment_converted() {
    // Going back to the previous segment must NOT revert the segment
    // we just left to raw kana.
    let mut engine = engine_in_partial_conversion_with_kanji();

    // Confirm "藍", advance into "うえお" (selected "ウエオ").
    engine.process_key(&press_key(Keysym::RIGHT));
    assert_eq!(
        engine.candidates().and_then(|c| c.selected_text()),
        Some("ウエオ")
    );

    // Go back to the first segment.
    engine.process_key(&press_key(Keysym::LEFT));

    // Current segment re-selects its previous choice "藍"...
    assert_eq!(
        engine.candidates().and_then(|c| c.selected_text()),
        Some("藍")
    );
    // ...and the segment we left stays converted in the preedit.
    let preedit = engine.preedit().unwrap();
    assert_eq!(
        preedit.text(),
        "藍ウエオ",
        "right-side segment must keep its conversion, not revert to うえお"
    );
    assert_eq!(engine.upcoming_segments.len(), 1);
    assert_eq!(engine.confirmed_segments.len(), 0);
}

#[test]
fn test_right_reenters_upcoming_segment_with_previous_selection() {
    let mut engine = engine_in_partial_conversion_with_kanji();

    engine.process_key(&press_key(Keysym::RIGHT)); // into "うえお"
    engine.process_key(&press_key(Keysym::LEFT)); // back to "あい"
    engine.process_key(&press_key(Keysym::RIGHT)); // forward again

    assert_eq!(
        engine.candidates().and_then(|c| c.selected_text()),
        Some("ウエオ"),
        "re-entering the segment must restore its previous selection"
    );
    assert_eq!(engine.upcoming_segments.len(), 0);
    assert_eq!(engine.confirmed_segments.len(), 1);

    // Enter commits everything joined in order.
    let result = engine.process_key(&press_key(Keysym::RETURN));
    assert_eq!(commit_text_of(&result).as_deref(), Some("藍ウエオ"));
}

#[test]
fn test_commit_includes_upcoming_segments() {
    let mut engine = engine_in_partial_conversion_with_kanji();

    engine.process_key(&press_key(Keysym::RIGHT)); // into "うえお"
    engine.process_key(&press_key(Keysym::LEFT)); // back to "あい" (upcoming: ウエオ)

    // Committing from the first segment must include the still-pending
    // converted segment to its right.
    let result = engine.process_key(&press_key(Keysym::RETURN));
    assert_eq!(commit_text_of(&result).as_deref(), Some("藍ウエオ"));
    assert!(matches!(engine.state(), InputState::Empty));
    assert_eq!(engine.upcoming_segments.len(), 0);
}

#[test]
fn test_cancel_restores_reading_including_upcoming_segments() {
    let mut engine = engine_in_partial_conversion_with_kanji();

    engine.process_key(&press_key(Keysym::RIGHT)); // into "うえお"
    engine.process_key(&press_key(Keysym::LEFT)); // back to "あい" (upcoming: ウエオ)

    engine.process_key(&press_key(Keysym::ESCAPE));

    assert!(matches!(engine.state(), InputState::Composing { .. }));
    assert_eq!(engine.preedit().unwrap().text(), "あいうえお");
    assert_eq!(engine.upcoming_segments.len(), 0);
}

#[test]
fn test_shift_arrow_dissolves_upcoming_segments_without_losing_chars() {
    let mut engine = engine_in_partial_conversion_with_kanji();

    engine.process_key(&press_key(Keysym::RIGHT)); // into "うえお"
    engine.process_key(&press_key(Keysym::LEFT)); // back to "あい" (upcoming: ウエオ)

    // Moving the segment boundary invalidates downstream conversions:
    // the upcoming segment reverts to kana and total reading is preserved.
    engine.process_key(&press_shift_key(Keysym::LEFT));

    assert_eq!(engine.upcoming_segments.len(), 0);
    assert_eq!(engine.state.reading(), Some("あ"));
    assert_eq!(engine.conversion_tail.as_deref(), Some("いうえお"));
}

#[test]
fn test_multiple_confirmed_segments_join_correctly_on_commit() {
    let mut engine = engine_in_partial_conversion();

    // Confirm "あい", advance into "うえお".
    engine.process_key(&press_key(Keysym::RIGHT));
    assert!(matches!(engine.state(), InputState::Conversion { .. }));
    assert_eq!(engine.confirmed_segments.len(), 1);

    // Enter commits the confirmed segment + current selection joined in order.
    let result = engine.process_key(&press_key(Keysym::RETURN));
    assert_eq!(commit_text_of(&result).as_deref(), Some("あいうえお"));
    assert!(matches!(engine.state(), InputState::Empty));
}

#[test]
fn test_bare_right_with_no_tail_does_not_commit() {
    let mut engine = InputMethodEngine::new();
    engine.converters.kanji = None;
    for ch in ['a', 'i'] {
        engine.process_key(&press(ch));
    }
    engine.process_key(&press_key(Keysym::SPACE));
    assert!(matches!(engine.state(), InputState::Conversion { .. }));

    let result = engine.process_key(&press_key(Keysym::RIGHT));
    assert!(result.consumed);
    assert!(
        commit_text_of(&result).is_none(),
        "bare Right with no tail must not commit"
    );
    assert!(
        matches!(engine.state(), InputState::Conversion { .. }),
        "state should remain Conversion"
    );
}

#[test]
fn test_bare_left_with_no_confirmed_segments_does_nothing() {
    let mut engine = InputMethodEngine::new();
    engine.converters.kanji = None;
    for ch in ['a', 'i'] {
        engine.process_key(&press(ch));
    }
    engine.process_key(&press_key(Keysym::SPACE));
    assert!(matches!(engine.state(), InputState::Conversion { .. }));
    let preedit_before = engine.preedit().unwrap().text().to_string();

    let result = engine.process_key(&press_key(Keysym::LEFT));
    assert!(result.consumed);
    assert!(commit_text_of(&result).is_none());
    assert!(matches!(engine.state(), InputState::Conversion { .. }));
    assert_eq!(engine.preedit().unwrap().text(), preedit_before);
}

#[test]
fn test_cancel_restores_full_reading_including_confirmed_segments_and_clears_them() {
    let mut engine = engine_in_partial_conversion();

    // Confirm "あい", advance into "うえお".
    engine.process_key(&press_key(Keysym::RIGHT));
    assert_eq!(engine.confirmed_segments.len(), 1);

    engine.process_key(&press_key(Keysym::ESCAPE));

    assert!(matches!(engine.state(), InputState::Composing { .. }));
    assert_eq!(engine.preedit().unwrap().text(), "あいうえお");
    assert_eq!(
        engine.confirmed_segments.len(),
        0,
        "confirmed_segments must be cleared on cancel"
    );
}

#[test]
fn test_partial_conversion_preedit_segments_and_caret() {
    let engine = engine_in_partial_conversion();

    let preedit = engine.preedit().unwrap();
    assert_eq!(preedit.text(), "あいうえお");
    assert_eq!(
        preedit.caret(),
        2,
        "caret should sit right after the highlighted \"あい\" segment"
    );

    let attrs = preedit.attributes();
    assert_eq!(attrs.len(), 2, "expected highlight + underline segments");
    assert_eq!(attrs[0].start, 0);
    assert_eq!(attrs[0].end, 2);
    assert_eq!(attrs[0].attr_type, AttributeType::Highlight);
    assert_eq!(attrs[1].start, 2);
    assert_eq!(attrs[1].end, 5);
    assert_eq!(attrs[1].attr_type, AttributeType::Underline);
}

#[test]
fn test_digit_selection_commits_and_learns_confirmed_segments() {
    // Uses its own fresh (unlearned) cache instead of engine_in_partial_conversion()'s
    // pre-seeded one, so it can assert that record_learning actually ran for the
    // confirmed segment — a pre-seeded cache would make that assertion vacuous.
    let mut engine = InputMethodEngine::new();
    engine.learning = Some(LearningCache::new(LearningConfig::default()));
    for ch in ['a', 'i', 'u', 'e', 'o'] {
        engine.process_key(&press(ch));
    }
    engine.process_key(&press_key(Keysym::LEFT));
    engine.process_key(&press_key(Keysym::LEFT));
    engine.process_key(&press_key(Keysym::LEFT));
    engine.process_key(&press_key(Keysym::SPACE));
    assert!(matches!(engine.state(), InputState::Conversion { .. }));

    let selected_for_ai = engine
        .candidates()
        .and_then(|c| c.selected_text())
        .unwrap()
        .to_string();

    // Confirm the first segment, advance into the tail.
    engine.process_key(&press_key(Keysym::RIGHT));
    assert_eq!(engine.confirmed_segments.len(), 1);

    // Select candidate 1 for the (now current) tail segment.
    let selected_for_tail = engine
        .candidates()
        .and_then(|c| c.selected_text())
        .unwrap()
        .to_string();
    let result = engine.process_key(&press_ctrl(Keysym::KEY_1));
    assert_eq!(
        commit_text_of(&result).as_deref(),
        Some(format!("{selected_for_ai}{selected_for_tail}").as_str())
    );
    assert!(matches!(engine.state(), InputState::Empty));
    assert_eq!(engine.confirmed_segments.len(), 0);

    // The confirmed first segment must have been recorded in the learning cache.
    let learned = engine.learning.as_ref().unwrap().lookup("あい");
    assert!(
        learned
            .iter()
            .any(|(surface, _)| surface == &selected_for_ai),
        "confirmed segment should be recorded in the learning cache, got {:?}",
        learned
    );
}

#[test]
fn test_shrink_range_ignores_predictive_learning_match() {
    let mut engine = InputMethodEngine::new();
    engine.converters.kanji = None;
    // Learn a long phrase whose reading starts with "ゆ".
    let mut cache = LearningCache::new(LearningConfig::default());
    cache.record("ゆーざーじしょをかくにんして", "ユーザー辞書を確認して");
    engine.learning = Some(cache);

    // Type "ゆー" and enter conversion (2 chars).
    engine.process_key(&press_key(Keysym::LEFT)); // no-op, engine empty
    for ch in ['y', 'u'] {
        engine.process_key(&press(ch));
    }
    engine.process_key(&press('-'));
    engine.process_key(&press_key(Keysym::SPACE));
    assert!(matches!(engine.state(), InputState::Conversion { .. }));

    // Shrink down to reading="ゆ" (1 char): the predictive learning match
    // "ユーザー辞書を確認して" (reading "ゆーざーじしょをかくにんして") must NOT
    // become the default selected candidate — it corresponds to a much
    // longer reading than the 1 char actually in scope.
    engine.process_key(&press_shift_key(Keysym::LEFT));

    let candidates = engine.candidates().expect("should be in conversion state");
    let selected = candidates.selected_text().unwrap_or("");
    assert_ne!(
        selected, "ユーザー辞書を確認して",
        "predictive learning candidate must not be auto-selected for a shrunk reading"
    );
    assert!(
        selected.chars().count() <= 3,
        "selected candidate {:?} is suspiciously long for a 1-char reading",
        selected
    );
}

#[test]
fn test_shrink_expand_conversion_range_does_not_duplicate_chars() {
    let mut engine = InputMethodEngine::new();

    // Type "あいうえお" and enter conversion
    for ch in ['a', 'i', 'u', 'e', 'o'] {
        engine.process_key(&press(ch));
    }
    engine.process_key(&press_key(Keysym::SPACE));
    assert!(matches!(engine.state(), InputState::Conversion { .. }));

    // Shrink 4 times (down to 1 char reading), then expand 4 times back.
    for _ in 0..4 {
        engine.process_key(&press_shift_key(Keysym::LEFT));
    }
    for _ in 0..4 {
        engine.process_key(&press_shift_key(Keysym::RIGHT));
    }

    // Total reading length (current conversion reading + tail) must still be 5.
    let reading = engine.state.reading().unwrap_or_default();
    let reading_len = reading.chars().count();
    let tail_len = engine
        .conversion_tail
        .as_deref()
        .map(|t| t.chars().count())
        .unwrap_or(0);
    assert_eq!(
        reading_len + tail_len,
        5,
        "reading={:?} tail={:?}",
        reading,
        engine.conversion_tail.as_deref()
    );
}

#[test]
fn test_shrink_only_repeated_does_not_duplicate_chars() {
    let mut engine = InputMethodEngine::new();

    for ch in ['a', 'i', 'u', 'e', 'o'] {
        engine.process_key(&press(ch));
    }
    engine.process_key(&press_key(Keysym::SPACE));

    // Shrink repeatedly past the minimum (10 times on a 5-char reading).
    for _ in 0..10 {
        engine.process_key(&press_shift_key(Keysym::LEFT));
    }

    let reading = engine.state.reading().unwrap_or_default();
    let reading_len = reading.chars().count();
    let tail_len = engine
        .conversion_tail
        .as_deref()
        .map(|t| t.chars().count())
        .unwrap_or(0);
    assert_eq!(
        reading_len + tail_len,
        5,
        "reading={:?} tail={:?}",
        reading,
        engine.conversion_tail.as_deref()
    );
}

#[test]
fn test_expand_only_repeated_does_not_duplicate_chars() {
    let mut engine = InputMethodEngine::new();

    for ch in ['a', 'i', 'u', 'e', 'o'] {
        engine.process_key(&press(ch));
    }
    engine.process_key(&press_key(Keysym::SPACE));

    for _ in 0..4 {
        engine.process_key(&press_shift_key(Keysym::LEFT));
    }
    // Expand repeatedly past the maximum (10 times when only 4 chars in tail).
    for _ in 0..10 {
        engine.process_key(&press_shift_key(Keysym::RIGHT));
    }

    let reading = engine.state.reading().unwrap_or_default();
    let reading_len = reading.chars().count();
    let tail_len = engine
        .conversion_tail
        .as_deref()
        .map(|t| t.chars().count())
        .unwrap_or(0);
    assert_eq!(
        reading_len + tail_len,
        5,
        "reading={:?} tail={:?}",
        reading,
        engine.conversion_tail.as_deref()
    );
}

#[test]
fn test_advance_then_shrink_expand_does_not_duplicate_chars() {
    let mut engine = InputMethodEngine::new();

    for ch in ['a', 'i', 'u', 'e', 'o'] {
        engine.process_key(&press(ch));
    }
    // Move cursor to middle before converting: "あい" | "うえお"
    engine.process_key(&press_key(Keysym::LEFT));
    engine.process_key(&press_key(Keysym::LEFT));
    engine.process_key(&press_key(Keysym::LEFT));
    engine.process_key(&press_key(Keysym::SPACE));
    assert!(matches!(engine.state(), InputState::Conversion { .. }));

    // Right arrow: advance to next segment (confirm "あい", convert "うえお")
    engine.process_key(&press_key(Keysym::RIGHT));

    // Now shrink/expand repeatedly on the "うえお" segment.
    for _ in 0..5 {
        engine.process_key(&press_shift_key(Keysym::LEFT));
    }
    for _ in 0..5 {
        engine.process_key(&press_shift_key(Keysym::RIGHT));
    }

    let reading = engine.state.reading().unwrap_or_default();
    let reading_len = reading.chars().count();
    let tail_len = engine
        .conversion_tail
        .as_deref()
        .map(|t| t.chars().count())
        .unwrap_or(0);
    assert_eq!(
        reading_len + tail_len,
        3,
        "reading={:?} tail={:?}",
        reading,
        engine.conversion_tail.as_deref()
    );
}

#[test]
fn test_shrink_expand_multi_char_romaji_does_not_duplicate_chars() {
    let mut engine = InputMethodEngine::new();

    // Type "きょうと" via romaji: kyouto
    for ch in ['k', 'y', 'o', 'u', 't', 'o'] {
        engine.process_key(&press(ch));
    }
    engine.process_key(&press_key(Keysym::SPACE));
    assert!(matches!(engine.state(), InputState::Conversion { .. }));

    let expected_len = engine.state.reading().unwrap_or_default().chars().count()
        + engine
            .conversion_tail
            .as_deref()
            .map(|t| t.chars().count())
            .unwrap_or(0);

    for _ in 0..3 {
        engine.process_key(&press_shift_key(Keysym::LEFT));
    }
    for _ in 0..3 {
        engine.process_key(&press_shift_key(Keysym::RIGHT));
    }

    let reading = engine.state.reading().unwrap_or_default();
    let reading_len = reading.chars().count();
    let tail_len = engine
        .conversion_tail
        .as_deref()
        .map(|t| t.chars().count())
        .unwrap_or(0);
    assert_eq!(
        reading_len + tail_len,
        expected_len,
        "reading={:?} tail={:?}",
        reading,
        engine.conversion_tail.as_deref()
    );
}

#[test]
fn test_alphabet_mode_space_inserts_literal_space() {
    let mut engine = InputMethodEngine::new();

    // Enter alphabet mode via Shift+N
    engine.process_key(&press_shift('N'));
    assert!(engine.mode.current() == InputMode::Alphabet);

    // Type "ew"
    engine.process_key(&press('e'));
    engine.process_key(&press('w'));
    assert_eq!(engine.preedit().unwrap().text(), "New");

    // Space → should insert literal space, NOT start conversion
    engine.process_key(&press_key(Keysym::SPACE));
    assert!(matches!(engine.state(), InputState::Composing { .. }));
    assert_eq!(engine.preedit().unwrap().text(), "New ");

    // Type "york"
    engine.process_key(&press('y'));
    engine.process_key(&press('o'));
    engine.process_key(&press('r'));
    engine.process_key(&press('k'));
    assert_eq!(engine.preedit().unwrap().text(), "New york");
}

#[test]
fn test_mid_buffer_conversion_does_not_adopt_longer_predictive_candidate() {
    // `あい|さつ` + Space converts only `あい`; a predictive learning match
    // (`挨拶`, reading あいさつ) must not become the default selection — the
    // tail `さつ` is still in the preedit, so committing it would duplicate
    // those characters (`挨拶さつ`).
    let mut engine = InputMethodEngine::new();
    let mut cache = LearningCache::new(LearningConfig::default());
    cache.record("あいさつ", "挨拶");
    engine.learning = Some(cache);

    for ch in ['a', 'i', 's', 'a', 't', 's', 'u'] {
        engine.process_key(&press(ch));
    }
    assert_eq!(engine.input_buf.display(), "あいさつ");
    engine.process_key(&press_key(Keysym::LEFT));
    engine.process_key(&press_key(Keysym::LEFT));

    engine.process_key(&press_key(Keysym::SPACE));
    assert!(matches!(engine.state(), InputState::Conversion { .. }));
    assert_eq!(engine.conversion_tail.as_deref(), Some("さつ"));

    let selected = engine
        .candidates()
        .and_then(|c| c.selected_text())
        .unwrap_or("");
    assert_ne!(
        selected, "挨拶",
        "predictive learning candidate must not be auto-selected when a tail remains"
    );
}

#[test]
fn test_live_conversion_entry_keeps_displayed_text_selected() {
    // Entering conversion from live conversion (Left arrow) must keep the
    // displayed live text selected even when it is already present in the
    // rebuilt candidate list (deduplicated instead of re-inserted at the
    // top) and a predictive learning candidate outranks it.
    let mut engine = InputMethodEngine::new();
    engine.live.enabled = true;
    let mut cache = LearningCache::new(LearningConfig::default());
    cache.record("あいさつ", "挨拶");
    engine.learning = Some(cache);

    engine.process_key(&press('a'));
    engine.process_key(&press('i'));
    // "アイ" is also produced as a katakana fallback candidate, so the live
    // text gets deduplicated against the list instead of inserted at index 0.
    set_live_text(&mut engine, "アイ");

    engine.process_key(&press_key(Keysym::LEFT));
    assert!(matches!(engine.state(), InputState::Conversion { .. }));
    assert_eq!(
        engine.candidates().and_then(|c| c.selected_text()),
        Some("アイ"),
        "the text displayed during live conversion must stay selected"
    );
}

#[test]
fn test_live_conversion_space_moves_to_strongest_candidate() {
    // Space during live conversion is an explicit conversion request: the
    // selection moves to the strongest candidate — predictions included —
    // rather than sticking to the displayed live text (which stays in the
    // list). Only the arrow-key segment-selection entry keeps the display.
    let mut engine = InputMethodEngine::new();
    engine.live.enabled = true;
    let mut cache = LearningCache::new(LearningConfig::default());
    cache.record("あいさつ", "挨拶");
    engine.learning = Some(cache);

    engine.process_key(&press('a'));
    engine.process_key(&press('i'));
    // "アイ" also exists as a katakana fallback candidate, so the live text
    // is deduplicated into the list rather than inserted at the top.
    set_live_text(&mut engine, "アイ");

    engine.process_key(&press_key(Keysym::SPACE));
    assert!(matches!(engine.state(), InputState::Conversion { .. }));
    assert_eq!(
        engine.candidates().and_then(|c| c.selected_text()),
        Some("挨拶"),
        "Space must select the predictive learning candidate (upstream behavior)"
    );
}
