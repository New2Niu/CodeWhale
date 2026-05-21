//! `/skill` picker modal: lists available skills and inserts `/skill <name>`
//! into the composer without executing — letting the user add args before
//! pressing Enter a second time.
//!
//! Single-pane list with live search: type to filter by name/description,
//! ↑/↓ moves, Enter selects, Esc/Bksp (empty filter) cancels.

use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Padding, Paragraph, Widget},
};

use crate::palette;
use crate::tui::views::{ModalKind, ModalView, ViewAction, ViewEvent};

/// Lightweight skill row suitable for picker rendering.
#[derive(Debug, Clone)]
pub struct SkillRow {
    pub name: String,
    pub description: String,
}

pub struct SkillPickerView {
    /// Unfiltered master list.
    all_skills: Vec<SkillRow>,
    /// Current filter text that narrows the visible subset.
    filter: String,
    /// Cursor index into the *filtered* list.
    cursor: usize,
}

impl SkillPickerView {
    #[must_use]
    pub fn new(skills: Vec<SkillRow>) -> Self {
        Self {
            all_skills: skills,
            filter: String::new(),
            cursor: 0,
        }
    }

    /// Return the slice of skills that match the current filter.
    fn visible_skills(&self) -> Vec<&SkillRow> {
        if self.filter.is_empty() {
            self.all_skills.iter().collect()
        } else {
            let lower = self.filter.to_ascii_lowercase();
            self.all_skills
                .iter()
                .filter(|s| {
                    s.name.to_ascii_lowercase().contains(&lower)
                        || s.description.to_ascii_lowercase().contains(&lower)
                })
                .collect()
        }
    }

    fn selected_name(&self) -> Option<String> {
        let visible = self.visible_skills();
        visible.get(self.cursor).map(|s| s.name.clone())
    }
}

