## Change

Describe the problem and the smallest useful solution.

## Validation

Record commands actually run and their results. Explicitly list checks you could
not run. For rendering/audio/input changes, include device and platform details.

## Compatibility

Describe Lua API, configuration or resource-lifetime changes, and link to the
updated documentation/example.

- [ ] Tests cover the changed behavior.
- [ ] Formatting, Clippy and workspace tests pass locally, or limitations are stated.
- [ ] No backend objects or raw pointers are exposed to Lua.
- [ ] No secrets, machine-specific paths or build artifacts are included.
