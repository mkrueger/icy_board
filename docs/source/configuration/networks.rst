Network configuration files
===========================

This page specifies the persisted FTN, QWKnet and ZCONNECT TOML formats,
including their area mappings and generated state files. These are three
independent networks, not interchangeable transports. FTN exchanges packets
and bundles through binkp; QWKnet provides a public-conference leaf gateway
with Synchronet HTTP(S); ZCONNECT provides a public-text leaf gateway with
ZIP/ZMODEM over Telnet or offline packet exchange.

File selection and notation
---------------------------

Merge these entries into the board configuration's existing ``[paths]`` table;
this fragment is not a complete board configuration:

.. code-block:: toml

   [paths]
   ftn_file = "main/ftn.toml"
   qwknet_file = "main/qwknet.toml"
   zconnect_file = "main/zconnect.toml"

All three fields are path strings, defaulting to ``""`` on deserialization
and in the board constructor. An empty selection uses the network's Rust
default configuration instead of loading a file. A relative configuration
filename is relative to the board configuration directory. Network spool and
JAM paths are relative to the board root, **not** the network file's directory,
except for the FREQ offer-path exception documented below. Absolute paths
remain absolute. ``local_area`` means a JAM base prefix, not an area-list file
or a conference number; normally omit the ``.jhr`` extension.

In the tables, **required** means that deserializing a present record without
that key fails. It does not mean that its string must be nonempty. A Rust
``Default`` implementation alone does not make a TOML field optional.
Defaults below are omission defaults unless explicitly called constructor
defaults. Booleans are unquoted ``true`` or ``false``; paths and addresses are
quoted strings. Arrays defaulting to empty may be omitted. Array-of-table
names are singular: ``[[aka]]``, ``[[link]]``, ``[[route]]``, ``[[hub]]`` and
``[[hub.area]]``/``[[link.area]]``. There is no enclosing ``[ftn]``,
``[qwknet]`` or ``[zconnect]`` table in the separate files.

Integer types are nonnegative TOML integers: ``u16`` is 0--65,535, ``u32`` is
0--4,294,967,295, ``u64`` is 0--18,446,744,073,709,551,615 at the Rust type
level, and ``usize`` is target-pointer-sized. For portable TOML, keep integers
at or below the signed 64-bit TOML maximum, 9,223,372,036,854,775,807.
Additional semantic ranges are stated separately. Floats and quoted numbers
are not substitutes.

.. warning::
   Network structs do not use ``deny_unknown_fields``: an unknown key can be
   silently ignored and lost on save. Generic network-file loading/saving
   performs serde conversion, not QWKnet/ZCONNECT semantic validation.
   Mailer operations apply those validators and additional runtime checks.
   At board load, FTN and QWKnet file read/parse errors are logged and replaced
   in memory by constructor defaults; a subsequent save can overwrite a
   recoverable configuration. A configured ZCONNECT file read/parse error
   instead propagates. A syntactically valid ZCONNECT file is still not proof
   that runtime validation will accept it.

FTN
---

Top-level fields
~~~~~~~~~~~~~~~~

.. list-table:: FTN file root
   :header-rows: 1
   :widths: 24 22 54

   * - Key
     - Type; omitted value
     - Meaning
   * - ``inbound``
     - Path; **required**
     - Received bundles, packets, TICs and requests. Constructor: ``"ftn/inbound"``.
   * - ``outbound``
     - Path; **required**
     - Outgoing spool. Constructor: ``"ftn/outbound"``.
   * - ``netmail``
     - Path; ``"ftn/netmail"``
     - JAM base for incoming netmail and locally originated netmail to scan.
   * - ``bad_netmail``
     - Path; ``"ftn/badmail"``
     - JAM base for netmail from unconfigured sources when secure mode is on.
   * - ``bad_packets``
     - Path; ``"ftn/badpkt"``
     - Quarantine for packets the tosser cannot parse; not the netmail base.
   * - ``nodelist``
     - Path; ``""``
     - Optional plain-text FTN nodelist, not a TOML table.
   * - ``new_areas``
     - Path; ``"ftn/areas"``
     - Root for automatically created echomail JAM bases.
   * - ``new_file_areas``
     - Path; ``"ftn/files"``
     - Root for automatically created TIC file directories.
   * - ``origin``
     - String; ``""``
     - Default origin text for locally originated echomail; an area can override it.
   * - ``options``
     - Table; defaults below
     - Processing, routing, auto-add and logging policy.
   * - ``freq``
     - Table; defaults below
     - File-request service.
   * - ``aka``
     - Array of tables; ``[]``
     - Local addresses, in preference order.
   * - ``link``
     - Array of tables; ``[]``
     - Remote peers and subscriptions.
   * - ``route``
     - Array of tables; ``[]``
     - Exact destination-to-next-hop routes.

Only ``inbound`` and ``outbound`` are required at the root. The constructor's
spool defaults do not apply to a TOML file that omits either one. The FTN
mailer requires at least one AKA when ``options.enabled`` is true; polling
also requires selected links. No automatic poller or inbound listener is
created by these fields.

Addresses: ``[[aka]]``, ``[[link]]`` and ``[[route]]``
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

