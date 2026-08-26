The Preprocessor
================

The preprocessor is not tied to a language version - it works whatever
``--lang-version`` is set to. Its directives are written as ``;``-comments so
that a source using them still reads as a comment to any older tool.

The language of a source
------------------------

=========================  ====================================================
Directive                  Meaning
=========================  ====================================================
``;$LANGVERSION number``   The language version the file is written in
=========================  ====================================================

A source states which language it is written in, so it wins over
``language_version`` in ``ppl.toml``, ``pplc --lang-version`` and
``PPL_LANG_VERSION``. That is not a preference but a fact: a file that uses
``BEGIN`` as a block cannot be read as 3.50, where ``begin`` may still be a
variable name.

.. code-block:: PPL

    ;$LANGVERSION 400

    BEGIN
        PrintLn "Hello"
    END

Nothing but comments and blank lines may come before it, because it decides
which words are keywords for everything that follows. For the same reason it is
read before the preprocessor runs, so it cannot stand in a ``;$IF`` branch, and
a file may only carry one. An unknown version number is an error.

Two files of one package may not declare different versions. ``ppld`` writes the
directive into the source it produces, which is what makes a decompiled PPE
compile again without an option.

Conditional compilation
-----------------------

====================  =========================================================
Directive             Meaning
====================  =========================================================
``;$DEFINE name``     Defines a preprocessor variable, optionally with a value
``;$IF expr``         Opens a block that is compiled only if ``expr`` is true
``;$ELSEIF expr``     Closes the preceding block and opens a new one
``;$ELIF expr``       Same as ``;$ELSEIF``
``;$ELSE``            Opens a block for the case where no branch was taken
``;$ENDIF``           Closes the preceding conditional block
====================  =========================================================

Directive names are case insensitive. Blocks nest, and text in a branch that is
not taken is never lexed, so it does not have to be valid PPL. Only the first
branch whose condition is true is compiled.

An ``;$IF`` left open, or an ``;$ELSE``, ``;$ELSEIF`` or ``;$ENDIF`` without a
matching ``;$IF``, is an error. A ``;$`` word that is not a directive is treated
as an ordinary comment.

Predefined variables
--------------------

===================  =============  ============================================
Name                 Type           Value
===================  =============  ============================================
``VERSION``          ``STRING``     The ``version`` field from ``ppl.toml``
``RUNTIME``          ``INTEGER``    The PPE runtime version being written
``LANGVERSION``      ``INTEGER``    The language version being compiled against,
                                    ``;$LANGVERSION`` included
===================  =============  ============================================

More can be added with ``;$DEFINE`` or with ``pplc --defines "A=1;B=2"``.

Because ``VERSION`` is a string, version comparisons want ``RUNTIME`` or
``LANGVERSION``:

.. code-block:: PPL

    ;$IF RUNTIME <= 340
        PrintLn "World"
    ;$ELSEIF RUNTIME < 200
        PrintLn "Old World"
    ;$ELSE
        PrintLn "New World"
    ;$ENDIF

This is how one source serves several boards: the branch for the old runtime may
use only what that runtime has, and the compiler never sees the other branches.

Substitution tokens
-------------------

``;#NAME`` is replaced by the value of the preprocessor variable ``NAME``, and a
name that was never defined is an error:

.. code-block:: PPL

    PrintLn "Version:", ;#Version
    PrintLn "Runtime:", ;#Runtime
    PrintLn "Language:", ;#LangVersion

Would print::

    Version:0.1.0
    Runtime:400
    Language:400
