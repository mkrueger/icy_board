# ZCONNECT public-text networking

Icy Board implements a limited ZCONNECT leaf-node gateway between public
ZCONNECT boards and local JAM message bases. This is separate from
[QWKnet and DOVE-Net](qwknet.md) and FTN. Online calling uses ZIP archives and
ZMODEM over Telnet. Offline packet exchange through an external mailer is also
supported. **No live interoperability with an external ZCONNECT system has
been verified.** Test with your peer before deploying unattended.

## Configuration

In ICBSetup, open **Mail & Network / Messaging & Networking → ZCONNECT**.
**General** edits the identity, spool directories and enabled flag; **Links**
adds or edits peers. Press **F2** on a link to edit public-board/JAM mappings.
The first edit chooses `zconnect.toml` automatically if no configuration path
exists. Save the board normally; the network file is saved with it.

Set `zconnect_file = "zconnect.toml"` in the board configuration's `[paths]`
section. All five commands take the **board configuration**, not the standalone
network configuration. Processing requires a configured path, `enabled = true`,
and a valid network configuration.

Example network configuration:

```toml
enabled = true
local_system = "bbs.example.org"
local_user = "sysop"
inbound = "zconnect/inbound"
outbound = "zconnect/outbound"

[[link]]
id = "PEER"
host = "peer.example.org"
remote_system = "The Remote BBS"
port = 23
username = ""
password = "CHANGE-ME"
login = "zconnect"
timeout_secs = 60

[[link.area]]
remote_board = "/PUBLIC/GENERAL"
local_area = "conferences/zconnect/general"

[[link.area]]
remote_board = "/PUBLIC/ANNOUNCEMENTS"
local_area = "conferences/zconnect/announcements"
read_only = true
```

- `local_system` is a fully qualified system name. `local_user` is a safe
  address localpart used when a local author's name cannot serve as one.
- Each link ID is unique (case-insensitive), starts with an ASCII letter or
  digit, and contains only ASCII letters, digits, underscores or hyphens.
- `host = ""` selects offline-only use. A nonempty host enables outbound
  calling; it does not start a listener. The dial host may be a hostname, DNS
  alias or IP address and need not match the peer's `SYS` display name.
- `remote_system` optionally specifies the expected peer `SYS` display name,
  compared case-insensitively for ASCII. It defaults to `""` when omitted:
  empty disables the name comparison, but the peer must always send a nonempty
  `SYS`. Names may contain spaces and punctuation and must be at most 255
  printable ASCII bytes, with no CR/LF or other control characters. This is
  an explicit name check, **not cryptographic peer authentication** and not a
  requirement that the peer name match the dial host.
- `login = "zconnect"` selects standard ZCONNECT login; `"janus"` selects
  JANUS login; `"direct"` skips the initial login dialogue for an endpoint
  already in protocol mode. `username` is our account/system name at this peer:
  it is sent as `SYS` during negotiation and as JANUS Systemname, defaulting to
  `local_system`. It may differ from the fully qualified message address domain.
  Standard login uses the ZCONNECT dispatch name
  and authenticates with the link password during negotiation.
  Online authentication requires a nonempty printable ASCII password of at
  most 10 characters, as specified by the Chapter II login profile. Replace
  the example value with the password agreed with the peer.
- `timeout_secs` must be 1–3600 (default 60). The online caller uses the exact
  configured connection/protocol timeout, without a 30-second cap. Separate
  finite login and transfer deadlines and bounded retries still apply.
- Remote board names are uppercase paths such as `/PUBLIC/GENERAL`. Mappings
  are per link. `local_area` is a JAM base path, not an area configuration file.
  Relative paths resolve against the board root. Inbound and outbound must be
  distinct directories. Do not use symlink aliases for spools or packet files.
- `read_only = true` prevents export to that mapping; it does not replace local
  conference access restrictions.

## Commands

| Command | Operation |
| --- | --- |
| `icbmailer zconnect-links <config> [link]` | List configured links and area counts. |
| `icbmailer zconnect-scan <config> [link]` | Prepare or reuse a retryable outbound ZIP packet. |
| `icbmailer zconnect-toss <config> [link]` | Import completed inbound ZIP packets for selected links. |
| `icbmailer zconnect-poll <config> [link]` | Scan, exchange, then toss all completed inbound archives. |
| `icbmailer zconnect-ack <config> <link>` | Manually confirm verified external delivery for exactly one link. |

Omitting the optional link processes all configured links; supplying one selects
that exact ID case-insensitively. Unknown IDs are errors. Acknowledgement always
requires an explicit link; there is no acknowledge-all mode. Command help is
localized in English and German.

Poll processes already received archives even if scanning or exchange fails,
including downloads completed before a disconnect or late protocol failure.
It continues with other selected links and returns failure if any scan,
exchange, import or archive-retirement operation fails. Unsupported or unmapped
mail also produces a failure status so partial processing is not mistaken for
complete delivery into local areas.