An address is exactly a quoted decimal ``zone:net/node`` or
``zone:net/node.point``. Every component is 0--65,535; whitespace, signs,
wildcards and an appended ``@domain`` are rejected. Zero components are
accepted by the parser but do not establish a valid assigned network address.
An explicit point zero serializes without ``.0``. Domains are separate fields.

.. list-table:: Each AKA
   :header-rows: 1
   :widths: 24 22 54

   * - Key
     - Type; omitted value
     - Meaning
   * - ``address``
     - Address string; **required**
     - An address this board answers to.
   * - ``domain``
     - String; ``""``
     - Network name, appended as ``@domain`` in the binkp identity.

The primary AKA is the first entry. For a link, selection first tries the
same domain (ASCII case-insensitive) **and** zone, then the same zone, then
the primary AKA. Packet address matching itself compares the four numeric
components, not the domain.

.. list-table:: Each link
   :header-rows: 1
   :widths: 24 22 54

   * - Key
     - Type; omitted value
     - Meaning
   * - ``address``
     - Address string; **required**
     - Remote node/point address.
   * - ``domain``
     - String; ``""``
     - Network name used with the peer's address.
   * - ``host``
     - String; **required**
     - Dial host; explicitly ``""`` enables nodelist lookup.
   * - ``port``
     - ``u16``; ``24554``
     - TCP binkp port. Serde accepts zero; use a reachable nonzero port.
   * - ``password``
     - String; ``""``
     - Cleartext-stored binkp session secret, not a user-password hash.
   * - ``packet_password``
     - String; ``""``
     - Type-2 packet password for inbound verification and outbound headers.
   * - ``area_fix_password``
     - String; ``""``
     - AreaFix subject password from this node; also used when forwarding requests to it.
   * - ``tic_password``
     - String; ``""``
     - Expected inbound TIC password; empty disables that password check.
   * - ``poll_minutes``
     - ``u32``; ``0``
     - Intended polling interval in minutes; currently stored only, not scheduled.
   * - ``areas``
     - Array of strings; ``[]``
     - Subscribed echomail tags, matched ASCII case-insensitively; empty exports no echomail.

The link constructor supplies zero address components and an empty host,
but both keys remain required in TOML. Configure packet passwords as at most
eight ASCII characters: there is no serde length rejection, and the packet
field is only eight characters wide. Inbound comparison trims trailing
whitespace, takes the first eight configured characters and compares without
ASCII case. A mismatch leaves the packet in inbound for investigation; it
does not silently disable authentication. TIC passwords compare without ASCII
case against the trimmed received value. Empty passwords do not authenticate
the source. Store all secrets in a file readable only by the board account.

.. list-table:: Each route
   :header-rows: 1
   :widths: 24 22 54

   * - Key
     - Type; omitted value
     - Meaning
   * - ``destination``
     - Address string; **required**
     - Exact final destination, including point.
   * - ``via``
     - Address string; **required**
     - Address of a configured link receiving the packet next.

There are no wildcard/default-route patterns or route-domain keys. Lookup
uses the first matching destination and then the first matching link address;
a missing ``via`` link makes that route unusable. There is no configuration-wide
duplicate-address, duplicate-route or referential-integrity validator.
Keep AKAs, links and route destinations unambiguous.

``[options]``
~~~~~~~~~~~~~

The entire table and every individual field can be omitted. This struct uses
its custom Rust default to fill missing fields.

.. list-table:: All FTN processing options
   :header-rows: 1
   :widths: 28 18 54

   * - Key
     - Type; default
     - Meaning
   * - ``enabled``
     - Boolean; ``true``
     - Master FTN processing switch; retains configuration when false.
   * - ``process_in``
     - Boolean; ``true``
     - Permit inbound mail processing.
   * - ``process_out``
     - Boolean; ``true``
     - Permit outgoing message scanning/packing.
   * - ``process_orphan``
     - Boolean; ``false``
     - Also process packets not addressed to a local AKA.
   * - ``dial_out``
     - Boolean; ``true``
     - Permit outgoing binkp calls.
   * - ``import_after_xfer``
     - Boolean; ``true``
     - Toss after polling receives files.
   * - ``check_dupe_msg_id``
     - Boolean; ``true``
     - Reject already-seen message IDs in an area.
   * - ``check_dupe_path``
     - Boolean; ``false``
     - Reject echomail whose path already names this board.
   * - ``msgs_to_track``
     - ``u32``; ``0``
     - Message-ID lookback count per area; zero means all messages.
   * - ``secure``
     - Boolean; ``false``
     - Separate netmail from unconfigured packet sources; require configured TIC sources.
   * - ``sysop_change``
     - Boolean; ``true``
     - Rewrite imported netmail recipient ``Sysop`` to ``FIDO_SYSOP``.
   * - ``auto_add``
     - Boolean; ``false``
     - Create a local message area for an unknown echo tag.
   * - ``auto_add_conference``
     - ``usize``; ``0``
     - Zero-based conference receiving automatically added message areas.
   * - ``auto_add_files``
     - Boolean; ``false``
     - Create local directories for validated TICs with unknown area tags.
   * - ``auto_add_file_conference``
     - ``usize``; ``0``
     - Zero-based conference receiving those file directories.
   * - ``pass_thru``
     - Boolean; ``false``
     - Forward unknown echomail tags to subscribed links without a local base.
   * - ``enable_routing``
     - Boolean; ``false``
     - Use routes for packets addressed to another system.
   * - ``route_echo_mail``
     - Boolean; ``false``
     - Apply routing to exported echomail for a configured destination node.
   * - ``re_address``
     - Boolean; ``false``
     - Change a routed packet header's destination to the next hop.
   * - ``make_response``
     - Boolean; ``false``
     - Generate AreaFix result/failure netmail.
   * - ``area_fix_forwarding``
     - Boolean; ``false``
     - Forward unknown subscriptions to the first other link with a nonempty host.
   * - ``auto_add_passthru``
     - Boolean; ``false``
     - Allow unknown AreaFix subscriptions when ``pass_thru`` is also true.
   * - ``default_zone``
     - ``u16``; ``0``
     - Zone used to complete two-dimensional packet addresses.
   * - ``default_net``
     - ``u16``; ``0``
     - Net used to complete two-dimensional packet addresses.
   * - ``log_level``
     - String enum; ``"normal"``
     - Exactly ``"normal"`` (warnings/errors), ``"detailed"`` (also info) or ``"debug"`` (also debug).

