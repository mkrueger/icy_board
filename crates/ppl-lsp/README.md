# PPL for VS Code

Editor support for the PCBoard Programming Language: syntax highlighting,
completion, signature help, hover, document symbols and formatting for `.pps`,
`.ppd` and `.ppx` files. Errors and warnings come from the same compiler that
`pplc` uses.

## Requirements

The extension talks to the `ppl-language-server` binary, which ships with the
Icy Board tools. Put it on your `PATH`, or point the setting `ppl.serverPath`
at it.

## Building this extension

```
pnpm i
npm run compile
pnpm run package
```