impl ModalView for SkillPickerView {
    fn kind(&self) -> ModalKind {
        ModalKind::SkillPicker
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn handle_key(&mut self, key: KeyEvent) -> ViewAction {
        match key.code {
            KeyCode::Esc => return ViewAction::Close,
            KeyCode::Enter => {
                if let Some(name) = self.selected_name() {
                    return ViewAction::EmitAndClose(ViewEvent::SkillPickerSelected {
                        name: name.to_string(),
                    });
                }
                return ViewAction::Close;
            }
            KeyCode::Up | KeyCode::Char('k')
                if !key.modifiers.intersects(crossterm::event::KeyModifiers::CONTROL) =>
            {
                if self.cursor > 0 {
                    self.cursor -= 1;
                }
                return ViewAction::None;
            }
            KeyCode::Down | KeyCode::Char('j')
                if !key.modifiers.intersects(crossterm::event::KeyModifiers::CONTROL) =>
            {
                let count = self.visible_skills().len();
                if self.cursor + 1 < count {
                    self.cursor += 1;
                }
                return ViewAction::None;
            }
            KeyCode::Backspace => {
                if !self.filter.is_empty() {
                    self.filter.pop();
                    self.cursor = 0;
                }
                return ViewAction::None;
            }
            KeyCode::Char(ch) => {
                if !ch.is_control() {
                    self.filter.push(ch);
                    self.cursor = 0;
                }
                return ViewAction::None;
            }
            _ => {}
        }
        ViewAction::None
    }

    fn render(&self, area: Rect, buf: &mut Buffer) {
        let visible = self.visible_skills();
        let popup_width = 72.min(area.width.saturating_sub(4)).max(48);
        let list_rows = visible.len() as u16;
        let needed_height = list_rows.saturating_add(5);
        let popup_height = needed_height.min(area.height.saturating_sub(4)).max(8);

        let popup_area = Rect {
            x: area.x + (area.width.saturating_sub(popup_width)) / 2,
            y: area.y + (area.height.saturating_sub(popup_height)) / 2,
            width: popup_width,
            height: popup_height,
        };

        Clear.render(popup_area, buf);

        // Show the current filter inside the title.
        let title_display = if self.filter.is_empty() {
            " Skills ".to_string()
        } else {
            format!(" Skills: {} ", self.filter)
        };

        let block = Block::default()
            .title(Line::from(Span::styled(
                title_display,
                Style::default()
                    .fg(palette::DEEPSEEK_SKY)
                    .add_modifier(Modifier::BOLD),
            )))
            .title_bottom(Line::from(vec![
                Span::styled(" Type ", Style::default().fg(palette::TEXT_MUTED)),
                Span::raw("to filter "),
                Span::styled(" \u{2191}/\u{2193} ", Style::default().fg(palette::TEXT_MUTED)),
                Span::raw("move "),
                Span::styled(" Enter ", Style::default().fg(palette::TEXT_MUTED)),
                Span::raw("select "),
                Span::styled(" Esc ", Style::default().fg(palette::TEXT_MUTED)),
                Span::raw("cancel "),
            ]))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(palette::BORDER_COLOR))
            .style(Style::default().bg(palette::DEEPSEEK_INK))
            .padding(Padding::uniform(1));

        let inner = block.inner(popup_area);
        block.render(popup_area, buf);

        if visible.is_empty() {
            let msg = if self.all_skills.is_empty() {
                Paragraph::new(Span::styled(
                    "No skills installed. Create .deepseek/skills/<name>/SKILL.md",
                    Style::default().fg(palette::TEXT_MUTED),
                ))
            } else {
                Paragraph::new(Line::from(vec![
                    Span::styled(
                        "No skills match \"",
                        Style::default().fg(palette::TEXT_MUTED),
                    ),
                    Span::styled(
                        &self.filter,
                        Style::default().fg(palette::DEEPSEEK_SKY),
                    ),
                    Span::styled(
                        "\"",
                        Style::default().fg(palette::TEXT_MUTED),
                    ),
                ]))
            };
            msg.render(inner, buf);
            return;
        }

        let mut lines: Vec<Line> = Vec::with_capacity(visible.len() + 1);

        if self.filter.is_empty() {
            lines.push(Line::from(Span::styled(
                "Type to filter, then select a skill:",
                Style::default().fg(palette::TEXT_MUTED),
            )));
        }

        for (idx, skill) in visible.iter().enumerate() {
            let is_cursor = idx == self.cursor;
            let row_style = if is_cursor {
                Style::default()
                    .fg(palette::SELECTION_TEXT)
                    .bg(palette::SELECTION_BG)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(palette::TEXT_PRIMARY)
            };
            let hint_style = if is_cursor {
                Style::default()
                    .fg(palette::SELECTION_TEXT)
                    .bg(palette::SELECTION_BG)
            } else {
                Style::default().fg(palette::TEXT_MUTED)
            };
            let pointer = if is_cursor { ">" } else { " " };

            lines.push(Line::from(vec![
                Span::styled(format!("{} /{}", pointer, skill.name), row_style),
                Span::raw("  "),
                Span::styled(&skill.description, hint_style),
            ]));
        }

        Paragraph::new(lines).render(inner, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyModifiers;

    fn mk_skills() -> Vec<SkillRow> {
        vec![
            SkillRow {
                name: "alpha".to_string(),
                description: "First skill".to_string(),
            },
            SkillRow {
                name: "beta".to_string(),
                description: "Second skill".to_string(),
            },
            SkillRow {
                name: "abacus".to_string(),
                description: "Counting tool".to_string(),
            },
        ]
    }

    #[test]
    fn filter_narrows_visible_list() {
        let mut view = SkillPickerView::new(mk_skills());
        assert_eq!(view.visible_skills().len(), 3);

        view.handle_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE));
        view.handle_key(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::NONE));
        assert_eq!(view.visible_skills().len(), 1); // only abacus matches "ab"
        assert_eq!(view.filter, "ab");
    }

    #[test]
    fn backspace_clears_filter_character() {
        let mut view = SkillPickerView::new(mk_skills());
        view.handle_key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));
        view.handle_key(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE));
        assert_eq!(view.filter, "xy");

        view.handle_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));
        assert_eq!(view.filter, "x");
        // Backspace again to clear fully
        view.handle_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));
        assert_eq!(view.filter, "");
        assert_eq!(view.visible_skills().len(), 3);
    }

    #[test]
    fn backspace_on_empty_filter_is_noop() {
        let mut view = SkillPickerView::new(mk_skills());
        view.handle_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));
        assert_eq!(view.filter, "");
    }

    #[test]
    fn filter_matches_by_description() {
        let mut view = SkillPickerView::new(mk_skills());
        for ch in "counting".chars() {
            view.handle_key(KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE));
        }
        assert_eq!(view.visible_skills().len(), 1);
        assert_eq!(view.visible_skills()[0].name, "abacus");
    }

    #[test]
    fn enter_emits_selected_skill() {
        let mut view = SkillPickerView::new(mk_skills());
        for ch in "beta".chars() {
            view.handle_key(KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE));
        }
        let action = view.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        match action {
            ViewAction::EmitAndClose(ViewEvent::SkillPickerSelected { name }) => {
                assert_eq!(name, "beta");
            }
            other => panic!("expected SkillPickerSelected, got {other:?}"),
        }
    }

    #[test]
    fn cursor_resets_on_filter_change() {
        let mut view = SkillPickerView::new(mk_skills());
        view.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        view.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        assert_eq!(view.cursor, 2);
        view.handle_key(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::NONE));
        assert_eq!(view.cursor, 0);
    }

    #[test]
    fn enter_with_no_match_closes() {
        let mut view = SkillPickerView::new(mk_skills());
        for ch in "zzz_not_found".chars() {
            view.handle_key(KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE));
        }
        let action = view.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert!(matches!(action, ViewAction::Close));
    }
}
