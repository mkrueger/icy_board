use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Margin, Rect},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Widget},
};

use crate::{chrome::dim_background, get_text, theme::get_tui_theme};

pub enum SaveChangesMessage {
    None,
    Cancel,
    Save,
    Close,
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend};

    #[test]
    fn localized_choices_use_display_cells_and_equal_focus_padding() {
        let mut dialog = SaveChangesDialog::new();
        for save in [false, true] {
            dialog.save = save;
            let line = dialog.choices_line("保存 e\u{301}?".into(), "是".into(), "Nein".into());
            assert_eq!(line.to_string(), "保存 e\u{301}?  是 / Nein ");
            assert_eq!(line.width(), 19);
            assert_eq!(line.spans[1].style, if save { get_tui_theme().selected_item } else { get_tui_theme().item });
            assert_eq!(line.spans[3].style, if save { get_tui_theme().item } else { get_tui_theme().selected_item });
        }
    }

    #[test]
    fn popup_is_bounded_and_dims_before_painting_at_classic_and_tiny_sizes() {
        for (width, height) in [(0, 0), (1, 1), (2, 2), (8, 3), (20, 5), (80, 25)] {
            let dialog = SaveChangesDialog::new();
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            terminal
                .draw(|frame| {
                    let area = frame.area();
                    Line::styled("Background", get_tui_theme().item).render(area, frame.buffer_mut());
                    dialog.render(frame, area);
                })
                .unwrap();
            if width == 80 {
                let buffer = terminal.backend().buffer();
                let mut expected = ratatui::buffer::Buffer::empty(buffer.area);
                Line::styled("Background", get_tui_theme().item).render(buffer.area, &mut expected);
                dim_background(&mut expected, buffer.area);
                assert_eq!(buffer[(0, 0)], expected[(0, 0)]);
                let field = dialog.choices_line(get_text("icbtext_save_changes"), get_text("yes"), get_text("no"));
                let x = (80 - (field.width() as u16 + 4)) / 2;
                assert_eq!(
                    buffer[(x, 11)].style(),
                    get_tui_theme().dialog_box.underline_color(ratatui::style::Color::Reset)
                );
                let no_x = x + 2 + field.spans[..3].iter().map(Span::width).sum::<usize>() as u16;
                assert_eq!(
                    buffer[(no_x, 12)].style(),
                    get_tui_theme()
                        .dialog_box
                        .patch(get_tui_theme().selected_item)
                        .underline_color(ratatui::style::Color::Reset)
                );
            }
        }
    }

    #[test]
    fn default_no_arrows_enter_and_escape_keep_their_meaning() {
        let mut dialog = SaveChangesDialog::new();
        assert!(matches!(dialog.handle_key_press(KeyCode::Enter.into()), SaveChangesMessage::Close));
        assert!(matches!(dialog.handle_key_press(KeyCode::Left.into()), SaveChangesMessage::None));
        assert!(matches!(dialog.handle_key_press(KeyCode::Enter.into()), SaveChangesMessage::Save));
        assert!(matches!(dialog.handle_key_press(KeyCode::Esc.into()), SaveChangesMessage::Cancel));
        assert!(matches!(dialog.handle_key_press(KeyCode::Right.into()), SaveChangesMessage::None));
        assert!(matches!(dialog.handle_key_press(KeyCode::Enter.into()), SaveChangesMessage::Close));
    }
}

pub struct SaveChangesDialog {
    save: bool,
}

impl Default for SaveChangesDialog {
    fn default() -> Self {
        Self::new()
    }
}

impl SaveChangesDialog {
    pub fn new() -> Self {
        Self { save: false }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let backdrop = frame.area();
        dim_background(frame.buffer_mut(), backdrop);
        let area = area.intersection(backdrop);
        let field = self.choices_line(get_text("icbtext_save_changes"), get_text("yes"), get_text("no"));
        let width = field.width().saturating_add(4).min(area.width as usize) as u16;
        let height = area.height.min(3);
        let save_area = Rect::new(area.x + (area.width - width) / 2, area.y + (area.height - height) / 2, width, height);

        Clear.render(save_area, frame.buffer_mut());

        Block::new()
            .borders(Borders::ALL)
            .style(get_tui_theme().dialog_box)
            .border_type(BorderType::Plain)
            .render(save_area, frame.buffer_mut());

        field.render(save_area.inner(Margin { horizontal: 2, vertical: 1 }), frame.buffer_mut());
    }

    fn choices_line(&self, prompt: String, yes: String, no: String) -> Line<'static> {
        let theme = get_tui_theme();
        Line::from(vec![
            Span::styled(format!("{prompt} "), theme.item),
            Span::styled(format!(" {yes} "), if self.save { theme.selected_item } else { theme.item }),
            Span::styled("/", theme.item),
            Span::styled(format!(" {no} "), if self.save { theme.item } else { theme.selected_item }),
        ])
    }

    pub fn handle_key_press(&mut self, key: KeyEvent) -> SaveChangesMessage {
        use KeyCode::*;
        match key.code {
            Left | Right => {
                self.save = !self.save;
                SaveChangesMessage::None
            }
            Enter => {
                if self.save {
                    SaveChangesMessage::Save
                } else {
                    SaveChangesMessage::Close
                }
            }
            Esc => SaveChangesMessage::Cancel,
            _ => SaveChangesMessage::None,
        }
    }
}
