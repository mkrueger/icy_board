# clap-i18n-richformatter

i18n support for `clap::error::RichFormatter`!

## Usage

Add `clap-i18n-richformatter` to `Cargo.toml`:

```toml
clap-i18n-richformatter = { version = "0.3.0", features = ["derive"] }
```

Initialise clap-i18n-richformatter as such:

```rust
use clap::Parser;
use clap_i18n_richformatter::clap_i18n;

#[derive(Debug, Parser)]
#[clap_i18n]
#[command(version, about = "An example program with i18n support")]
struct Args {
    #[arg(long, short)]
    a: bool,
    #[arg(long, short, requires = "a")]
    b: bool,
}

fn main() {
    let args = Args::parse_i18n_or_exit();
    println!("{args:?}");
}
```

Running `LANG=zh_CN.UTF-8 cargo run -- -b` and `LANG=C cargo run -- -b` will now show localised (and non-localised below) error output for Clap:

```
错误: 未提供下列必要参数:
  --a

用法: arg_requires --a --b

如需了解更多信息，请输入 --help。
```

```
error: the following required arguments were not provided:
  --a

Usage: arg_requires --a --b

For more information, try '--help'.
```

## Contribution

Your contribution are welcome via Pull Requests:

- Code improvement (of course).
- Localisation to other languages, see [i18n/](/i18n) for l10n metadata (in [Fluent](https://github.com/projectfluent/fluent-rs)).

## Inspiration

From https://github.com/clap-rs/clap/issues/380#issuecomment-1254059266:

> In discussing this with someone on reddit, I think users can get something working, even if it isn't streamlined yet
>
>   - Replace Commands: header with a localized value via `Command::subcommand_help_heading`
>   - Replace <COMMAND> value name with a localized value via Command::subcommand_value_name
>   - Replace Arguments and Options: headers with localized values via Command::next_help_heading and/or Arg::help_heading
>   - Disable the built-in flags (`Command::disable_help_flag(false)` and `Command::disable_version_flag(false)`) and provide your own versions with localized Arg::help
>   - If the help template has any hard coded strings, replace them with a localized version via `Command::help_template`
>   - Fork clap::error::RichFormatter, replacing any hard coded strings with localized values. Use Error::apply to swap the formatter (e.g. use `Parser::try_parse` to get the error, call apply, and then `err.exit()`)

`clap-i18n-richformatter` follows the last suggesion from [@epage](https://github.com/epage) to implement i18n support for Clap errors, since the error messages are quite finite and predictable.

> Fork clap::error::RichFormatter, replacing any hard coded strings with localized values. Use Error::apply to swap the formatter (e.g. use `Parser::try_parse` to get the error, call apply, and then `err.exit()`)
