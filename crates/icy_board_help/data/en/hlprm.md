# Help: (RM) Re-Read Memorized Message

While a message is displayed, enter `M` at the End of
Message Command prompt to remember its number and area.
This replaces the previously memorized message. Use the
memorized number to return without writing it down.

## Main Command Prompt

- `RM` Return to the memorized area and message, then
  continue forward as you press Enter.
- `RM+` Do the same as `RM`.
- `RM-` Begin at the memorized message and read backward.

## Inside the Reader

- `RM` Read only the memorized message.
- `RM+` Read it and continue forward.
- `RM-` Read it and continue backward.

Reader forms require the memorized message to belong to
the current area. Unlike main-menu `RM`, bare `RM` inside
the reader does not request a forward range.

## Example

At the End of Message Command prompt, enter:

```text
M
```

After leaving the reader, enter `RM` to return. Use `N`
at the end of a displayed message to stop reading again.

The memorized position belongs to this session, not a
permanent bookmark. It records an area and number, not a
conference: return to the original conference before
using it. Normal message access checks still apply, and
a deleted message may no longer be available. If no
matching position is remembered, the board reports that
no message has been memorized.