Netmail security checks the packet's claimed source address against configured
links, **not** whether the recipient is a local user. Use packet authentication
where source trust matters. On netmail export, a direct configured destination
is preferred, followed by an applicable route, or a sole callable link; do not
expect arbitrary multi-uplink routing. ``netmail`` and ``bad_netmail`` are JAM
bases, not arrays of recipient records.

AreaFix handles netmail to ``AREAFIX`` from configured peers. Its first subject
word is checked as the password. Supported body commands are ``+TAG``, ``-TAG``,
``%+ALL``, ``%-ALL``, ``%LIST``, ``%QUERY`` and ``%HELP``. Successful subscription
changes persist in the link's ``areas`` array. Put the intended forwarding
uplink before other callable links; there is no separate upstream-ID setting.

Auto-add conference numbers must identify existing conferences with the relevant
area/directory-list file configured. This is an operational requirement, not a
serde integer-range check. Message auto-add rejects empty tags, ``.``/``..``,
slashes, backslashes, colons and control characters. File auto-add additionally
requires 1--64 printable ASCII bytes without spaces. Generated names lowercase
the tag and escape punctuation to avoid JAM extension collisions. File auto-add
creates local directories only, not FileFix subscriptions or forwarding.

Nodelist lookup
~~~~~~~~~~~~~~~

``nodelist`` names a conventional comma-separated text nodelist. It has no
nested TOML settings and does not automatically create links. Only links
whose configured ``host`` is empty are looked up. ``IBN`` advertises binkp:
bare ``IBN`` uses ``INA:host`` with port 24554; ``IBN:port`` uses that port with
``INA``; ``IBN:host`` or ``IBN:host:port`` supplies the endpoint directly.
Bracketed IPv6 with a port is supported. Lookup fills both host and port for
that invocation; it does not save them back into the link. Hold/Down entries
are not returned as callable endpoints. Invalid port text in host/port forms
falls back to 24554. No host after lookup means polling that link fails.

File requests: ``[freq]`` and nested records
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

.. list-table:: FREQ configuration
   :header-rows: 1
   :widths: 26 24 50

   * - Key/table
     - Type; omitted value
     - Meaning
   * - ``freq.enabled``
     - Boolean; ``false``
     - Enable service; ``options.enabled`` must also be true.
   * - ``freq.deny``
     - Array of address strings; ``[]``
     - Exact node/point addresses refused service; no masks or domains.
   * - ``freq.limits``
     - Table; defaults below
     - Byte limits, not KiB or MiB quantities.
   * - ``freq.limits.session_bytes``
     - ``u64``; **required if limits table exists**
     - Default for an omitted table: 10,485,760 (10 MiB). Zero means unlimited.
   * - ``freq.limits.daily_bytes``
     - ``u64``; **required if limits table exists**
     - Default for an omitted table: 52,428,800 (50 MiB). Zero means unlimited.
   * - ``freq.path``
     - Array of tables; ``[]``
     - Offered directories; fields below.
   * - ``freq.magic``
     - Array of tables; ``[]``
     - Named file/mask aliases; fields below.

.. list-table:: Each ``[[freq.path]]``
   :header-rows: 1

   * - Key
     - Type; omitted value
     - Meaning
   * - ``path``
     - Path; **required**
     - Directory to list for matching regular files (not recursive).
   * - ``password``
     - String; ``""``
     - Optional request password; ASCII case-insensitive.

.. list-table:: Each ``[[freq.magic]]``
   :header-rows: 1

   * - Key
     - Type; omitted value
     - Meaning
   * - ``name``
     - String; **required**
     - Request alias, matched ASCII case-insensitively.
   * - ``file``
     - Path; **required**
     - Real file, or a path with a ``*``/``?`` filename mask.
   * - ``password``
     - String; ``""``
     - Optional alias password; ASCII case-insensitive.

