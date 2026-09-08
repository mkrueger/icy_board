# Hotkey bars

The shared `icy_board_tui::hotkeys` API centralizes **presentation only** for
the hint bars that ICBSetup, ICBSM, ICBText (`mkicbtxt`), the call-wait monitors
and the shared dialogs already had. It neither infers nor remaps handlers, and
it does not add hints to screens that never showed any. Existing input dispatch,
ownership, modifiers and case handling remain authoritative.

## Structured entries and central presets

- `Hotkey::new(key, label)` describes one actual `KeyCode` and an action label.
- `Hotkey::alternatives(keys, label)` describes alternative keys for one action;
  they share the entry's modifiers and are displayed with `/` separators.
- `Hotkey::modified(modifiers, key, label)` describes an explicit modifier chord.
- `HotkeyBar::new(entries)` builds a bar; `HotkeyBar::for_id(id)` selects an
  explicit central preset. `append(other)` consumes and composes bars in order,
  retaining the receiving bar's style selection; it does not deduplicate keys.

The catalog in [presets.rs](../crates/icy_board_tui/src/hotkeys/presets.rs)
defines entries and the public `hotkeys::presets::PRESET_IDS` inventory together.
IDs are exact; an unknown ID panics. Most retain the former footer message IDs,
but they are no longer localized strings to parse. Common action translations
use `hotkey_*` messages in the shared
[English](../crates/icy_board_tui/i18n/en/icy_board_tui.ftl) and
[German](../crates/icy_board_tui/i18n/de/icy_board_tui.ftl) catalogs. Keep key
codes and modifiers out of translated labels, and localize dynamic labels too.
Do not build legacy strings such as `"F1=Help | Esc Back"` or extract bindings
from prose; compose typed entries and presets instead.

Add a preset only for a bar that already exists on screen. Removing a bar, or
inventing one for a screen that had none, is a UI change rather than footer
maintenance.

## Placement and measurement

A bar normally sits on its block's bottom border, exactly where the previous
hint line was:

```rust
let block = Block::bordered().title_bottom(HotkeyBar::for_id("icbsm_menu_keys").line());
```

`line()` returns one centered, contiguous block with a padding cell on each
side, so the surrounding surface keeps its own colour. The API does not truncate
it; keep presets short enough for 80 columns.

For a caller that owns a dedicated hint area instead of a border,
`render(area, buf)` paints measured rows from the top of that area, `rows(width)`
returns the independently centered lines and `height(width)` measures them.
Rows are measured in terminal cells, not bytes or characters. Entries remain
intact when possible; oversized entries wrap at words and then at extended
grapheme boundaries, with the key first. Two of the available cells belong to
the block's padding, so widths below three produce no rows; at three cells a
two-cell grapheme is replaced by `�`. Rendering stays inside the given area and
never fills it edge to edge.

## Symbols, modifiers and themes

`key_symbol` and chord formatting in
[hotkeys.rs](../crates/icy_board_tui/src/hotkeys.rs) own the notation:

| Key | Symbol | Key | Symbol |
| --- | --- | --- | --- |
| Esc | ␛ | Enter | ↵ |
| Tab | ⇥ | Insert | ⎀ |
| Delete | ⌦ | Backspace | ⌫ |
| PageUp | ⇞ | PageDown | ⇟ |
| Home | ↖ | End | ↘ |
| Space | ␠ | Arrows | ↑ ↓ ← → |

Function keys remain `F1`, `F2`, etc. Modifiers remain explicit (`Ctrl+`, `Alt+`,
`Shift+`, `Super+`, `Hyper+`, `Meta+`); `BackTab` is displayed as `Shift+⇥`.
Character case is preserved. Symbols avoid emoji variation selectors but still
require a monospace terminal font with the relevant glyphs; not all fonts have
them. Central display-cell measurement cannot fix a font's missing symbols.

The active theme supplies `key_binding` for chords and `key_binding_description`
for labels. Styles are read at layout/render time unless `with_styles(key, label)`
overrides them: deterministic theme tests do that, and so do the call-wait
monitors, which keep the call-wait screen's DOS palette through
`cws_chrome::hotkeys` instead of the administration theme. Their shared chrome
also supplies yellow double borders, a centered red `[ Title ]`, the board's
configured date format at the left and the clock at the right. Panel titles and
modal dimming use the same runtime palette, independent of admin settings.
The normal call-wait main screen retains its white double border and plain
yellow title on blue; the bracketed red titles apply to its subscreens only.
Do not hard-code per-tool colours anywhere else.

## Modes, conditions and overlays

Select presets using the same screen state and conditions as the runtime keys:

- Text-editor command, edit, filter, jump and quit modes have separate presets.
- Log views keep their existing source, navigation and mode hint areas, and show
  the search-edit hints while typing.
- Keep path browse availability, dirty-only F2 user saves, backup-only restore
  and selected-node monitoring conditional. Keep counters, filter context and
  status text out of the key hints. Appending a dynamic sort label should use
  the user-list actions preset that omits F4, to avoid duplicating it.
- Editors keep hiding their own hints while a modal dialog owns the keyboard,
  through the existing caller-side `modal` flags.

`Hotkey::matches` is an optional exact code/modifier helper for press/repeat
events, never releases. It normalizes BackTab to Shift+Tab, but does not infer
case-insensitive application shortcuts. The preset catalog is not a dispatcher;
do not replace existing handlers with it as part of presentation maintenance.
