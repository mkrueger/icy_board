# Project instructions

## Scope and working tree

- Keep changes focused on the requested task. Preserve unrelated and pre-existing changes, including work from other agents.
- Commit or push only when explicitly requested. Before committing, review the staged diff and include only task-related changes.
- Do not create documentation files or perform unrelated cleanup unless requested or required by the task.

## Pull requests and commit messages

- Follow the language and style of the project's existing contributions unless the user specifies otherwise.
- Use short, concrete titles describing the change. Avoid promotional language and generic claims about improvements.
- Explain the problem, why the change is needed, and any important limitations. Do not narrate every implementation step or repeat the diff.
- Report only tests and checks actually run, with their results. Distinguish verified behavior from assumptions, untested behavior, and unrelated failures.
- Never claim that a bug is fixed merely because the source looks correct. Reproduce the reported behavior and validate the relevant execution or rendering path where possible.

## Code and review comments

- Add code comments only when they explain intent, constraints, or non-obvious behavior that the code cannot express clearly.
- Prefer one short line. Do not restate the next line of code or write change summaries addressed to reviewers inside the source.
- Keep review comments factual and actionable: identify the location, the problem, and its concrete impact. Label uncertainty rather than presenting speculation as fact.
- Keep user-facing summaries concise. Do not include tool-by-tool narration.

## Validation

- Prefer `cargo test-low` for local Rust tests; it limits build parallelism. Run checks relevant to the changed crates and behavior.
- For localized UI changes, validate both English and German. Use explicit locale loaders or separate test processes rather than changing a shared global locale during parallel tests.
- For rendering bugs, test the rendered terminal output, including the reported geometry; parser output alone is not sufficient.
- Add regression coverage when practical. Do not weaken assertions just to make tests pass.
- Preserve test command exit statuses; when piping output, use `pipefail` so a failing test is not hidden by a successful filter.