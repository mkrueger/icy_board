# Working with Icy Board

For a new board, follow the [agent setup guide](docs/agent-setup.md). It covers
installation, board creation, local-only testing, validation and handoff.
For an existing PCBoard installation, use the [migration guide](docs/migration.md)
instead; do not run `icbsetup create` over an existing board.

Use the [documentation index](docs/README.md) to find task-specific guides and
the [TOML configuration reference](docs/configuration/README.md) for exact on-disk
fields. The full handbook also contains reStructuredText under
[`docs/source/`](docs/source/); searching only Markdown misses some material.

Do not commit a generated board, credentials or logs. Do not expose a listener
outside the local machine without the operator's explicit approval. Validate
changes with `icbsetup check`, then test the actual local call before reporting
that a board is ready.
