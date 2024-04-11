use ratatui::prelude::*;

use crate::theme::{DOS_LIGHT_CYAN, THEME};

pub struct RgbSwatch;

impl Widget for RgbSwatch {
    #[allow(clippy::cast_precision_loss, clippy::similar_names)]
    fn render(self, area: Rect, buf: &mut Buffer) {
        for (yi, y) in (area.top()..area.bottom()).enumerate() {
            let value = f32::from(area.height) - yi as f32;
            let value_fg = value / f32::from(area.height);
            let value_bg = (value - 0.5) / f32::from(area.height);

            for (xi, x) in (area.left()..area.right()).enumerate() {
                let mut upper = Color::Rgb((64.0 * value_fg) as u8, (128.0 * value_fg) as u8, (255.0 * value_fg) as u8);
                let lower = Color::Rgb((64.0 * value_fg) as u8, (128.0 * value_bg) as u8, (255.0 * value_bg) as u8);

                if y == area.top() {
                    if let Some(col) = THEME.tabs_selected.fg {
                        upper = col;
                    }
                }

                buf.get_mut(x, y).set_char('▀').set_fg(upper).set_bg(lower);
            }
        }
    }
}
