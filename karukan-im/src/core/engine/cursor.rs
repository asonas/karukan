//! Cursor movement and character deletion

use super::*;

impl InputMethodEngine {
    /// Common helper for cursor movement: flush romaji, clear live conversion, set new position
    fn move_caret(&mut self, new_pos: usize) -> EngineResult {
        if !self.converters.romaji.buffer().is_empty() {
            self.flush_romaji_to_composed();
            self.converters.romaji.reset();
        }
        self.live.text.clear();
        self.input_buf.cursor_pos = new_pos;
        self.log_chunk_state("cursor");
        let preedit = self.set_composing_state();
        EngineResult::consumed()
            .with_action(EngineAction::UpdatePreedit(preedit))
            .with_action(EngineAction::HideCandidates)
            .with_action(EngineAction::UpdateAuxText(self.format_aux_composing()))
    }

    /// Handle backspace in composing mode
    pub(super) fn backspace_composing(&mut self) -> EngineResult {
        // If romaji buffer is not empty, backspace from buffer (not from composed text)
        if !self.converters.romaji.buffer().is_empty() {
            self.converters.romaji.backspace();
            if let Some(result) = self.try_reset_if_empty() {
                return result;
            }

            let preedit = self.set_composing_state();
            return EngineResult::consumed()
                .with_action(EngineAction::UpdatePreedit(preedit))
                .with_action(EngineAction::UpdateAuxText(self.format_aux_composing()));
        }

        // Remove character before cursor from composed_hiragana
        if self.input_buf.cursor_pos > 0 {
            self.input_buf.remove_char_before_cursor();
            let new_pos = self.input_buf.cursor_pos;
            let removed_raw = if new_pos < self.raw_inputs.len() {
                self.raw_inputs.remove(new_pos)
            } else {
                String::new()
            };
            // A secondary char of a multi-kana group (e.g. "ゃ" from "kya") has an
            // empty raw entry. Delete back through the group to its root so that
            // F9/F10 never emit romaji for chars that no longer exist.
            if removed_raw.is_empty() {
                while self.input_buf.cursor_pos > 0 {
                    let prev = self.input_buf.cursor_pos - 1;
                    let is_secondary = self.raw_inputs.get(prev).map_or(false, |s| s.is_empty());
                    self.input_buf.remove_char_before_cursor();
                    let p = self.input_buf.cursor_pos;
                    if p < self.raw_inputs.len() {
                        self.raw_inputs.remove(p);
                    }
                    if !is_secondary {
                        break;
                    }
                }
            }
        } else {
            // Nothing to delete
            return EngineResult::consumed();
        }

        if let Some(result) = self.try_reset_if_empty() {
            return result;
        }

        self.refresh_input_state()
    }

    /// Move caret left within hiragana input
    pub(super) fn move_caret_left(&mut self) -> EngineResult {
        let new_pos = self.input_buf.cursor_pos.saturating_sub(1);
        self.move_caret(new_pos)
    }

    /// Move caret right within hiragana input
    pub(super) fn move_caret_right(&mut self) -> EngineResult {
        let total = self.input_buf.text.chars().count();
        let new_pos = (self.input_buf.cursor_pos + 1).min(total);
        self.move_caret(new_pos)
    }

    /// Handle delete key in hiragana mode
    pub(super) fn delete_composing(&mut self) -> EngineResult {
        // If romaji buffer is not empty, don't delete from composed (buffer is at cursor)
        if !self.converters.romaji.buffer().is_empty() {
            return EngineResult::consumed();
        }

        // Delete character at cursor position
        let cursor_pos = self.input_buf.cursor_pos;
        if self.input_buf.remove_char_at_cursor().is_none() {
            return EngineResult::consumed();
        }
        if cursor_pos < self.raw_inputs.len() {
            self.raw_inputs.remove(cursor_pos);
        }

        if let Some(result) = self.try_reset_if_empty() {
            return result;
        }

        self.refresh_input_state()
    }

    /// Move caret to start of input
    pub(super) fn move_caret_home(&mut self) -> EngineResult {
        self.move_caret(0)
    }

    /// Move caret to end of input
    pub(super) fn move_caret_end(&mut self) -> EngineResult {
        let total = self.input_buf.text.chars().count();
        self.move_caret(total)
    }
}