## Inbound retention and retries

Place externally received ZIP packets directly in `<inbound>/<link.id>/`, using
the configured spelling of the ID. Only regular files with a `.zip` extension
(case-insensitive) are selected; other links, symlinks, temporary files and
nested directories are not scanned. Finish writing a packet before publishing
its final ZIP name.

After import, a fully supported archive moves into that link's `processed/`
subdirectory. An archive reporting unsupported messages or unknown public
boards moves intact into `retained/` for manual review, including packets where
some supported messages were imported. The original archive bytes are preserved,
not discarded. Neither subdirectory is rescanned automatically. Review mappings
or support limitations before manually returning a retained archive to the
inbox; duplicate detection protects messages already imported.

Collision suffixes may follow the original extension (for example `.zip.1`);
restore a final `.zip` extension when deliberately retrying such an archive.

Malformed packets and import failures leave the original ZIP in the inbox.
Other packets are still attempted. Such failures remain visible on subsequent
runs until an operator resolves them. Archive moves never overwrite an existing
file: repeated names receive numeric suffixes. The move uses a same-filesystem
hard link followed by retirement of the inbox name, with directory syncing on
Unix. A filesystem without hard-link support causes failure and leaves the
inbox original available for recovery. Do not mount archive subdirectories on
a different filesystem.

CLI offline operations share the engine's per-link online-poll lock, while the
engine owns its separate short packet-transaction lock. The CLI does not nest
the online-poll lock around an online exchange, nor acquire the packet lock
before calling the engine. Locks are advisory and their files must never be
deleted to bypass a running process. External mailers must be scheduled so
they cannot modify outbound packets concurrently with scan, poll or ack.

## Manual offline acknowledgement

1. Run `icbmailer zconnect-scan <config> PEER` to prepare the pending archive
   at `<outbound>/PEER/mail.zip`. Keep its checkpoint state beside it intact.
2. Give an external mailer a copy of that exact archive. Preserve the pending
   original unchanged until delivery is verified. Repeated scans reuse it;
   they do not advance past unsent mail.
3. Verify the remote mailer's successful acceptance using its actual receipt or
   delivery record. A local copy, a completed upload, or a dropped connection is
   **not** proof of acceptance. Resolve uncertain delivery with the peer.
4. Only then run `icbmailer zconnect-ack <config> PEER`. This commits the pending
   scan checkpoint and retires that link's outbound archive. Do not automate
   this command blindly after an external process exits or apply it to every
   link. It cannot verify the external receipt for you.
5. Scan again to prepare later local messages. Import separately received
   archives with `icbmailer zconnect-toss <config> PEER`.

Online polling performs acknowledgement internally only after the required
protocol receipt; do not run manual acknowledgement in parallel with it.

## Supported profile and limits

- Public text only: mapped public recipients, message/reply IDs, routing-loop
  detection and duplicate suppression. Imported wire data is preserved in JAM
  extensions and is not blindly re-exported.
  Existing local JAM IDs in FTN or other formats receive stable domain-qualified
  ZCONNECT IDs; the same translation applies to reply references.
- Explicit `CHARSET: ISO1` (ISO-8859-1) text and the safe ASCII subset of legacy
  packets are supported. Unspecified legacy extended-byte mappings are not
  guessed as CP437/CP850. Export requires text losslessly representable as ISO1
  and printable ASCII headers; unrepresentable data is rejected, not mangled.
- Private recipients, attachments and opaque content have no delivery gateway.
  Archives containing unsupported recipients/content are retained for review;
  supported copies addressed explicitly to mapped public boards may still be
  imported. Unknown public boards are not auto-created. Retention must not be
  interpreted as successful delivery of private or otherwise unsupported copies.
- No inbound listener, general private-mail routing, Internet-mail gateway,
  arbitrary BBS login scripting or automatic polling scheduler is provided by
  these commands. Only the documented standard/JANUS/direct login profiles are
  supported, with the implemented ZIP/ZMODEM profile.
- Current resource bounds include 32 MiB compressed/expanded packets, 512 KiB
  message bodies, 32 KiB headers, 4096 messages and 128 ZIP entries per packet.
  An online session receives at most 32 archives and 256 MiB. Oversized input
  fails rather than being silently truncated.
- JAM renumbering behind committed scan positions can require operator
  intervention. Do not remove checkpoints to "fix" a pending-delivery error.

**Security:** Telnet is cleartext, including credentials and mail; it provides
no TLS confidentiality or cryptographic peer authentication. An optional
`remote_system` match does not change this: a peer can claim that display name.
Use only a trusted network or a separately secured tunnel. Credentials are also
stored in cleartext in the configuration. Keep passwords and spool directories
restricted to the mailer account. Synthetic protocol tests do not constitute
live external interoperability verification.