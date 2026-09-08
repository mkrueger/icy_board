# Help: (Q)uick Message Scan

Use `Q` to list message headers in the current message area
without displaying each message body. The listing shows
the message number, reference number, recipient, sender,
and subject. Long names and subjects are shortened to fit.

## Starting the Scan

Enter a starting message number at the scan prompt. The
scan proceeds forward from that number to the last message
in the area. A blank answer leaves without scanning.
Deleted messages and private messages you may not read
are omitted.

For example, enter `Q` at the main command prompt, then:

```text
100
```

This lists accessible headers from message 100 onward.
After the listing, the normal Read Messages prompt lets
you choose a message or reading range. Enter a number to
read that message, or press Enter to leave the reader.

## Status Marks

The character before the number describes the message:

- `-` Public message not marked read.
- A blank means a public message marked read.
- `*` Private message not marked read; `+` means read.
- A backtick marks unread private mail to SYSOP;
  `~` marks read private mail to SYSOP.
- `%` Password-protected message not marked read;
  `^` means it is marked read.

The read mark is the message's recipient-read status, not
your last-read position. Listing headers does not itself
mark messages read or advance your last-read pointer.

## Scan Limits

Although the prompt accepts parts of the reader command
language, this header listing uses only the first starting
number and always scans forward to the end. Range ends,
reverse direction, new-mail and text/user filters, and
cross-conference options are not applied here. Use `R`
for filtered reading or `Y` for a personal mail summary.
