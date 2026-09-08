# Help: (R)ead Messages

Use `R` to read messages in the current message area.
Enter numbers or subcommands after `R`, or answer the Read
Messages prompt. A blank answer at that prompt leaves the
reader. Only messages you are allowed to read are shown;
group-password messages ask for the password.

## Numbers and Ranges

- `42` Read message 42 only.
- `42+` Read from 42 forward to the last message.
- `42-` Read from 42 backward to the first message.
- `10-20` Read messages 10 through 20 in order.
- `20-10` Read the same range in reverse order.
- `L` or `-` Begin at the last message and read backward.

Separate several numbers or ranges with spaces. Press
Enter after each displayed message to continue within
the requested range. After a single message or a finished
range, the reader returns to its Read Messages prompt.

## Selection Subcommands

These meanings apply at the initial Read Messages prompt:

- `S`, `*`, or `+` Read after your saved last-read number.
- `N` Read messages written on or after a date. Enter the
  date as MMDDYY, either after `N` or at the date prompt.
- `U` Select messages not marked read by their recipient.
  This is different from reading after your last-read mark.
- `Y` Select messages addressed to your name or alias.
- `YA` Also include messages addressed to `ALL`.
- `F` Select messages from your name or alias.
- `TS` Search recipient, sender, subject, and message text.
- `TO`, `FROM`, or `USER` Search part of a recipient name,
  sender name, or either name, without regard to case.
- `A` Read across selected accessible conferences.
- `ALL` Include accessible conferences not selected.
- `WAIT` Read across conferences marked as having mail
  waiting, including conferences not selected.

Cross-conference reading normally starts after each area's
last-read mark unless you supply a range or date selection.
It may ask whether to resume an earlier conference scan.
Search commands ask for missing text or names and, when
needed, a starting number. Use the separate search prompt
if your text contains words that are also reader commands.

## While Reading

- `N` Stop reading; it does not mean a date scan here.
- `/` Redisplay the current message.
- `NEXT` or `+` Continue forward from the next message.
- `PREV` or `-` Continue backward from the previous one.
- `REPLY` or `RE` Reply to the message on screen.
- `RO` Reply with a different recipient.
- `M` Memorize this message number.
- `RM` Read the memorized message in this area.
- `RR` Read the referenced message, if there is one.
- `T+` or `T-` Follow messages with the same subject
  forward or backward, ignoring a leading `Re: `.
- `LONG` or `SHORT` Change the displayed header format.
- `HELP` Show the End of Message Command help for more
  actions, including editing, deletion, and capture.

Inside the reader, `E` edits the header and `U` makes a
message public, subject to permission. Do not use them as
new-message or unread-only selection commands there.

## Examples

```text
R 100-110
R S Y
R N 090126
R 1+ TS MODEM
```

These read a range, read newer mail to you, read messages
dated September 1, 2026 or later, and search from message 1.
Reading normally updates your last-read position when the
board enables it. `O` suppresses pointer updates; preventing
recipient-read status updates also requires permission.
Reader `NET` capture is not available.
