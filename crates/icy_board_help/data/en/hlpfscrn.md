# Help: Full Screen Editor : Keyboard Help

The full-screen message editor lets you move through your draft and
change text in place. At the message command prompt, `F` or `V` enters
full-screen editing in insert mode. Text wraps at the editing margin.

## Subcommands

- `Esc` or `Ctrl-U` Leave full-screen editing for the message command
  prompt. This does not save or discard the draft.
- `F1` or `Ctrl-Z` Display this keyboard help. Press `Enter` after the
  help to return to the draft.
- `Ctrl-L` Redraw the editing screen.

`F1` is the only function key assigned by the editor's keyboard
matcher. Other function keys are not save, abort or formatting keys.
If your terminal does not send a recognized key sequence, use the
listed control-key alternative. The local sysop console may reserve
function keys for its own commands.

## Movement

- `Left` / `Ctrl-S` Move left one column.
- `Right` / `Ctrl-D` Move right one column.
- `Up` / `Ctrl-E` Move up one line.
- `Down` / `Ctrl-X` Move down one line.
- `Home` / `Ctrl-W` Move to the start of the line.
- `End` / `Ctrl-P` Move to the end of the line.
- `Ctrl-Left` / `Ctrl-A` Move to the previous word.
- `Ctrl-Right` / `Ctrl-F` Move to the next word.
- `PageUp` / `Ctrl-R` Move up a page.
- `PageDown` / `Ctrl-C` Move down a page.

## Editing

- `Insert` / `Ctrl-V` Toggle insert and overwrite modes.
- `Enter` Split the line at the cursor in insert mode; in overwrite
  mode, move to the start of the next line.
- `Ctrl-N` Split the line at the cursor regardless of editing mode.
- `Tab` / `Ctrl-I` Advance to the next tab stop. In insert mode this
  can insert spaces; near the right margin it moves to the next line.
- `Backspace` / `Ctrl-H` Delete the preceding character. At the start
  of a line, try to join it to the previous line.
- `Delete` / `Ctrl-G` Delete at the cursor. At the end of a line, try
  to join text from the next line.
- `Ctrl-T` Delete the word at the cursor.
- `Ctrl-K` Delete from the cursor to the end of the line.
- `Ctrl-Y` Delete the current line.
- `Ctrl-J` Join text from the next line, as space permits.
- `Ctrl-B` Rewrap the current paragraph to the editing width.
- `Ctrl-O` Quote from the original message, when one is available.
  Enter start and end line numbers; `Q` cancels the quote prompt.
- `Ctrl-_` Toggle 72/79-column editing if your terminal sends this
  control character. Narrowing is refused if a line exceeds 72 columns.

## Examples

To finish a draft, press `Esc`, then enter at the message command prompt:

```text
S
```

To discard it, press `Esc`, enter `A`, and confirm with `Y`. To return
to full-screen editing without saving, enter:

```text
F
```
