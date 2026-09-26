# Contributing

Thanks for taking an interest in Kairo2D. The project is experimental, and there is still plenty to improve. Before picking up a large change, open an issue or discussion so we can agree on the problem and its scope.

## Before you start

This 3.4.0 source snapshot has not passed a native build and release check. Please start by following [VALIDATION.md](VALIDATION.md) and note which checks you can run. Don't assume a check passed because a test exists. If the build fails, include the command and relevant output in your report.

For a change, use a focused branch and keep the pull request about one problem. Add or update tests when behavior changes. Run formatting, workspace checks, tests, and Clippy if your setup supports them. For editor, device, audio, graphics, or network changes, describe which parts you tried on a real system and which you could not verify. The exact native release checklist is in [docs/release-acceptance.md](docs/release-acceptance.md).

If a change is entirely AI-generated and has not been checked by a person, keep it on an AI/... branch until it has been reviewed. After reviewing and verifying the change, move it to a normally named feature branch before opening a pull request. Disclose AI assistance in the pull request template, including whether a person reviewed the result.

## Project conventions

- Keep Lua APIs small and consistent. Document their arguments, defaults, lifetime, errors, and reload behavior.
- When adding a public Lua function, update docs/lua-api.json and regenerate the reference with python scripts/generate_api_docs.py.
- Keep engine APIs independent of a particular graphics or physics backend. Use opaque handles rather than passing native pointers into Lua.
- Keep file access within the project and bound work that can grow with user input.
- Don't silence a warning just to make CI green. Explain why a lint exception is needed if one is unavoidable.
- If a control or feature is not implemented, document that clearly rather than leaving a control that appears to work.

## Submitting a pull request

Describe the problem, the change, and how you checked it. Include known limits and compatibility changes. Be clear when a check could not be run. Don't include local build output, tokens, private project data, or generated dependency notices without reviewing them first. Release builds should use a real resolved Cargo.lock; don't invent one or copy one from an unrelated project.

The main branch ruleset requires a pull request and an approving review of the latest push before changes can be merged.

Code and original assets are covered by the [MIT license](LICENSE). Please follow the [Code of Conduct](CODE_OF_CONDUCT.md). For security issues, use [SECURITY.md](SECURITY.md) instead of opening a public issue with exploit details.
