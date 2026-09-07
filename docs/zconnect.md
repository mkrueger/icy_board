# ZCONNECT: overview and setup guide

Icy Board implements a limited ZCONNECT leaf-node gateway between public
ZCONNECT boards and local JAM message bases. This is separate from
[QWKnet and DOVE-Net](qwknet.md) and FTN. Online calling uses ZIP archives and
ZMODEM over Telnet. Offline packet exchange through an external mailer is also
supported. **No live interoperability with an external ZCONNECT system has
been verified.** Test with your peer before deploying unattended.

## Overview: what connects to what?

ZCONNECT is a legacy mailbox-network standard with both an online exchange
protocol and a message format. It is **not the same as ZMODEM**: ZMODEM moves
the files; ZCONNECT negotiates the exchange and describes the messages inside
them. A **leaf node** exchanges its own mail with an upstream peer, rather than
acting as a general-purpose router for other systems.

| Term | Meaning in this setup |
| --- | --- |
| Peer / link | A remote ZCONNECT system with which you have arranged an account and public-board feed. |
| Remote board / Brett | A public discussion area, for example `/PUBLIC/GENERAL`. |
| Local message area | The area callers see in Icy Board; its JAM base stores the messages. |
| Mapping | Connects a remote board name to that local area's JAM base path. |
| Inbound / outbound | Separate spool directories for received and queued ZIP packets, not the live message bases. |

The normal online flow is:

```text
Local public posts in a mapped JAM base
  -> scan: prepare a retryable ZIP packet
  -> poll: log in, negotiate ZCONNECT, exchange files using ZMODEM
  -> toss: import received public posts into mapped JAM bases
  -> callers read and reply in the local message area
```

The `zconnect-poll` command performs all three steps. You do not need a separate
scan/toss script for normal online use. An external mailer can instead handle
transport while Icy Board scans and tosses packets offline.

## Before you start

- Have a working Icy Board installation and an `icbmailer` binary available.
- Arrange a connection with an actual ZCONNECT peer. There is no built-in
  public server directory or automatic network enrollment.
- Obtain the dial host and port, our account/system name at the peer, its
  password, the login profile (`zconnect`, `janus` or `direct`), and the exact
  public-board names supplied by that peer. Confirm ZIP/ZMODEM compatibility.
- Agree on your fully qualified message-system name and the text encoding.
  Start with ASCII subjects/names and ASCII or explicit ISO1 message bodies.
- Ask the peer to enable the desired **public** board subscriptions. Adding a
  local mapping does not subscribe to the board remotely; there is no MAPS
  subscription interface in this implementation.
- Back up any existing message bases and use dedicated test areas for the
  first exchange. Do not map a private mailbox or an unrelated FTN/QWK feed.
- Use a trusted connection or a secure tunnel: Telnet transmits credentials
  and mail in cleartext.

## Quick start: first public board

### 1. Prepare a local message area

In ICBSetup, create or choose a public message area in the conference callers
will use. Note its **JAM base path**, for example
`conferences/zconnect/general`, without `.jhr` or `.jdt`.

Use that exact path in the ZCONNECT mapping. The importer can create the JAM
files, but a network mapping does **not** create a conference or add a visible
area to its area list. Configure the local read/write access separately.

### 2. Set the local identity

Open **Messaging & Networking → ZCONNECT → General settings**. Fill in:

- **Local system (FQDN):** the message-system name agreed with the peer,
  such as `bbs.example.org`.
- **Local user:** the fallback sender localpart, normally `sysop`.
- **Configuration file:** keep the default `zconnect.toml` unless your board
  stores its network configuration elsewhere.
- **Inbound / outbound:** keep the separate default spool directories.
- **Enabled:** turn on when the identity and link configuration are complete.

### 3. Add the peer and its area mapping

Under **Links**, press **Insert** to add a peer. Give it a stable local ID such
as `PEER`, then enter the host, port, username/account, password and login
profile supplied by the remote sysop. The optional **Expected peer SYS** is
the remote system's advertised name, not necessarily its DNS hostname.

Press **F2** on the link to open its area mappings, then **Insert**. Set:

| Field | Example |
| --- | --- |
| Remote board | `/PUBLIC/GENERAL` — replace with the actual subscribed name |
| Local JAM base | `conferences/zconnect/general` — same path as step 1 |
| Read only | `true` for the initial receive-only test |

