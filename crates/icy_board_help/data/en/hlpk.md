# Help: (K)ill a Message

This command marks a message for deletion making any message making it
inactive and unreadable. The message isn't physically removed until the
message base is packed.

## Subcommands

- `[msg. #|` The message number you wish to kill.

## Description

You can delete any message with this command. There are two prompts
where messages can be deleted. The main prompt and the read message prompt.

RECEIVER ONLY messages can only be killed by the recipient and sender.
SENDER PWD messages can only be killed by the sender.
GROUP PWD can only be killed with the password.
The SysOp may kill any message.

## Examples

To kill a message with the number 42 type:

```text
K 42
```
