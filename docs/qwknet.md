# QWKnet and DOVE-Net

For the separate ZCONNECT public-text network and offline-mailer workflow, see
[ZCONNECT networking](zconnect.md).

Icy Board can operate as a leaf node of a QWK message network. The initial
implementation supports Synchronet's HTTP or HTTPS `qwk.ssjs` transport and
public conference mail. It creates REP packets, downloads QWK packets, maps the
hub's conference numbers to local JAM bases, and retains QWK message and reply
IDs for duplicate detection.

## DOVE-Net account

Create a dedicated QWK networking account on Vertrauen. Its user name must be
the board's unique QWK ID: two to eight DOS-safe characters, beginning with a
letter. Do not use a normal caller account.

Until Icy Board supports `HEADERS.DAT`, configure the account to exclude
`HEADERS.DAT` and `VOTING.DAT`, and to include the classic message path and
message/reply ID kludge lines. Conference indexes and QWKE packets are not
required.

## Board configuration

Add the QWKnet configuration file to the `[paths]` section of
`icyboard.toml`:

```toml
qwknet_file = "qwknet.toml"
```

Create `qwknet.toml` beside `icyboard.toml`:

```toml
enabled = true
local_id = "MYBBS"
inbound = "qwknet/inbound"
outbound = "qwknet/outbound"

[[hub]]
id = "VERT"
host = "https://dove.synchro.net"
username = "MYBBS"
password = "the-qwknet-account-password"
poll_minutes = 360

[[hub.area]]
remote_conference = 2001
local_area = "conferences/dove/general"

[[hub.area]]
remote_conference = 2002
local_area = "conferences/dove/advertisements"

[[hub.area]]
remote_conference = 2030
local_area = "conferences/dove/announcements"
read_only = true
```

`local_area` is the JAM base path, not the conference or area TOML file. A
relative path is resolved from the board directory. Each hub has its own
conference mapping, so local user QWK conference numbers do not need to match
the DOVE-Net numbers.

The current DOVE-Net conference numbers are published by Synchronet. Common
ones include:

| Number | Area |
| ---: | --- |
| 2001 | General |
| 2002 | Advertisements |
| 2003 | Entertainment |
| 2004 | Debate |
| 2005 | Hardware/Software Help |
| 2006 | Programming |
| 2007 | Synchronet Discussion |
| 2008 | Synchronet Sysops Only |
| 2009 | Unix Discussion |
| 2010 | DOVE-Net Sysops Only |
| 2011 | Synchronet Programming (Baja) |
| 2012 | Synchronet Programming (JavaScript) |
| 2013 | Synchronet Data |
| 2014 | Synchronet Programming (C/C++) |
| 2015 | HAM Radio |
| 2016 | Internet Discussion |
| 2017 | Pro-Audio Discussion |
| 2018 | Firearms Discussion |
| 2019 | Sports Discussion |
| 2020 | Religious Discussion |
| 2021 | Hobby Corner |
| 2022 | Tech Talk |
| 2030 | Synchronet Announcements |

Apply the access restrictions published by DOVE-Net, especially for the sysop,
data, and announcement areas. `read_only = true` prevents network export but
does not replace the local conference security settings.

## Operation

The normal operation is one command:

```sh
icbmailer qwk-poll /path/to/icyboard.toml VERT
```

It scans local messages, uploads `VERT.REP`, downloads and acknowledges
`VERT.QWK`, and imports the packet. Omitting `VERT` processes every configured
hub.

The individual stages are available for diagnosis or an external scheduler:

```sh
icbmailer qwk-links /path/to/icyboard.toml
icbmailer qwk-scan /path/to/icyboard.toml VERT
icbmailer qwk-toss /path/to/icyboard.toml VERT
```

Scan positions are stored beside the outbound packet as `<hub>.state.toml`.
An existing REP packet is retained and retried until the hub accepts it.

## Current limits

- HTTP and HTTPS are supported; FTP transport is not yet implemented.
- Public conference mail is supported; conference 0 and routed QWK NetMail are
  not yet implemented.
- `HEADERS.DAT`, `VOTING.DAT`, attachments, and QWK control messages are not
  processed.
- Configuration is currently file-based; ICBSetup does not yet edit QWKnet
  hubs or mappings.