.. important::
   ``freq.path.path`` and ``freq.magic.file`` are used directly by filesystem
   operations. Board path resolution does **not** root these nested fields.
   Relative values therefore depend on the mailer process working directory.
   Prefer absolute paths for offers, especially under an external scheduler.

Only requests attributable to a configured link are served. Request names
may contain ``*``/``?`` masks, are at most 100 characters, cannot begin with a
dot, and cannot contain path separators, colon or NUL. Alias configuration
does not itself enforce those request-name limits, so choose requestable
names. In the current implementation the session counter resets for each
processed request file; daily counters aggregate per node for the local
calendar date. Do not assume the session limit aggregates several separate
request files in one network connection.

Complete FTN example
~~~~~~~~~~~~~~~~~~~~

This example spells out every FTN field, including normally omitted defaults.
Replace assigned addresses, hosts, secrets, and absolute FREQ paths. A path
string is valid configuration even when the referenced file does not yet exist;
actual processing still requires the appropriate files and permissions.

.. code-block:: toml

   inbound = "ftn/inbound"
   outbound = "ftn/outbound"
   netmail = "ftn/netmail"
   bad_netmail = "ftn/badmail"
   bad_packets = "ftn/badpkt"
   nodelist = ""
   new_areas = "ftn/areas"
   new_file_areas = "ftn/files"
   origin = "Example BBS * bbs.example.org"

   [options]
   enabled = true
   process_in = true
   process_out = true
   process_orphan = false
   dial_out = true
   import_after_xfer = true
   check_dupe_msg_id = true
   check_dupe_path = false
   msgs_to_track = 0
   secure = true
   sysop_change = true
   auto_add = false
   auto_add_conference = 0
   auto_add_files = false
   auto_add_file_conference = 0
   pass_thru = false
   enable_routing = true
   route_echo_mail = false
   re_address = false
   make_response = false
   area_fix_forwarding = false
   auto_add_passthru = false
   default_zone = 0
   default_net = 0
   log_level = "normal"

   [freq]
   enabled = false
   deny = ["2:240/999"]

   [freq.limits]
   session_bytes = 10485760
   daily_bytes = 52428800

   [[freq.path]]
   path = "/srv/icyboard/public"
   password = ""

   [[freq.magic]]
   name = "INFO"
   file = "/srv/icyboard/public/info.zip"
   password = ""

   [[aka]]
   address = "2:240/100"
   domain = "fidonet"

   [[link]]
   address = "2:240/200"
   domain = "fidonet"
   host = "hub.example.org"
   port = 24554
   password = "replace-me"
   packet_password = "PACKET01"
   area_fix_password = "replace-me"
   tic_password = "replace-me"
   poll_minutes = 0
   areas = ["EXAMPLE.GENERAL"]

   [[route]]
   destination = "2:240/300"
   via = "2:240/200"

Local message and file-area mappings
--------------------------------------------------------------------------------

FTN mappings live in the ordinary conference lists, not in an FTN ``[[area]]``
table. The conference's ``area_file`` selects its message list and ``dir_file``
its file-directory list. Both list roots use **``[[area]]``**, and both require
the ``area`` array to exist on deserialization (``area = []`` is an empty list).
Runtime ``number`` and ``valid`` fields are skipped, not persisted settings.
QWKnet and ZCONNECT instead keep their remote-to-JAM mappings per hub/link in
their network file; the same JAM path should be exposed to callers through
the ordinary message-area list if desired.

.. list-table:: All persisted fields in each message ``[[area]]``
   :header-rows: 1
   :widths: 28 22 50

   * - Key
     - Type; omitted value
     - Meaning
   * - ``name``
     - String; **required**
     - Local display name.
   * - ``path``
     - Path; **required**
     - Local JAM base prefix, relative to the board root.
   * - ``is_read_only``
     - Boolean; **required**
     - Local message-entry restriction, separate from network export restrictions.
   * - ``allow_aliases``
     - Boolean; **required**
     - Permit local aliases.
   * - ``qwk_name``
     - String; ``""``
     - Local caller QWK display name, not a QWKnet hub ID.
   * - ``qwk_conference_number``
     - ``u16``; ``0``
     - Local caller QWK conference number; zero requests automatic allocation. Not the hub mapping.
   * - ``ftn_area_tag``
     - String; ``""``
     - FTN echo tag; empty excludes the area from echomail.
   * - ``ftn_origin``
     - String; ``""``
     - Nonempty overrides board-wide FTN origin text.
   * - ``req_level_to_enter``
     - Security-expression string; ``"0"``
     - Local entry access requirement.
   * - ``req_level_to_list``
     - Security-expression string; ``"0"``
     - Local listing access requirement.
   * - ``req_level_to_save_attach``
     - Security-expression string; ``"0"``
     - Local attachment-save access requirement.

The message-area constructor uses empty strings/paths and false booleans, but
the four marked fields are nevertheless required in TOML. Security expressions
are parsed strings, not integers or nested tables; ``""`` and ``"0"`` both
represent the default level-zero expression. For example, ``"100"`` sets a
level requirement. Invalid expressions can fail deserialization through these
fields' ``DisplayFromStr`` adapters.

