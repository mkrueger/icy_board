# Help: (E)nter A Message

Use `E` to write a new message in the current message area.
The conference and area must allow you to post messages.
To answer an existing message, use `REPLY` instead.

## Entering a Message

First enter the recipient. Press Enter to accept the
displayed name, normally `ALL`. A name after `E` supplies
the default recipient; you still confirm it at the prompt.
If name checking is enabled, the board may offer a search
for similar-sounding names or ask you to re-enter the name.

Enter a subject, then answer the security and delivery
questions offered by this conference. `N` means public;
`R` means receiver-only. Some conferences choose security
automatically. Private mail may offer a return receipt;
networked conferences may ask about echoing and routing.

The board uses your editor preference or asks whether to
use the full screen editor. In the line editor, enter a
blank line to reach the editor command prompt. The message
is not posted until you save it.

## Editor Subcommands

- `S` Save the message.
- `A` Abandon the draft after confirmation.
- `C` Continue entering text with the line editor.
- `L` List the draft; `L 3` starts at line 3.
- `E` Edit a line, asking for its number.
- `D` Delete a line after confirmation.
- `I` Insert text before a specified line.
- `F` or `V` Switch to the full screen editor.
- `Q` Quote selected lines when replying to a message.
- `U` Upload message text.
- `SA` Attach a file and save, if attachments are allowed.
- `SC` Save and request additional recipients, if enabled.
- `SN` Save a reply and advance within the reader's range.
- `SK` Save a reply and request deletion of the original,
    subject to deletion rights. No yes/no confirmation
    is requested.
- `H` Display this help.

`SN` and `SK` have no next-message or original-message
action when composing a new message with `E`.

## Multiple Recipients

Where conference permissions and limits allow it, enter
`@LIST@` as the recipient to build a carbon list. Enter
names at the following prompts; a blank answer ends the
list. Each named recipient receives a separate private
copy; a copy addressed to `ALL` is public instead.
The `SC` editor command instead saves the original first
and then asks for extra recipients, if the board enables
that feature. A blank recipient ends those copies.

## Example

```text
E SYSOP
```

Confirm SYSOP at the recipient prompt, supply a subject,
write your text, and use `S` at the editor command prompt.
While reading a message, `E` edits its header instead of
starting a new message.
