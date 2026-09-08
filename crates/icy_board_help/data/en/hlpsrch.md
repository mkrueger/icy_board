# Help: Searching For Text

Enter the text to find at the search prompt, up to 40
characters. Searches ignore letter case. A blank answer
at the message reader's text prompt stops the search.

## Message Searches

Use `TS` in the message reader to search the recipient,
sender, subject, and body. A match in any one of these
fields is enough. Matching text is highlighted when the
message is displayed.

At the initial Read Messages prompt, the reader asks for
a starting number if no range, since-last-read option, or
date was supplied. A plain number at that search-start
prompt means from that number forward. Inside a message,
`TS` starts after the displayed message unless you give
an explicit range.

Use the separate text prompt for phrases or words that
could be mistaken for reader subcommands. For example,
`ALL`, `NEXT`, and numbers on the reader command line
have command meanings even after `TS`.

## Search Patterns

- `MODEM` Finds that text anywhere in a field or body.
- `FILE TRANSFER` Finds that phrase, including its space.
- `MODEM|TERMINAL` Finds either term in message searches.

Message searches compile terms as regular expressions,
so punctuation such as `.`, `*`, and brackets can have
special meaning. These are not filename wildcards.
Use simple words or phrases for predictable results.
Logical AND and NOT are not implemented correctly:
`&` currently behaves like `|`, and `!` does not exclude
matching messages. Do not rely on them to narrow a search.

## Examples

```text
R 1+ TS MODEM
R S TS
```

The first searches from message 1 forward. The second
asks for text and searches after your last-read mark.
For names only, use reader `TO`, `FROM`, or `USER`; those
match part of a name rather than a text-search pattern.

Searches do not bypass private-message access. Without
read-all-mail permission, text searches skip messages
that require a group password rather than prompting for
the password. Sender-password messages remain searchable.

## Caller Log Searches

If this help appears at a caller-log search prompt, enter
plain text. That search matches a literal part of each
log line, ignoring case; it does not interpret the message
reader's patterns or operators.