.. code-block:: toml

   [[area]]
   name = "Example General"
   path = "conferences/network/general"
   is_read_only = false
   allow_aliases = false
   qwk_name = "General"
   qwk_conference_number = 0
   ftn_area_tag = "EXAMPLE.GENERAL"
   ftn_origin = "Example BBS"
   req_level_to_enter = "0"
   req_level_to_list = "0"
   req_level_to_save_attach = "0"

.. list-table:: All persisted fields in each file-directory ``[[area]]``
   :header-rows: 1
   :widths: 28 22 50

   * - Key
     - Type; omitted value
     - Meaning
   * - ``name``
     - String; **required**
     - Local display name; also a case-insensitive fallback TIC area match.
   * - ``path``
     - Path; **required**
     - Files directory, relative to board root.
   * - ``metadata_path``
     - Path; ``""``
     - File-index prefix; explicitly set it for a manually configured network directory.
   * - ``password``
     - Password string; **required**
     - Local directory access password, not the link's TIC password.
   * - ``sort_order``
     - String enum; ``"FileName"``
     - Exactly ``"NoSort"``, ``"FileName"`` or ``"FileDate"``.
   * - ``sort_direction``
     - String enum; ``"Ascending"``
     - Exactly ``"Ascending"`` or ``"Descending"``.
   * - ``ftn_area_tag``
     - String; ``""``
     - TIC ``Area`` match, preferred over the directory-name fallback.
   * - ``has_new_files``
     - Boolean; ``false``
     - Persisted new-files indicator.
   * - ``is_free``
     - Boolean; ``false``
     - Local free-download policy.
   * - ``list_security``
     - Security-expression string; ``"0"``
     - Listing requirement.
   * - ``download_security``
     - Security-expression string; ``"0"``
     - Download requirement.

Password values deserialize as strings: ``bcrypt:`` selects a bcrypt hash,
``$argon2`` selects an Argon2 hash, otherwise the value is plaintext (an inner
pair of double quotes is stripped). Saving plaintext adds that inner pair;
``password = '""'`` is the saved empty-password representation. The legacy
``password = ""`` is also accepted. There are no password-enum tables.

.. code-block:: toml

   [[area]]
   name = "EXAMPLE_FILES"
   path = "files/network"
   metadata_path = "files/network/dir"
   password = '""'
   sort_order = "FileName"
   sort_direction = "Ascending"
   ftn_area_tag = "EXAMPLE_FILES"
   has_new_files = false
   is_free = false
   list_security = "0"
   download_security = "0"

TIC tags match without ASCII case. Unknown file tags normally retain both TIC
and payload in inbound for retry; validated auto-add is optional. Setting a
message/file tag does not subscribe an uplink automatically.

QWKnet
------

.. list-table:: QWKnet file root
   :header-rows: 1
   :widths: 24 24 52

   * - Key
     - Type; omitted value
     - Meaning
   * - ``enabled``
     - Boolean; ``false``
     - Required true by QWKnet mailer commands.
   * - ``local_id``
     - String; ``""``
     - Local network ID; a valid identity is required by semantic validation.
   * - ``inbound``
     - Path; ``"qwknet/inbound"``
     - Received QWK archives.
   * - ``outbound``
     - Path; ``"qwknet/outbound"``
     - REP packets and scan checkpoints.
   * - ``hub``
     - Array of tables; ``[]``
     - Remote hubs, each with its own conference mapping.

Unlike deserializing an empty TOML file, ``QwkNetworkConfig::default()`` uses
empty inbound/outbound paths because it derives Rust ``Default``. The serde
path-default functions run only when parsing omitted fields. A missing board
file selection or failed QWKnet load uses that constructor, not the TOML path
defaults.

.. list-table:: Each ``[[hub]]``
   :header-rows: 1
   :widths: 24 24 52

   * - Key
     - Type; omitted value
     - Meaning
   * - ``id``
     - String; **required**
     - Remote hub QWK ID, also used in packet/checkpoint filenames.
   * - ``host``
     - String; ``""``
     - HTTP(S) base URL or host; required nonempty for polling, not offline scan/toss.
   * - ``username``
     - String; ``""``
     - HTTP Basic-auth username; empty uses ``local_id`` at runtime.
   * - ``password``
     - String; ``""``
     - Cleartext-stored HTTP Basic-auth password.
   * - ``poll_minutes``
     - ``u32``; ``0``
     - Stored interval in minutes; no automatic scheduler consumes it.
   * - ``area``
     - Array of tables; ``[]``
     - Per-hub remote conference mappings.

.. list-table:: Each ``[[hub.area]]``
   :header-rows: 1
   :widths: 24 24 52

   * - Key
     - Type; omitted value
     - Meaning
   * - ``remote_conference``
     - ``u16``; **required**
     - Hub packet conference number; validator permits 1--65,535 only.
   * - ``local_area``
     - Path; **required**
     - Local JAM prefix relative to board root.
   * - ``read_only``
     - Boolean; ``false``
     - Suppress export to this mapping, not import or local message entry.

