about = PCBoard Programming Language Compiler
disassemble = output the disassembly instead of compiling
nowarnings = don't report any warnings
version = print the version and exit
mono = write plain text, without the ansi escapes that colour the output
runtime = version number for the compiled PPE, valid: 100, 200, 300, 310, 320, 330, 340, 400 (default)
lang-version = language version (defaults to the manifest, PPL_LANG_VERSION, then runtime capped at 400)
compression = PPE 400 section compression: none (default) or zstd
debug = include optional source symbol names in PPE 400 debug data
cp437 = specify the encoding of the file (cp437 = true, utf8 = false), defaults to autodetection
init = create & init new ppl package in target directory
defines = semicolon separated list of pre processor variables
format = formats source file instead of compile
stdout = with --format, write the result to stdout and leave the file alone
check = checks source/package for errors without compiling
print-config = prints the effective compiler configuration without compiling
print-config-json = prints the effective compiler configuration as json without compiling
file = file[.pps] to compile (extension defaults to .pps if not specified)