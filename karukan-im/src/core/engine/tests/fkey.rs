use super::*;

// "n" alone does not produce "ん" — "nn" or "n'" is required by the romaji converter.

#[test]
fn test_f6_commits_hiragana_in_composing() {
    let mut engine = InputMethodEngine::new();
    type_str(&mut engine, "aiueo");
    assert_eq!(engine.preedit().unwrap().text(), "あいうえお");

    let result = engine.process_key(&press_key(Keysym::F6));
    assert!(result.consumed);
    assert_eq!(commit_text(&result), Some("あいうえお".to_string()));
    assert!(matches!(engine.state(), InputState::Empty));
}

#[test]
fn test_f7_commits_full_katakana_in_composing() {
    let mut engine = InputMethodEngine::new();
    // "nihonn" → "にほん" ("nn" で "ん" を確定)
    type_str(&mut engine, "nihonn");
    assert_eq!(engine.preedit().unwrap().text(), "にほん");

    let result = engine.process_key(&press_key(Keysym::F7));
    assert!(result.consumed);
    assert_eq!(commit_text(&result), Some("ニホン".to_string()));
    assert!(matches!(engine.state(), InputState::Empty));
}

#[test]
fn test_f8_commits_half_katakana_in_composing() {
    let mut engine = InputMethodEngine::new();
    type_str(&mut engine, "aiueo");

    let result = engine.process_key(&press_key(Keysym::F8));
    assert!(result.consumed);
    assert_eq!(commit_text(&result), Some("ｱｲｳｴｵ".to_string()));
    assert!(matches!(engine.state(), InputState::Empty));
}

#[test]
fn test_f9_commits_fullwidth_romaji_in_hiragana_mode() {
    let mut engine = InputMethodEngine::new();
    // "aiueo" は各1文字ずつ確定するため raw_input = "aiueo"
    type_str(&mut engine, "aiueo");

    let result = engine.process_key(&press_key(Keysym::F9));
    assert!(result.consumed);
    // 打ったローマ字をそのまま全角化: aiueo → ａｉｕｅｏ
    assert_eq!(commit_text(&result), Some("ａｉｕｅｏ".to_string()));
    assert!(matches!(engine.state(), InputState::Empty));
}

#[test]
fn test_f10_commits_halfwidth_romaji_in_hiragana_mode() {
    let mut engine = InputMethodEngine::new();
    type_str(&mut engine, "aiueo");

    let result = engine.process_key(&press_key(Keysym::F10));
    assert!(result.consumed);
    // 打ったローマ字をそのまま: aiueo
    assert_eq!(commit_text(&result), Some("aiueo".to_string()));
    assert!(matches!(engine.state(), InputState::Empty));
}

#[test]
fn test_f10_windows_stays_windows() {
    // ユーザーが "windows" と打った場合、かな変換されても F10 で "windows" に戻れること
    let mut engine = InputMethodEngine::new();
    type_str(&mut engine, "windows");

    let result = engine.process_key(&press_key(Keysym::F10));
    assert!(result.consumed);
    // 逆変換せず打ったローマ字をそのまま返す
    let text = commit_text(&result).unwrap_or_default();
    assert_eq!(text, "windows");
}

#[test]
fn test_f9_windows_fullwidth() {
    let mut engine = InputMethodEngine::new();
    type_str(&mut engine, "windows");

    let result = engine.process_key(&press_key(Keysym::F9));
    assert!(result.consumed);
    let text = commit_text(&result).unwrap_or_default();
    assert_eq!(text, "ｗｉｎｄｏｗｓ");
}

#[test]
fn test_f10_includes_pending_romaji_buffer() {
    let mut engine = InputMethodEngine::new();
    // "nihon" → raw_input="niho", romaji_buffer="n"
    type_str(&mut engine, "nihon");

    let result = engine.process_key(&press_key(Keysym::F10));
    assert!(result.consumed);
    // raw_input "niho" + pending "n" = "nihon"
    assert_eq!(commit_text(&result), Some("nihon".to_string()));
}

