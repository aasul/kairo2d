# Security

Kairo2D is experimental. The 3.4.0 source has not been audited or validated as a
production release. The notes here describe the current design and its limits, not a
security certification.

## Reporting a vulnerability

Please use GitHub's private vulnerability reporting for this repository. Don't post
exploit details, tokens, or private project files in a public issue. Include the
affected version, platform, steps to reproduce, and likely impact. If private
reporting is unavailable, open an issue asking for a private contact and leave out
the sensitive details.

## Running projects

Treat game code, native extensions, and project assets as trusted input. Path checks,
resource limits, and the Lua instruction watchdog reduce some risks, but they do not
isolate a game from the operating system. Native decoders may allocate memory before
all size checks run, and Lua code can call native work that the watchdog cannot stop.
Do not run an untrusted project with elevated privileges or access to data you need
to protect.

## Kairo Link

Link is off until someone chooses to host or connect. It uses an OS-generated shared
key and authenticated encryption, but the key identifies a session, not an individual
person. A trusted host can update the tester's game APIs and use the configured save
identity. Encryption does not make a hostile host safe.

Keep the token outside the project and source control. Restart the host to rotate it;
disconnecting a tester does not revoke their key. On Windows, token-file access is
inherited from its parent folder, so protect that folder. Avoid exposing a listener
to the public internet. Link has not been compiled, run through fuzz testing, or
independently audited for this source snapshot. See [the Link guide](docs/kairo-link.md)
for the protocol and its limits.

There is no remote shell, native binary transfer, automatic service startup, or
external command API. A Link session is intended for trusted development, not as a
multi-user permission system. Rate and size limits help contain mistakes but do not
turn an untrusted host into a safe one.

## Inspector, Replay, and exports

Inspector only exposes values that game code registers. Values are read-only unless
the author marks them writable; edits are checked against the current session, value,
type, and range. These controls are development tools, not user or tenant permissions.

Replay restores registered game data and supported physics state. It does not undo
file writes or other outside effects. Bookmark exports contain metadata, but labels
and notes can still include private information. Game exports may include data stored
under ordinary filenames, so review each package before sharing it. Export exclusions
are not a complete secret scanner.

Release profiles turn off development tools and Link joining by default. Projects can
change those defaults. A profile is not an operating-system sandbox. Signed binaries,
dependency review, and full release acceptance remain future work.
