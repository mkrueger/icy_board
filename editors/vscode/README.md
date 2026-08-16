# IcyBoard PPL

Editor support for the PCBoard Programming Language: syntax highlighting,
completion, signature help, hover, document symbols and formatting for `.pps`,
`.ppd` and `.ppx` files. Errors and warnings come from the same compiler that
`pplc` uses.

## Requirements

None, if you took the package for your platform: the `icyboard-ppl`
binary comes with it. The package without a platform in its name does not carry
a server, so it needs one on your `PATH`. Either way, `icyboardPpl.serverPath` points
the extension at a server you built yourself.

## Running a PPE

**IcyBoard PPL: Run PPE** builds the open `.pps` with `pplc` and hands the
executable beside it to `icboard --ppe`, in a terminal of its own - a PPE asks
its caller questions, so it needs one.

That takes two programs the extension does not ship. Both are looked for on the
`PATH` unless a setting says otherwise:

| Setting | What it points at |
| --- | --- |
| `icyboardPpl.compilerPath` | the `pplc` that builds the source |
| `icyboardPpl.boardPath` | the `icboard` that runs the executable |
| `icyboardPpl.boardConfig` | the `icyboard.toml` to run against, or its directory |

A PPE runs on a board, so `icboard` needs a configuration. Left empty, it looks
in the workspace folder and then in `ICB_PATH`.

## Building this extension

```
pnpm i
npm run compile
pnpm run package
```


