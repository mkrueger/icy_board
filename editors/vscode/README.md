# IcyBoard PPL

Editor support for the PCBoard Programming Language: syntax highlighting,
completion, signature help, hover, document symbols and formatting for `.pps`,
`.ppd` and `.ppx` files. Errors and warnings come from the same compiler that
`pplc` uses.

## Requirements

None, if you took the package for your platform: the `icyboard-ppl`
binary comes with it. The package without a platform in its name does not carry
a server, so it needs one on your `PATH`.

`icyboardPpl.binPath` points at the directory holding the IcyBoard programs -
the server, `pplc` and `icboard` are built together, so one path finds all three.

## Running a PPE

**IcyBoard PPL: Run PPE** builds the open `.pps` with `pplc` and hands the
executable beside it to `icboard --ppe`, in a terminal of its own - a PPE asks
its caller questions, so it needs one. It sits on `Ctrl+F5` and behind the play
button above a `.pps`. If either program is nowhere to be found, it says so and
offers the setting rather than leaving an error in a log.

A PPE runs on a board, so `icboard` needs a configuration.
`icyboardPpl.boardConfig` names the `icboard.toml` or the directory holding it;
left empty, the workspace folder and `ICB_PATH` are searched. Without one, Run
PPE says so instead of starting a board that has nothing to run on.

`icyboardPpl.runArguments` decides how the board is called. It defaults to
`["--ppe", "${ppe}"]`, where `${ppe}` is the executable that was just built;
`${source}` and `${workspaceFolder}` are there too. A PPE that wants a caller
and its own parameters takes the other door:

```json
"icyboardPpl.runArguments": ["--runppe", "Sysop;;PWRD:secret;PPE:${ppe};first;second"]
```

Being a workspace setting, it can differ per project.

A workspace that has not been trusted keeps its own answers to all of these: the
extension starts programs, and which ones is not a question a folder you just
opened gets to answer.

## Building this extension

```
pnpm i
npm run compile
pnpm run package
```


