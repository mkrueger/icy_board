Introduction
------------

What is Icy Board?
------------------

Icy Board is a modern, memory-safe re-implementation of the classic
PCBoard Bulletin Board System.

Written in Rust—aiming to preserve the original experience while
enabling secure, scriptable expansion on today's platforms
(Linux, macOS, Windows, ARM, etc.).

Unlike emulation layers that just “run the old EXE”, 
Icy Board rebuilds core subsystems: 
user base, conferences, message storage (JAM),
time/byte bank, accounting scaffolding, PPL execution, and TUI
administration—providing a foundation that is both compatible *and*
extensible.

Due to its inheritance of PCBoard's architecture, Icy Board aims to
provide a familiar experience for long-time users while introducing 
modern features and improvements.

Icy Board's Key features
~~~~~~~~~~~~~~~~~~~~~~~~

* Full PCBoard 15.4 compatibility (users, conferences, messages, PPE)
* Modern, memory-safe codebase (Rust)
* UTF-8 support (with legacy CP437 compatibility)
* Internet protocols: Telnet, SSH, WebSockets (Web Terminal)
* JAM message base (with QWK/QWKE support)
* PPE support (with modern extensions)
* PPL toolchain (compile, decompile, LSP support)
* Interactive TUI configuration (icbsetup)
* Modular architecture (plugins, extensions)
* Cross-platform (Linux, macOS, Windows, ARM)


Key Goals
~~~~~~~~~

* High compatibility with PCBoard 15.4 behaviors and PPE ecosystem
* Safe modernization: UTF-8, Internet protocols (Telnet, SSH, WebSockets)
* Preserve PCB / ANSI / Avatar / RIP aesthetics (nostalgia intact)
* Provide a fully featured PPL toolchain (compile, decompile, LSP)
* Make migration of legacy installations feasible
* Extend with new objects / APIs *without* breaking old PPE plugins

Non-Goals (by design)
~~~~~~~~~~~~~~~~~~~~~

* PCBoard was never simple; neither is IcyBoard
* Heavy GUI configuration (focus is terminal / SSH TUIs)
* Running on DOS / Windows 9x / OS/2

License
~~~~~~~

Dual-licensed (Apache 2.0 / MIT) — see repository LICENSE files.


Why Icy Board?
~~~~~~~~~~~~~~

PCBoard was a pioneering BBS software in the late 80s and 90s, known for its
robust feature set, extensive customization options, and a vibrant community of
users and developers. It introduced many innovations that became standard in
the BBS world, such as the PPL scripting language, which allowed sysops to
create custom functionalities and enhance user experience.

However, as technology advanced and the internet became the dominant
communication platform, traditional BBS systems like PCBoard faced obsolescence.
Many BBS software projects were abandoned, and their communities dwindled.

PCBoard itself was discontinued, and its last official version (15.4) was released in
the late 90s. Over the years there have been attempts to revive or emulate PCBoard,
but none have fully captured its essence or provided a sustainable path forward.

Icy Board aims to fill this gap by reimagining PCBoard for the modern era while 
preserving its core principles and functionalities. Modern BBS systems are lacking 
diversity most boards are just out of the box installations from major software packages.

These boards are great and have developed their own strengths but since they're so good 
at what they do they don't emphasize creativity and individuality.

Icy Board wants to bring back the individuality of BBSes. The issue is that not many people write
new BBS software these days. So Icy Board wants to be a platform that makes it easy to create
custom BBS experiences without requiring extensive programming knowledge.

PCBoards PPL language was a great way to customize the board. It's possible to run them on modern systems
and Icy Board is the platform for that. Out of the box Icy Board is crap - it'll scare people away. 
But with some PPL magic it can become a unique experience.

