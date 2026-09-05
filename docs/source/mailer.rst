FidoNet Mailer
==============

Icy Board can take part in a fidonet technology network: it fetches mail from
an uplink, writes it into the message bases, and packs what your users wrote
back into bundles for the next call. The work is done by ``icbmailer``, a
separate program that runs beside the board.

``icbmailer`` does not poll automatically and does not answer incoming calls.
Run it yourself, from the board's timed events, or from the operating system's
scheduler.

.. note::
   This chapter assumes you already have a node number from the coordinator of
   the network you are joining, and the host name, port and password of the
   system that feeds you.


How the pieces fit together
---------------------------

Mail travels as *bundles*: zip archives holding one or more *packets*, and a
packet holds the messages themselves. Three steps move mail through the board::

   scan   message bases  ->  bundles in the outbound
   poll   outbound       ->  uplink, and uplink -> inbound
   toss   bundles in the inbound  ->  message bases

Each step is a separate command, so you can run them in whatever order your
setup needs. The usual order is scan, poll, toss.

There are two kinds of mail. **Echomail** is public: it belongs to an area,
every system carrying that area gets a copy, and it lands in the message base
you tied to that area. **Netmail** is addressed to one person and normally
lands in one base for the sysop to read. With secure netmail enabled, mail for
an unknown recipient is kept in a separate base.


The configuration file
----------------------

The network configuration lives in its own file, named by ``ftn_file`` in
``icboard.toml``. A board created by ICBSetup already has an empty one at
``main/ftn.toml``. The mailer does nothing until at least one address and one
link are configured.

.. code-block:: toml

   inbound = "ftn/inbound"
   outbound = "ftn/outbound"
   netmail = "ftn/netmail"
   bad_netmail = "ftn/badmail"
   origin = "My Board * bbs.example.org"

   [options]
   enabled = true
   secure = false
   sysop_change = true
   log_level = "normal"

   [[aka]]
   address = "21:1/100"
   domain = "fsxnet"

   [[link]]
   address = "21:1/100"
   domain = "fsxnet"
   host = "agency.bbs.geek.nz"
   port = 24554
   password = "secret"
   areas = ["FSX_GEN", "FSX_BBS"]

Paths
~~~~~

``inbound``
   Where a session drops what it received. The tosser empties it.

``outbound``
   Where bundles wait for the next call. The scanner fills it, and it holds
   one directory per link, named after the address: ``21.1.100``.

``netmail``
   The message base arriving netmail is written to.

``bad_netmail``
   Where netmail from an unconfigured FTN node is written while
   ``options.secure`` is enabled. The packet's origin address must match a
   ``[[link]]`` address. Recipient names are not part of this check.

All three are relative to the board directory and are created for you.

``origin``
   The line appended to every echomail message written on this board. Custom
   in fidonet is your board name and how to reach it, and some networks insist
   on one, so set it before you scan for the first time.

Addresses
~~~~~~~~~

A fidonet address is ``zone:net/node``, with ``.point`` appended for a point
system. Every ``[[aka]]`` block is one address this board answers to; a board
in several networks has one per network.

``domain`` is the name of the network. Binkp sends it after an ``@``, which is
how a system reached under several addresses knows which network you mean.
Mail going to a link is stamped with the address whose domain matches that
link, so a board in two networks needs the domain filled in on both sides. The
first address is used when nothing matches.

Links
~~~~~

A ``[[link]]`` block is a system to exchange mail with: the uplink that feeds
you, or a downlink you feed.

``host``, ``port``
   Where to call. The port defaults to 24554, the binkp one.

``password``
   What the link expects to hear. Binkp either sends this in the clear or
   answers a challenge with it, so it cannot be stored hashed the way user
   passwords are. Keep the file readable by the board account only.

``packet_password``
   The eight characters a packet from this link has to carry, and the ones
   packets sent to it are stamped with. A packet claiming this link's address
   without them is left in the inbound rather than tossed. Leave it out and
   packets are taken as they come, which is what `PCBoard` did when the field
   on the ``~FIDO~`` user record was blank.

``areas``
   The echo tags this link carries. Mail written here is offered to a link
   only for the areas it asked for, and to every link that asked. Leave it
   empty and the link gets no echomail.

``area_fix_password``
   The password expected as the first word of AreaFix request subjects from
   this node. An empty value accepts an empty subject.

``poll_minutes``
   Reserved for a scheduler that does not exist yet. Zero, the default, means
   the link is called only when you ask for it.

Processing options
~~~~~~~~~~~~~~~~~~

``enabled``
   Master switch for Fido processing. Turning it off stops polling, importing
   and exporting without losing addresses, nodes or paths. Existing
   configurations which do not name the option default to enabled.

``log_level``
   ``normal`` reports warnings and errors, ``detailed`` also reports regular
   mailer activity, and ``debug`` includes packet and protocol details. The
   command-line ``-v`` switch temporarily selects debug for that run.

Routing
~~~~~~~

Exact destination routes name a configured node as their next hop::

   [[route]]
   destination = "2:240/6000"
   via = "2:240/5853"

