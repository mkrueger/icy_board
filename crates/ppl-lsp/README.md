# PPL for VS Code

Editor support for the PCBoard Programming Language: syntax highlighting,
completion, signature help, hover, document symbols and formatting for `.pps`,
`.ppd` and `.ppx` files. Errors and warnings come from the same compiler that
`pplc` uses.

## Requirements

None, if you took the package for your platform: the `ppl-language-server`
binary comes with it. The package without a platform in its name does not carry
a server, so it needs one on your `PATH`. Either way, `ppl.serverPath` points
the extension at a server you built yourself.

## Building this extension

```
pnpm i
npm run compile
pnpm run package
```