Validation requires both local and hub IDs to have 2--8 ASCII bytes, begin
with a letter, and contain only letters, digits, underscore or hyphen.
``SYSOP`` and ``NETMAIL`` are reserved, without ASCII case. Hub IDs must be
unique without ASCII case; remote conference numbers must be unique within
each hub. Conference zero is reserved for unsupported NetMail. There is no
check that a hub differs from ``local_id``, no per-hub duplicate-JAM-path check,
and no spool path safety/distinctness check in this validator. Choose distinct
spools and verify every mapping yourself. Selected-hub commands fail if no hub
matches, even though an empty hub list passes configuration validation.

Polling appends ``/qwk.ssjs`` to ``host`` after removing trailing slashes; do
not include that script in ``host``. A value starting exactly with ``http://``
or ``https://`` keeps its scheme; otherwise ``https://`` is prepended. Other
schemes are not supported. A numeric TCP port belongs in the URL, not a
separate TOML key. Prefer HTTPS: HTTP exposes the Basic-auth credentials.

.. code-block:: toml

   enabled = true
   local_id = "MYBBS"
   inbound = "qwknet/inbound"
   outbound = "qwknet/outbound"

   [[hub]]
   id = "HUB01"
   host = "https://hub.example.org"
   username = "MYBBS"
   password = "replace-with-network-account-password"
   poll_minutes = 0

   [[hub.area]]
   remote_conference = 2001
   local_area = "conferences/qwknet/general"
   read_only = false

   [[hub.area]]
   remote_conference = 2030
   local_area = "conferences/qwknet/announcements"
   read_only = true

This is public conference mail, not routed NetMail. Configure the peer to send
classic message-path, message-ID and reply-ID kludges. ``HEADERS.DAT``,
``VOTING.DAT``, attachments, control messages and FTP transport are not
implemented. Per-hub numbers above are examples, not an authoritative network
area list. ``qwk-poll`` scans, uploads/downloads and then tosses; ``qwk-scan``
and ``qwk-toss`` expose individual stages. All take the board configuration,
not the standalone network file. Scheduling is external or through
:doc:`events` using maintenance execution.

Caller QWK settings are separate
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

The board's optional ``[qwk_settings]`` table configures caller QWK capture,
not QWKnet identity or transport. Omitting the whole table supplies its Rust
default. In a present table the first eight fields below are required, even
though the constructor initializes them to empty strings/paths.

.. list-table:: All persisted caller QWK settings
   :header-rows: 1

   * - Key
     - Type; omitted value in a present table
     - Meaning
   * - ``bbs_name``
     - String; **required**
     - Packet board name.
   * - ``bbs_city_and_state``
     - String; **required**
     - Packet location.
   * - ``bbs_phone_number``
     - String; **required**
     - Packet contact number.
   * - ``bbs_sysop_name``
     - String; **required**
     - Packet operator name.
   * - ``bbs_id``
     - String; **required**
     - Caller QWK board ID; not ``QwkNetworkConfig.local_id``.
   * - ``welcome_screen``
     - Path; **required**
     - Welcome file to include.
   * - ``goodbye_screen``
     - Path; **required**
     - Goodbye file to include.
   * - ``news_sceen``
     - Path; **required**
     - News file; the persisted spelling really is ``news_sceen``.
   * - ``max_msgs``
     - ``u16``; ``600``
     - System-wide ceiling for captured messages.
   * - ``max_msgs_per_conf``
     - ``u16``; ``200``
     - Per-conference capture ceiling.

Merge into the board file, not the QWKnet file:

.. code-block:: toml

   [qwk_settings]
   bbs_name = "Example BBS"
   bbs_city_and_state = "Example City"
   bbs_phone_number = ""
   bbs_sysop_name = "Sysop"
   bbs_id = "MYBBS"
   welcome_screen = ""
   goodbye_screen = ""
   news_sceen = ""
   max_msgs = 600
   max_msgs_per_conf = 200

ZCONNECT
--------

Both the root configuration and each link use struct-level serde defaults.
All their keys may be omitted during parsing; semantic validation can still
reject the resulting values. Area records do not use struct-level defaults.

.. list-table:: ZCONNECT file root
   :header-rows: 1
   :widths: 24 24 52

   * - Key
     - Type; omitted value
     - Meaning
   * - ``enabled``
     - Boolean; ``false``
     - Must be true for mail processing.
   * - ``local_system``
     - String; ``""``
     - Fully qualified message-address domain of this board.
   * - ``local_user``
     - String; ``"sysop"``
     - Safe address localpart used when a local author cannot serve as one; also protocol sysop identity.
   * - ``inbound``
     - Path; ``"zconnect/inbound"``
     - Received archives, under one subdirectory per link ID.
   * - ``outbound``
     - Path; ``"zconnect/outbound"``
     - Pending archives, checkpoints and locks, under one subdirectory per link ID.
   * - ``link``
     - Array of tables; ``[]``
     - Remote peers and their board mappings.

