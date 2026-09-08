# Help: (REPLY) To Messages

Use `REPLY` to answer an existing message in the current
message area. Supply its number after the command or at
the reply prompt. A blank reply prompt cancels the command.
The original must be readable, and you must have permission
to post in the destination. Read-only areas reject replies.

## Recipient and Subject

The reply normally goes to the original sender. If you
reply to your own message, it goes to the original
recipient instead. The original subject is used as the
default; when a subject prompt appears, press Enter to
keep it or type a replacement.

An ordinary reply to private mail remains private. Group
password mail requires read authorization and an ordinary
reply keeps its group password. Other security, receipt,
and network delivery prompts depend on the conference.

## Reader Subcommands

At the End of Message Command prompt:

- `REPLY` or `RE` Reply to the message currently displayed.
- `RO` Ask for a different recipient before composing.

`RO` asks for a subject as well. It selects security for
the new recipient rather than automatically keeping the
original privacy or group password. Choose appropriately.
Carbon lists using `@LIST@` are not accepted by `RO`.

## Composing the Reply

Write the reply with the normal message editor. Original
text is available to quote but is not automatically added
to your draft. At the editor command prompt, `Q` lists
the source lines and asks which first and last lines to
quote. Answer `Q` at either line prompt to cancel quoting.

- `S` Save the reply.
- `A` Abandon it after confirmation.
- `SN` Save and advance within the reader's current range.
- `SK` Save, then request deletion of the original.
  Deletion requires permission and may ask for a password,
  but there is no separate yes/no confirmation. If deletion
  cannot be done, the saved reply remains.

The standalone `REPLY` command does not start a reading
range, so `SN` there simply saves and returns.

## Example

```text
REPLY 42
```

This answers message 42 in the selected area. To reply to
a message already on screen, enter `RE` without a number.
