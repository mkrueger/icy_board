# Help: Message Security

Message security determines who can read a message and
whether a password protects it. This is not encryption.
Choose the appropriate letter at the security prompt;
the available choices depend on conference policy and
your permissions.

## Security Choices

- `N` No special protection. The message is public to
  callers who can read the area.
- `R` Receiver-only mail. The sender, recipient, and users
  with read-all-mail permission can read it.
- `S` Sender password. The message remains publicly
  readable; the password protects modification, not
  reading. It does not make the message private.
- `G` Group password. Reading requires the password,
  except for users with read-all-mail permission.

Passwords accept up to 12 characters and are uppercased
by these prompts. When entering a new group-password
message, confirm that callers must know the password,
then supply a nonempty password.

## Conference Policy

Some conferences force private messages, while others
disallow private messages and skip the security question.
Internet e-mail and named carbon-list copies are private;
a carbon copy addressed to `ALL` is public instead.
When choosing security for a new message to `ALL`, `R`
is refused: address receiver-only mail to a person.

For new messages, Enter at the security prompt selects
`N` when the prompt is offered. Private messages to a
person may offer a return-receipt request if permitted.

## Pack-Out Date

When composing a new message, authorized users may also
choose `D` to set a pack-out date for later message-base
maintenance. This leaves the message public. A blank
date skips setting it. `D` is not a header-edit security
choice and is not a delayed-delivery command.

## Changing Existing Messages

Inside the reader, `E` edits a message header. Its `P`
option offers `N`, `R`, `S`, and `G`, subject to ownership
and administrative permissions. The current security is
the default. Choosing `N` removes private and password
protection; `R` clears a password and sets private status;
`S` or `G` sets a password and clears private status.