#[test]
fn test_f9_includes_pending_romaji_buffer() {
    let mut engine = InputMethodEngine::new();
    type_str(&mut engine, "nihon");

    let result = engine.process_key(&press_key(Keysym::F9));
    assert!(result.consumed);
    // raw_input "niho" + pending "n" = "nihon" → 全角: "ｎｉｈｏｎ"
    assert_eq!(commit_text(&result), Some("ｎｉｈｏｎ".to_string()));
}

#[test]
fn test_f9_commits_fullwidth_alpha_in_alphabet_mode() {
    let mut engine = InputMethodEngine::new();
    // Shift+A でアルファベットモードに入り、そのまま b, c と入力
    engine.process_key(&press_shift('A'));
    engine.process_key(&press('b'));
    engine.process_key(&press('c'));
    assert_eq!(engine.input_mode, InputMode::Alphabet);
    // input_buf = "Abc"

    let result = engine.process_key(&press_key(Keysym::F9));
    assert!(result.consumed);
    // 全角変換: 大小を保ったまま全角に
    assert_eq!(commit_text(&result), Some("Ａｂｃ".to_string()));
    assert!(matches!(engine.state(), InputState::Empty));
}

#[test]
fn test_f10_commits_halfwidth_alpha_in_alphabet_mode() {
    let mut engine = InputMethodEngine::new();
    engine.process_key(&press_shift('A'));
    engine.process_key(&press('b'));
    engine.process_key(&press('c'));
    assert_eq!(engine.input_mode, InputMode::Alphabet);
    // input_buf = "Abc" (半角ASCII なので変化なし)

    let result = engine.process_key(&press_key(Keysym::F10));
    assert!(result.consumed);
    assert_eq!(commit_text(&result), Some("Abc".to_string()));
    assert!(matches!(engine.state(), InputState::Empty));
}

// 無変換: Composing ステートでひらがな↔カタカナをトグル（コミットしない）

#[test]
fn test_muhenkan_toggles_to_katakana_in_composing() {
    let mut engine = InputMethodEngine::new();
    type_str(&mut engine, "aiueo");
    assert_eq!(engine.preedit().unwrap().text(), "あいうえお");

    let result = engine.process_key(&press_key(Keysym::MUHENKAN));
    assert!(result.consumed);
    // コミットは発生しない
    assert!(commit_text(&result).is_none());
    // preedit がカタカナに変わる
    assert_eq!(engine.preedit().unwrap().text(), "アイウエオ");
    assert_eq!(engine.input_mode, InputMode::Katakana);
    // Composing のまま
    assert!(matches!(engine.state(), InputState::Composing { .. }));
}

#[test]
fn test_muhenkan_toggles_back_to_hiragana_in_composing() {
    let mut engine = InputMethodEngine::new();
    type_str(&mut engine, "aiueo");

    engine.process_key(&press_key(Keysym::MUHENKAN)); // → カタカナ
    engine.process_key(&press_key(Keysym::MUHENKAN)); // → ひらがなに戻る

    assert_eq!(engine.input_mode, InputMode::Hiragana);
    assert_eq!(engine.preedit().unwrap().text(), "あいうえお");
}

#[test]
fn test_muhenkan_then_enter_commits_katakana() {
    let mut engine = InputMethodEngine::new();
    // "nihonn" で "にほん" を確定
    type_str(&mut engine, "nihonn");
    assert_eq!(engine.preedit().unwrap().text(), "にほん");

    engine.process_key(&press_key(Keysym::MUHENKAN)); // カタカナモードに
    assert_eq!(engine.preedit().unwrap().text(), "ニホン");

    let result = engine.process_key(&press_key(Keysym::RETURN));
    assert_eq!(commit_text(&result), Some("ニホン".to_string()));
}

// Conversion ステート（Space 後）の F キー

