use super::*;

#[test]
fn test_conversion_char_commits_selected_candidate_and_starts_new_input() {
    let mut engine = InputMethodEngine::new();
    engine.converters.kanji = None;
    engine.dicts.user = Some(dict_from_json(
        r#"[{"reading":"あい","candidates":[
            {"surface":"藍","score":2.0},
            {"surface":"愛","score":1.0}
        ]}]"#,
    ));

    // Type "あい", open conversion, and focus the second candidate.
    engine.process_key(&press('a'));
    engine.process_key(&press('i'));
    engine.process_key(&press_key(Keysym::SPACE));
    let first_candidate = engine
        .candidates()
        .and_then(|c| c.selected_text())
        .unwrap()
        .to_string();
    engine.process_key(&press_key(Keysym::SPACE));
    let focused_candidate = engine
        .candidates()
        .and_then(|c| c.selected_text())
        .unwrap()
        .to_string();
    assert_ne!(focused_candidate, first_candidate);

    // Typing the next word commits the focused candidate first, then starts
    // the new composition with the same key.
    let result = engine.process_key(&press('k'));
    assert_eq!(
        committed(&result).as_deref(),
        Some(focused_candidate.as_str())
    );
    assert!(result.actions.iter().any(|action| matches!(
        action,
        EngineAction::UpdatePreedit(preedit) if preedit.text() == "k"
    )));
    assert!(matches!(engine.state(), InputState::Composing { .. }));
    assert_eq!(engine.input_buf.display(), "k");

    engine.process_key(&press('a'));
    assert_eq!(engine.input_buf.reading(), "か");
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
fn test_stray_keys_are_consumed_during_conversion() {
    // Unbound chords and special keys must be consumed as no-ops while the
    // conversion window is shown — leaking them would let the application
    // act on them (e.g. Ctrl+R reloading a browser page) mid-conversion.
    let mut engine = InputMethodEngine::new();
    engine.process_key(&press('a'));
    engine.process_key(&press('i'));
    engine.process_key(&press_key(Keysym::SPACE));
    assert!(matches!(engine.state(), InputState::Conversion { .. }));
    let cursor = engine.candidates().unwrap().cursor();

    for key in [
        press_ctrl(Keysym(0x0067)), // Ctrl+g (unbound)
        press_ctrl(Keysym(0x0077)), // Ctrl+w (unbound; closes a browser tab)
        press_key(Keysym(0xffc2)),  // F5
    ] {
        let result = engine.process_key(&key);
        assert!(result.consumed, "key must not leak to the application");
        assert!(matches!(engine.state(), InputState::Conversion { .. }));
        assert_eq!(engine.candidates().unwrap().cursor(), cursor);
    }
}

/// Text of the first Commit action in a result, if any.
fn committed(result: &EngineResult) -> Option<String> {
    result.actions.iter().find_map(|a| match a {
        EngineAction::Commit(text) => Some(text.clone()),
        _ => None,
    })
}

#[test]
fn test_bare_digit_during_conversion_commits_and_starts_new_input() {
    let mut engine = InputMethodEngine::new();
    engine.converters.kanji = None;
    engine.dicts.user = Some(dict_from_json(
        r#"[{"reading":"あい","candidates":[{"surface":"藍","score":1.0}]}]"#,
    ));

    engine.process_key(&press('a'));
    engine.process_key(&press('i'));
    engine.process_key(&press_key(Keysym::SPACE));
    assert!(matches!(engine.state(), InputState::Conversion { .. }));

    let result = engine.process_key(&press('2'));
    assert_eq!(committed(&result).as_deref(), Some("藍"));
    assert!(matches!(engine.state(), InputState::Composing { .. }));
    assert_eq!(engine.input_buf.display(), "2");
}