.. list-table:: Each ``[[link]]``
   :header-rows: 1
   :widths: 24 24 52

   * - Key
     - Type; omitted value
     - Meaning
   * - ``id``
     - String; ``""``
     - Local unique spool/selection ID; empty fails semantic validation.
   * - ``host``
     - String; ``""``
     - Dial host, not a URL; empty permits offline-only exchange.
   * - ``remote_system``
     - String; ``""``
     - Optional expected peer ``SYS`` display name, not necessarily the dial host.
   * - ``port``
     - ``u16``; ``23``
     - TCP Telnet port; validator requires 1--65,535.
   * - ``username``
     - String; ``""``
     - Our account/system name at the peer; empty uses ``local_system`` at runtime.
   * - ``password``
     - String; ``""``
     - Cleartext-stored link secret. Online profile has stricter limits below.
   * - ``login``
     - String; ``"zconnect"``
     - Validator accepts exactly ``"zconnect"``, ``"janus"`` or ``"direct"``.
   * - ``timeout_secs``
     - ``u32``; ``60``
     - Connection/protocol timeout in seconds; validator permits 1--3,600.
   * - ``area``
     - Array of tables; ``[]``
     - Public remote-board mappings.

``login`` is stored as a string, not a Rust enum in the config. Other strings
can deserialize, but fail validation. There is **no** ``poll_minutes`` field
or built-in ZCONNECT polling schedule. ``zconnect`` uses standard dispatch,
``janus`` the JANUS dialogue, and ``direct`` skips initial login for a peer
already in protocol mode; direct mode still authenticates during negotiation.
``username`` supplies our negotiation ``SYS`` and JANUS Systemname. The exact
configured timeout is used, not a 30-second clamp, although separate finite
login/transfer deadlines and bounded retries also apply.

.. list-table:: Each ``[[link.area]]``
   :header-rows: 1
   :widths: 24 24 52

   * - Key
     - Type; omitted value
     - Meaning
   * - ``remote_board``
     - String; **required**
     - Public board path such as ``/PUBLIC/GENERAL``.
   * - ``local_area``
     - Path; **required**
     - JAM base prefix relative to board root.
   * - ``read_only``
     - Boolean; ``false``
     - Suppress export for this mapping, not import or local entry.

Exact semantic validation rules
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

* ``local_system`` must be an FQDN whenever it is nonempty, networking is
  enabled, or any links exist. The sole empty-identity exception is a disabled
  config with no links. FQDN here means at most 253 bytes, at least one dot,
  with every label 1--63 ASCII letters/digits/hyphens, beginning and ending in
  a letter/digit. A trailing dot, empty label or underscore is invalid. This
  validates spelling, not DNS existence or ownership.
* ``local_user`` is 1--128 bytes, each ASCII code 33 through 124 inclusive,
  excluding ``@ < > / \ ( ) [ ] { } ' ` " , ! %``. Spaces and non-ASCII text
  are not allowed.
* Link IDs are 1--64 ASCII bytes, start with a letter/digit, and thereafter
  contain only letters, digits, underscores or hyphens. IDs are unique without
  ASCII case. Preserve configured spelling for spool-directory names.
* A nonempty ``host`` must be an FQDN as above, a single DNS label with those
  label rules, or a parsed IPv4/IPv6 address. Use an unbracketed IPv6 value;
  port is separate. Schemes, paths and ``host:port`` are not accepted as hosts.
* ``remote_system`` is 0--255 printable ASCII bytes (32--126). Nonempty values
  are compared to the peer's ``SYS`` without ASCII case. Empty disables only
  this comparison: the peer must still send a nonempty ``SYS``. It is a
  display-name check, not cryptographic authentication.
* Configuration validation allows ``username`` and ``password`` to be empty
  or up to 1,024 printable ASCII bytes. **Online** login/negotiation requires
  a nonempty password of at most **10** bytes and nonempty identities of at
  most **255** printable ASCII bytes. Thus a long username or an empty/long
  password can pass config validation and still fail a call. The same password
  bound applies to direct negotiation.
* ``remote_board`` is at most 1,024 bytes, begins with ``/``, does not end in
  ``/`` or contain ``//``, and contains only uppercase ASCII letters, digits,
  slash, underscore, exclamation mark, plus and hyphen. Board names are unique
  within a link without ASCII case. Unknown boards are not auto-created.
* Inbound, outbound and local-area paths must be nonempty UTF-8 strings with
  no controls or backslashes, contain a normal path component, and contain no
  parent/current-directory or platform-prefix components as exposed by Rust
  path iteration. Absolute paths are allowed. Avoid ``.`` segments even
  where platform normalization might remove them; ``..`` is rejected.
* Inbound and outbound must differ as path values. Within each link, local
  JAM paths must be unique after replacing their extension with ``jhr`` and
  lowercasing the result. Thus ``general.foo`` and ``general.bar`` conflict.
  These checks do not canonicalize symlinks or establish cross-link uniqueness;
  do not use aliases or overlapping spool trees.

Complete ZCONNECT example
~~~~~~~~~~~~~~~~~~~~~~~~~

.. code-block:: toml

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
   username = "bbs.example.org"
   password = "CHANGE-ME"
   login = "zconnect"
   timeout_secs = 60

   [[link.area]]
   remote_board = "/PUBLIC/GENERAL"
   local_area = "conferences/zconnect/general"
   read_only = false

   [[link.area]]
   remote_board = "/PUBLIC/ANNOUNCEMENTS"
   local_area = "conferences/zconnect/announcements"
   read_only = true