#[test]
fn test_f9_commits_fullwidth_romaji_after_space() {
    let mut engine = InputMethodEngine::new();
    type_str(&mut engine, "aiueo");
    engine.process_key(&press_key(Keysym::SPACE));
    assert!(matches!(engine.state(), InputState::Conversion { .. }));

    let result = engine.process_key(&press_key(Keysym::F9));
    assert!(result.consumed);
    // raw_input "aiueo" → 全角
    assert_eq!(commit_text(&result), Some("ａｉｕｅｏ".to_string()));
    assert!(matches!(engine.state(), InputState::Empty));
}

#[test]
fn test_f10_commits_halfwidth_romaji_after_space() {
    let mut engine = InputMethodEngine::new();
    type_str(&mut engine, "aiueo");
    engine.process_key(&press_key(Keysym::SPACE));

    let result = engine.process_key(&press_key(Keysym::F10));
    assert!(result.consumed);
    // raw_input "aiueo" → 半角そのまま
    assert_eq!(commit_text(&result), Some("aiueo".to_string()));
    assert!(matches!(engine.state(), InputState::Empty));
}

#[test]
fn test_f7_commits_katakana_after_space() {
    let mut engine = InputMethodEngine::new();
    type_str(&mut engine, "nihonn");
    engine.process_key(&press_key(Keysym::SPACE)); // 変換モードへ
    assert!(matches!(engine.state(), InputState::Conversion { .. }));

    let result = engine.process_key(&press_key(Keysym::F7));
    assert!(result.consumed);
    assert_eq!(commit_text(&result), Some("ニホン".to_string()));
    assert!(matches!(engine.state(), InputState::Empty));
}

#[test]
fn test_f6_commits_hiragana_after_space() {
    let mut engine = InputMethodEngine::new();
    type_str(&mut engine, "aiueo");
    engine.process_key(&press_key(Keysym::SPACE));
    assert!(matches!(engine.state(), InputState::Conversion { .. }));

    let result = engine.process_key(&press_key(Keysym::F6));
    assert!(result.consumed);
    assert_eq!(commit_text(&result), Some("あいうえお".to_string()));
    assert!(matches!(engine.state(), InputState::Empty));
}

#[test]
fn test_muhenkan_commits_katakana_after_space() {
    let mut engine = InputMethodEngine::new();
    type_str(&mut engine, "aiueo");
    engine.process_key(&press_key(Keysym::SPACE));
    assert!(matches!(engine.state(), InputState::Conversion { .. }));

    let result = engine.process_key(&press_key(Keysym::MUHENKAN));
    assert!(result.consumed);
    assert_eq!(commit_text(&result), Some("アイウエオ".to_string()));
    assert!(matches!(engine.state(), InputState::Empty));
}

// --- Regression tests (bugs found in code review) ---

#[test]
fn test_f10_after_cursor_move_with_pending_romaji() {
    // Bug 1: flush_romaji_to_composed via cursor movement didn't push a raw entry,
    // so F10 after LEFT dropped the flushed char from the output.
    let mut engine = InputMethodEngine::new();
    type_str(&mut engine, "nihon"); // input_buf="にほ", romaji_buf="n"

    engine.process_key(&press_key(Keysym::LEFT)); // flushes "n"→"ん", cursor moves left
    // Now input_buf="にほん", raw_inputs=["ni","ho","n"]

    let result = engine.process_key(&press_key(Keysym::F10));
    assert_eq!(commit_text(&result), Some("nihon".to_string()));
}

#[test]
fn test_backspace_after_ctrl_space_does_not_corrupt_raw_inputs() {
    // Bug 2: Ctrl+Space (full-width space) wasn't tracked in raw_inputs, so
    // backspacing it would pop the preceding kana's entry and corrupt raw_inputs.
    let mut engine = InputMethodEngine::new();
    type_str(&mut engine, "ai"); // raw_inputs=["a","i"]
    engine.process_key(&press_ctrl(Keysym::SPACE)); // inserts "　", raw_inputs=["a","i"," "]
    engine.process_key(&press_key(Keysym::BACKSPACE)); // removes space, raw_inputs=["a","i"]

    let result = engine.process_key(&press_key(Keysym::F10));
    assert_eq!(commit_text(&result), Some("ai".to_string()));
}