Return with **Esc**, then save the board settings. Merely leaving a form does
not save the configuration to disk. The separate network file is saved along
with the board configuration.

### 4. Check and receive

From the board directory, run `icbmailer zconnect-links icyboard.toml PEER`.
Replace `icyboard.toml` with the actual board configuration filename. Check that
the intended host and area count are shown. This checks configuration, **not**
network reachability or credentials.

Next run `icbmailer zconnect-poll icyboard.toml PEER`. With the mapping read-only,
no local posts from that area are exported. Inspect the result:

- Successfully imported ZIP packets move to `zconnect/inbound/PEER/processed/`.
- Unsupported content or unknown board names move to `retained/` for review.
- Malformed packets stay in the inbox and cause a failure status.
- No mail waiting is possible even after a successful connection; confirm the
  subscription and test-message availability with the remote sysop.

Log into Icy Board and open the local area to check the sender, subject, text
and replies. An imported message count alone does not prove the area is visible
or correctly configured for callers.

### 5. Test sending, then automate

After receive-only testing succeeds, turn off **Read only** for the mapping
and save. Write a small public test post through Icy Board and poll again.
Ask the remote sysop to confirm that the post arrived correctly. Poll once
more to check that already imported messages are not sent back.

**The first scan can export existing eligible local posts in the mapped base**;
it does not start at the time you enable the network. Use a new test base to
avoid unexpectedly sending an area's history.

Only after successful two-way testing, schedule `zconnect-poll` using your OS
scheduler or an appropriate board event. Use absolute executable/configuration
paths, capture output and nonzero exit statuses, and avoid overlapping runs.
No automatic polling schedule is created by the ZCONNECT settings. Retained
archives require review and consume disk space; plan backups and cleanup.

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

## Troubleshooting

| Symptom | Check / action |
| --- | --- |
| No ZCONNECT configuration, or processing disabled | Save the board after editing; check its `[paths].zconnect_file` selection and the network's `enabled` flag. Commands take the board configuration, not the standalone network file. |
| Invalid local system or board mapping | Use a fully qualified local system name, uppercase remote board paths, and unique link IDs/mappings. Do not enter a conference number as a JAM path. |
| Timeout during login | Verify host, port and the agreed login profile. A normal BBS login prompt is not necessarily a ZCONNECT endpoint. `direct` expects an endpoint already starting the protocol. |
| Authentication or expected-SYS failure | Check our account name (`username`), the agreed password (1–10 printable ASCII characters online), and the peer's actual advertised SYS. A successful DNS lookup is not a SYS-name check. |
| Unsupported protocol or archive mode | The initial profile requires ZIP/ZIP2 and ZMODEM. Agree on that profile with the peer; arbitrary transfer protocols and encrypted/deferred modes are not implemented. |
| No messages visible locally after import | Ensure the conference's area list points to the same JAM base and grants the caller read access. A network mapping alone does not create that UI entry. |
| Unknown boards / retained packet | Match the actual incoming board names and subscriptions. After correcting a mapping, deliberately retry the retained ZIP; already imported copies are deduplicated. |
| Unsupported content or encoding | Review the limits below. Private mail, binary content and unsupported charsets are retained, not delivered as public text. Do not delete the only copy. |
| Local post is not exported | Check read-only mapping, message privacy/password/hold flags, origin and encoding. Imported network messages are not automatically forwarded. |
| Same outgoing ZIP appears on another scan | Expected until receipt is confirmed. Retry online polling; use manual `zconnect-ack` only after verified external delivery. |
| Pending archive/checkpoint mismatch | Preserve both files and backups; investigate external modification or an incomplete manual workflow. Do not delete the checkpoint to force a rescan. |
| Poll already active | Check for another running mailer and overlapping schedules. Do not remove lock files to bypass it; the lock is released when the owning handle closes. |

## Further reading

- [Network configuration reference](source/configuration/networks.rst): exact
  fields, defaults, validation and generated state-file formats.
- [QWKnet](qwknet.md): a separate alternative for compatible public-message hubs.
- [ZCONNECT 3.1 specification](../crates/doc/ZCONNECT.pdf): protocol reference;
  the implementation supports only the subset described here.

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