Use a password agreed with the peer, not the example value. Telnet exposes
credentials and mail; use a trusted network or separately secured tunnel.
This implementation has synthetic protocol tests, not verified live external
interoperability. Test with the peer before unattended deployment.

``zconnect-poll`` scans, exchanges and tosses completed inbound archives;
``zconnect-scan``, ``zconnect-toss`` and ``zconnect-ack`` support external
mailers. Commands take the board configuration and optionally select a link;
acknowledgement requires exactly one explicit link and verified remote
delivery. Never acknowledge simply because a copy/upload finished. No inbound
listener, private-mail delivery, attachment gateway, arbitrary login scripting
or automatic polling is enabled by this file.

Generated TOML state (not configuration)
----------------------------------------

These formats are persisted by networking operations. Preserve their keys and
pending-delivery state; do not delete checkpoints to make a stalled queue
appear empty. Their tables do not belong in the network configuration file.

FTN request usage
~~~~~~~~~~~~~~~~~

The outbound root's ``freq_usage.toml`` holds ``day`` (required string, written
as local ``YYYY-MM-DD``) and ``nodes`` (string-to-``u64`` map, default empty).
Map keys are formatted FTN addresses and values are served bytes. Missing,
unreadable, unparsable or different-day usage is treated as a fresh day by the
service; this is not a tamper-proof quota ledger. Example shape:

.. code-block:: toml

   day = "2026-09-07"

   [nodes]
   "2:240/200" = 4096

QWKnet scan state
~~~~~~~~~~~~~~~~~

The outbound root holds ``<hub-id>.rep`` and ``<hub-id>.state.toml``. The state
has one optional ``conferences`` string-to-``u32`` map (default empty). Keys
are decimal remote conference numbers, **not local paths**; values are scan
positions. Existing REP packets are reused until accepted, rather than
overwritten by the next scan. A missing state file starts with empty state;
an existing malformed state file causes an error.

.. code-block:: toml

   [conferences]
   "2001" = 125
   "2030" = 8

ZCONNECT delivery checkpoint
~~~~~~~~~~~~~~~~~~~~~~~~~~~~

Each ``<outbound>/<link-id>/scan.toml`` contains:

.. list-table:: Checkpoint fields
   :header-rows: 1

   * - Key
     - Type; omitted value
     - Meaning
   * - ``committed``
     - String-to-``u32`` map; empty
     - Acknowledged scan positions keyed by uppercase remote board path.
   * - ``pending``
     - Optional table; absent
     - Prepared, not yet acknowledged delivery checkpoint.
   * - ``pending.next``
     - String-to-``u32`` map; **required in pending**
     - Positions to commit after verified receipt.
   * - ``pending.messages``
     - ``usize``; **required in pending**
     - Number of messages in the pending archive.
   * - ``pending.sha256``
     - String; **required in pending**
     - Lowercase hexadecimal SHA-256 of the exact archive bytes.
   * - ``retired``
     - Optional string; absent
     - Hash of acknowledged archive whose filesystem retirement needs completion.

Omit absent optional fields; TOML has no null. ``pending`` and ``retired``
must not coexist at recovery. Archive/checkpoint hash mismatches fail closed.
The checkpoint is paired with ``mail.zip`` (or crash-recovery ``prepared.zip``);
keep them together. Missing checkpoint plus an existing ``mail.zip`` is an
error, not permission to rescan. Example pending state, using a shape-only hash
that must be replaced by the actual archive digest:

.. code-block:: toml

   [committed]
   "/PUBLIC/GENERAL" = 125

   [pending]
   messages = 1
   sha256 = "0000000000000000000000000000000000000000000000000000000000000000"

   [pending.next]
   "/PUBLIC/GENERAL" = 126

``operation.lock`` and ``poll.lock`` are lock files, not TOML. Do not unlink
them to bypass active operations. Received ZIPs go directly under
``<inbound>/<link-id>/``; completed supported archives move to ``processed/``;
unsupported/unmapped archives move intact to ``retained/`` for review.
Malformed/import-failed archives remain in inbound. These are runtime
subdirectories, not additional configurable fields.

Validation and deployment checklist
-----------------------------------

* Use singular array/table names and the exact enum spelling. Put root scalar
  keys before nested table headers so they do not accidentally become fields
  of the last table. There is no automatic warning for most network typos.
* Supply FTN ``inbound``/``outbound``, link ``host`` even when blank, all
  required address/mapping fields, and both limits if ``[freq.limits]`` exists.
* Parsing does not validate reachability, writable spools, assigned addresses,
  existing local bases, conference registration or successful authentication.
  Perform controlled scan/toss and peer tests after configuration validation.
* The two stored ``poll_minutes`` fields do not run timers. Use a maintenance
  event or carefully coordinated offline scheduling; do not run mailer writes
  concurrently with the live board merely because a config parses.
* Keep network export ``read_only`` separate from local caller security, and
  verify mappings against the peer's actual area list before importing mail.

The format above follows the engine's FTN (including FREQ), QWKnet, ZCONNECT,
message-area, file-directory and board-QWK serde definitions, plus the mailer
and protocol runtime checks. See :doc:`../mailer` for the FTN workflow and
:doc:`events` for timed maintenance execution.