#[test]
fn test_f10_mid_buffer_insert_preserves_order() {
    // Bug 3: raw_inputs was an end-stack; inserting mid-buffer after cursor movement
    // appended the new entry at the wrong position, so F10 emitted romaji out of order.
    let mut engine = InputMethodEngine::new();
    type_str(&mut engine, "aiueo"); // raw_inputs=["a","i","u","e","o"], cursor=5

    engine.process_key(&press_key(Keysym::LEFT));
    engine.process_key(&press_key(Keysym::LEFT)); // cursor=3

    engine.process_key(&press('k'));
    engine.process_key(&press('a')); // inserts "か" at pos 3 → raw_inputs=["a","i","u","ka","e","o"]

    let result = engine.process_key(&press_key(Keysym::F10));
    assert_eq!(commit_text(&result), Some("aiukaeo".to_string()));
}

#[test]
fn test_muhenkan_flushes_pending_romaji_before_toggle() {
    // Bug 4: toggle_katakana_composing didn't flush the romaji buffer first.
    // "as" leaves "s" pending; without the flush, typing "a" after MUHENKAN
    // continues the old sequence "sa"→"さ"→"サ". With the flush "s" is frozen
    // as a literal and the subsequent "a" starts a fresh sequence → "ア".
    let mut engine = InputMethodEngine::new();
    type_str(&mut engine, "as"); // input_buf="あ", romaji_buf="s"

    engine.process_key(&press_key(Keysym::MUHENKAN)); // flush "s" as literal, toggle
    engine.process_key(&press('a')); // fresh "a"→"あ"→katakana"ア"

    assert_eq!(engine.preedit().unwrap().text(), "アsア");
}

#[test]
fn test_ctrl_space_from_empty_then_type_does_not_panic() {
    // Bug (review finding 1): Ctrl+Space from Empty state inserted U+3000 into
    // input_buf without pushing to raw_inputs, so the next keystroke panicked
    // with "insertion index out of bounds".
    let mut engine = InputMethodEngine::new();
    assert!(matches!(engine.state(), InputState::Empty));
    engine.process_key(&press_ctrl(Keysym::SPACE));
    assert!(matches!(engine.state(), InputState::Composing { .. }));
    // Must not panic; just verify it proceeds normally
    engine.process_key(&press('a'));
    assert!(matches!(engine.state(), InputState::Composing { .. }));
}

#[test]
fn test_f10_after_digraph_partial_backspace() {
    // Bug (review finding 2): backspacing the secondary char of a multi-kana
    // group (e.g. "ゃ" from "kya"→"きゃ") left the root "き" with raw="kya",
    // so F10 would emit "kya" even though only "き" was still in the buffer.
    // Fix: backspace on a secondary char now deletes the whole group atomically.
    let mut engine = InputMethodEngine::new();
    type_str(&mut engine, "kya"); // "きゃ" (two chars from one sequence)
    assert_eq!(engine.preedit().unwrap().text(), "きゃ");

    engine.process_key(&press_key(Keysym::BACKSPACE)); // removes whole "きゃ" group
    assert!(matches!(engine.state(), InputState::Empty));

    // Confirm raw_inputs is clean: start fresh and F10 gives only the new input
    type_str(&mut engine, "a");
    let result = engine.process_key(&press_key(Keysym::F10));
    assert_eq!(commit_text(&result), Some("a".to_string()));
}

// --- ヘルパー ---

fn type_str(engine: &mut InputMethodEngine, s: &str) {
    for ch in s.chars() {
        engine.process_key(&press(ch));
    }
}

fn commit_text(result: &EngineResult) -> Option<String> {
    result.actions.iter().find_map(|a| {
        if let EngineAction::Commit(text) = a {
            Some(text.clone())
        } else {
            None
        }
    })
}
