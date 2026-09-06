about = PCBoard Programming Language Decompiler
raw = raw ppe without reconstruction control structures
disassemble = output the disassembly instead of ppl
output = write source to stdout instead of a file; banner, progress and warnings go to stderr
check = check runtime compatibility; findings exit 0 unless --strict is used; read, analysis or report errors exit 1
strict = requires --check; exit 1 for any unsupported, unimplemented or partially implemented reference, otherwise 0; errors still exit 1
cp437 = write the source as cp437 instead of utf8, for use with the original tooling
style = keyword casing style, valid values are u=upper (default), l=lower, c=camel
lang-version = language version the source is written for, defaults to PPL_LANG_VERSION then the newest one
version = print the version and exit
file = file[.ppe] to decompile