``enable_routing`` moves incoming packets for a routed destination to that
node's outbound. ``re_address`` changes the routed packet header to the next
hop; without it the final destination remains in the header. With
``route_echo_mail`` the same route table is used for echomail exported to a
configured destination node.

AreaFix
~~~~~~~

Netmail to ``AREAFIX`` from a configured node accepts ``+TAG``, ``-TAG``,
``%+ALL``, ``%-ALL``, ``%LIST``, ``%QUERY`` and ``%HELP``. Successful changes
update that node's ``areas`` list in ``ftn.toml``. ``make_response`` sends a
result message back. ``auto_add_passthru`` allows an unknown tag to be added as
a passthru subscription while ``pass_thru`` is enabled. Otherwise,
``area_fix_forwarding`` passes an unknown subscription request to the first
other configured node, so place the uplink before downlinks.

Netmail options
~~~~~~~~~~~~~~~

``secure``
   Keep netmail from nodes not configured as links in ``bad_netmail``. This is
   the equivalent of PCBoard accepting secure netmail only from ``~FIDO~`` node
   records. Turn this off to accept netmail from every source. If a message is
   kept apart, the toss output shows its source address, destination base, and
   the settings that can fix it. The source address is only a claim the packet
   makes, so set ``packet_password`` on a link that matters.

``sysop_change``
   Change the conventional recipient ``Sysop`` to ``FIDO_SYSOP`` on import.
   This keeps generic Fido mail from setting the board sysop's mail-waiting
   flag, matching PCBoard. This is enabled by default.


Tying message areas to echos
----------------------------

An echo has a *tag*, an upper-case name like ``FSX_GEN`` that every system in
the network uses for it. Set ``Fido Area Tag`` on the message area that should
carry it, in the area editor of its conference, which writes ``ftn_area_tag``
into the ``area.toml``:

.. code-block:: toml

   [[area]]
   name = "General Chatter"
   path = "general"
   ftn_area_tag = "FSX_GEN"
   is_read_only = false
   allow_aliases = false

An area without a tag is one this board keeps to itself: nothing is sent out
of it, and nothing arrives in it.

Mail for a tag no board area claims is counted and reported by the tosser but
not stored, so a mistyped tag shows up as an unknown area rather than
disappearing.


The origin line
---------------

Every echomail message leaving this board carries an origin line naming the
board and the address it can be reached at. The board-wide text is the
``Default Origin`` under ``Message Networking > Fido Configuration``, written
as ``origin`` in ``ftn.toml``.

An area can say something else by naming its own ``Fido Origin`` in the area
editor, which is written as ``ftn_origin``:

.. code-block:: toml

   [[area]]
   name = "German Chatter"
   path = "german"
   ftn_area_tag = "FSX_GER"
   ftn_origin = "Icy Board, now in German (fsxnet.example)"

PCBoard kept these in ``ORIGINS.DAT``, where one origin named a range of
conferences such as ``1-200 203 250-100``. On import each of those conferences
hands its origin to all of its areas, so the same messages leave the board with
the same origin they did before.


Running the mailer
------------------

Every command takes the path to ``icboard.toml`` and understands ``-v`` for a
running commentary.

.. code-block:: shell

   icbmailer links icboard.toml
   icbmailer scan  icboard.toml
   icbmailer poll  icboard.toml [address]
   icbmailer toss  icboard.toml
   icbmailer show  <file>

``links``
   Lists the configured links and what is waiting in the outbound for each.
   Good for checking a fresh configuration without calling anybody.

``scan``
   Reads what was written on this board since the last run and packs it into
   bundles for the links carrying those areas. The first run of an area only
   notes how far it got, so old messages are not sent out to the network by
   surprise. The bookkeeping sits in ``scan.toml`` in the outbound; delete it
   and the next scan starts over from where the bases stand now.

``poll``
   Calls a link, hands over what is waiting for it and takes what it has.
   Without an address every link is called. ``-k`` leaves the delivered files
   in the outbound instead of deleting them, which is what you want while
   testing.

``toss``
   Unpacks everything in the inbound and writes it into the message bases.
   Messages carrying an id already seen in that area are dropped as
   duplicates; the same message reaching you over two paths is normal in
   fidonet. A file that cannot be read is left where it is and reported, so
   nothing is lost to a truncated download. When secure netmail is enabled,
   the summary also names unknown recipients whose mail was stored in the
   bad-netmail base.

``show``
   Prints what is inside a packet or a bundle. ``-t`` prints the message text
   as well. This one does not need a board.

A cron entry that keeps a node current looks like this::

   0 * * * * icbmailer scan /bbs/icboard.toml && icbmailer poll /bbs/icboard.toml && icbmailer toss /bbs/icboard.toml


Checking that it works
----------------------

Run ``icbmailer poll -v`` once by hand and read what it says. The session
should authenticate, name the files it sends and receives, and end without an
error. If the link rejects the password, it is either wrong or the link is
expecting a different address from you than the one you present.

After a poll, ``toss -v`` reports how many messages were imported, how many
were dropped as duplicates and which tags nobody claimed. Zero imported with a
list of unknown areas means the tags in ``area.toml`` do not match what the
uplink sends.
