The Preprocessor
================

The preprocessor is not tied to a language version - it works whatever
``--lang-version`` is set to. Its directives are written as ``;``-comments so
that a source using them still reads as a comment to any older tool.

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
``LANGVERSION``      ``INTEGER``    The language version being compiled against
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
    Runtime:401
    Language:400