#[test]
fn test_ctrl_digit_selects_candidate_during_conversion() {
    let mut engine = InputMethodEngine::new();
    engine.dicts.user = Some(dict_from_json(
        r#"[{"reading":"あい","candidates":[
            {"surface":"藍","score":2.0},
            {"surface":"愛","score":1.0}
        ]}]"#,
    ));

    engine.process_key(&press('a'));
    engine.process_key(&press('i'));
    engine.process_key(&press_key(Keysym::SPACE));
    let shown: Vec<String> = engine
        .candidates()
        .unwrap()
        .candidates()
        .iter()
        .map(|c| c.text.clone())
        .collect();

    let result = engine.process_key(&press_ctrl(Keysym::KEY_2));
    assert_eq!(committed(&result).as_deref(), Some(shown[1].as_str()));
    assert!(matches!(engine.state(), InputState::Empty));
    assert!(engine.input_buf.is_empty(), "buffer must be cleared");
}

#[test]
fn test_ctrl_digit_selects_candidate_while_composing() {
    // The suggestion window is on screen while composing, so Ctrl+digit
    // commits straight from it — no Space needed first.
    let mut engine = InputMethodEngine::new();
    engine.converters.kanji = None;
    engine.dicts.user = Some(dict_from_json(
        r#"[{"reading":"あい","candidates":[{"surface":"藍","score":1.0}]}]"#,
    ));

    engine.process_key(&press('a'));
    let result = engine.process_key(&press('i'));
    let shown = result
        .actions
        .iter()
        .find_map(|a| match a {
            EngineAction::ShowCandidates(list) => Some(list.candidates().to_vec()),
            _ => None,
        })
        .expect("suggestion window");
    // The dictionary entry's position depends on what else the suggestion
    // list holds (a loaded model contributes its own row), so select it by
    // the digit it is actually shown under.
    let digit = shown
        .iter()
        .position(|c| c.text == "藍")
        .expect("dictionary candidate in the suggestion window")
        + 1;
    assert!(matches!(engine.state(), InputState::Composing { .. }));

    let result = engine.process_key(&press_ctrl(Keysym(b'0' as u32 + digit as u32)));
    assert_eq!(committed(&result).as_deref(), Some("藍"));
    assert!(matches!(engine.state(), InputState::Empty));
}

#[test]
fn test_ctrl_digit_with_no_suggestion_is_consumed() {
    // Nothing to select: the chord must still be swallowed rather than
    // leaking to the application mid-composition.
    let mut engine = InputMethodEngine::new();
    engine.converters.kanji = None;

    engine.process_key(&press('a'));
    let result = engine.process_key(&press_ctrl(Keysym::KEY_9));
    assert!(result.consumed);
    assert!(committed(&result).is_none());
    assert!(matches!(engine.state(), InputState::Composing { .. }));
}

#[test]
fn test_emoji_digit_selection_does_not_pollute_learning() {
    // Committing an emoji by number must not record `:query` → 😀 into the
    // kana-keyed learning cache, and must leave emoji mode.
    let mut engine = engine_with_learned("あい", "愛");
    engine.process_key(&press(':'));
    assert_eq!(engine.mode.current(), InputMode::Emoji);
    for ch in "smile".chars() {
        engine.process_key(&press(ch));
    }

    let result = engine.process_key(&press_ctrl(Keysym::KEY_1));
    assert!(committed(&result).is_some(), "emoji must commit");
    assert_eq!(engine.mode.current(), InputMode::Hiragana);
    let learned = engine.learning.as_ref().unwrap();
    assert!(
        learned.lookup(":smile").is_empty(),
        "emoji query must not enter the learning cache"
    );
}

#[test]
fn test_ctrl_b_in_conversion_moves_caret_like_left() {
    let mut engine = InputMethodEngine::new();
    for ch in "kyou".chars() {
        engine.process_key(&press(ch));
    }
    engine.process_key(&press_key(Keysym::SPACE));
    assert!(matches!(engine.state(), InputState::Conversion { .. }));

    let result = engine.process_key(&press_ctrl(Keysym::KEY_B));
    assert!(result.consumed);
    assert!(matches!(engine.state(), InputState::Composing { .. }));
    assert_eq!(engine.input_buf.cursor(), 2);
}
