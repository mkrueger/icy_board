Icy Board Reference
===================

The handbook for running, customizing and programming an Icy Board BBS.
This edition describes version |icy_version| from the checkout used to build
it. Development builds can differ from a released binary; check
``icboard --version`` and read the compatibility notes before migrating a board.

**New sysop?** Start with :doc:`/docs/gettingstarted`.
**Coming from PCBoard?** Read :doc:`/docs/migration`.
**Looking something up?** Use :doc:`/docs/reference/index` or :ref:`search`.

.. note::

   This is an Icy Board manual, not a promise that every PCBoard feature works.
   Command help documents unavailable commands as well as supported ones.
   :doc:`/docs/known_limitations` records restrictions and incomplete behavior.

.. toctree::
   :maxdepth: 1
   :caption: Start here

   introduction
   /INSTALL
   /docs/gettingstarted
   /docs/reference/concepts
   /docs/migration

.. toctree::
   :maxdepth: 1
   :caption: Operate a board

   /docs/reference/administration
   icy_board
   /docs/reference/tools
   /docs/icbfile
   /docs/upload_processing
   /docs/accounting
   /docs/events
   /docs/reference/troubleshooting

.. toctree::
   :maxdepth: 1
   :caption: Reference and customization

   /docs/reference/index
   /docs/reference/commands
   /docs/configuration/README
   customizing/display_files
   /docs/macros
   /docs/mkicbmnu
   /docs/reference/customization

.. toctree::
   :maxdepth: 1
   :caption: Networking and doors

   mailer
   /docs/qwknet
   /docs/zconnect
   /docs/native_dos_doors

.. toctree::
   :maxdepth: 1
   :caption: PPL and PPE programming

   /docs/ppl
   /docs/pplc
   ppl/ppl
   /docs/new_ppl
   /docs/ppe_format

.. toctree::
   :maxdepth: 1
   :caption: Compatibility and contributing

   /docs/differences
   /docs/known_limitations
   /docs/feature_parity
   /docs/roadmap
   /docs/reference/documentation

Offline use
-----------

The complete HTML directory can be copied to another machine and opened without
a running board or web server. Search is local to the manual. The PDF edition
is built from the same sources. See :doc:`/docs/reference/documentation` for
build instructions and the distinction between development and release editions.
