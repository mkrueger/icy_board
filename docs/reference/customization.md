# Customizing the caller experience

Icy Board separates presentation from board data and command dispatch.
Customize one layer at a time and keep a working normal-caller account for
testing. The sysop's security level can hide an access problem.

## Choose the correct layer

| Change | Edit / tool |
| --- | --- |
| Welcome, news, conference introduction or artwork | Configured display file |
| A system question or response | ICBTEXT using `mkicbtxt` |
| Menu appearance and action sequences | Menu using `mkicbmnu` |
| Command keyword or override | Main/conference command list |
| Caller help text | Exported help Markdown and `icbsetup genhelp` |
| Dynamic values in a display | `@` macros |
| New interactive behavior | PPL source compiled to PPE, or a configured door |
| Setup-tool color scheme | Board configuration-screen palette, not caller artwork |

## Display files and variants

Configure the base filename and let the display resolver choose graphics,
language and security variants. Keep the
[filename conventions](../source/customizing/display_files.rst) beside you
when adding variants: a more specific file may take precedence over the one
you edited.

Legacy display content without a BOM is interpreted as CP437. A UTF-8 display
file must carry a UTF-8 BOM. Do not infer the runtime encoding solely from
how an editor displays the file.

The [macro reference](../macros.md) covers values, colors, widths, alignment,
trimming, screen control and recognized-but-unimplemented codes. Macros are
context-sensitive: a value meaningful during a transfer may not exist on the
welcome screen.

## Menus and command lists

Use [mkicbmnu](../mkicbmnu.md) to build menus, inspect actions and preview
rendering. Its preview does not execute actions or prove all permissions.

Use [command lists](../configuration/commands.md) to add a new command, override
a built-in, run a PPE or chain supported actions. A menu file and a command
list have different root schemas; neither is a renamed PCBoard binary file.

When adding a command:

1. Choose its keyword and access policy.
2. Decide whether it belongs globally or only to one conference.
3. Configure the action and any required parameter or file.
4. Add menu text and help appropriate to that command.
5. Check the menu, then test dispatch and denied access in real sessions.

## Prompts and languages

`mkicbtxt` edits numbered ICBTEXT records, not artwork or command-help files.
Preserve the meaning of record substitutions when rewording prompts.
See [the text format](../configuration/data.md) and
[text-editor options](tools.md#mkicbtxt-system-prompts).

Language definitions include the display suffix, locale, yes/no characters and
date formats. Consult [language configuration](../configuration/components.md#languages)
rather than assuming the file suffix is the locale code.

For English and German installations, test both explicitly:

- Login and choose each configured language.
- Check yes/no prompts and date entry.
- Inspect graphics and plain-text variants.
- Verify fallback when a translated file is absent.
- Test at the terminal dimensions your callers actually use.

The built-in generated help catalog is English. `genhelp --language ger`
only names the output files; provide translated sources as well.
Do not mistake the legacy German source material for a reviewed, shipped
translation of the full current catalog.

## Help generation

Use the [complete help workflow](../gettingstarted.md#command-help) to export,
edit, validate and install topics. This manual and the built-in terminal help
share those source topics, but website rendering does not replace the
generator's terminal-width and encoding checks.

Keep custom source files and themes under your own version control, excluding
credentials and live board data. Preserve the ownership ledger and review
dry-run reports before adopting or replacing generated files.

## PPEs and external doors

Use [PPL and PPEs](../ppl.md) for scripted behavior. Decide the language and
runtime target explicitly for code intended to remain usable on PCBoard.
The [new-language reference](../new_ppl.md) documents Icy Board-only APIs.

External programs use [door definitions](../configuration/components.md#doors-and-bbslink-accounts).
DOS doors require the [native DOS-door workflow](../native_dos_doors.md);
merely putting a DOS executable in a directory does not make it a native PPE.

## Validation checklist

- Run configuration and menu checks.
- Test both permitted and denied actions.
- Test normal callers, not only the sysop.
- Test English and configured translations.
- Test 80x25 and any other supported caller geometry.
- Verify CP437 and BOM-marked UTF-8 content as applicable.
- Inspect logs after PPE, display or protocol failures.
- Preserve the previous working files until a real local call succeeds.
