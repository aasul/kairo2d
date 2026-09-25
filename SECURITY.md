# Security policy

Kairo2D 3.4.0 is an experimental, uncompiled source snapshot. No audited or supported
production release is represented by this archive.

## Reporting

Do not post exploitable details, tokens or private projects in a public issue.
When this repository is published, use the host's private vulnerability-reporting
channel if enabled. No maintained security email/address is asserted here. If no
private channel exists, open a minimal request for private contact without exploit
material. Include affected version, platform, reproducer and impact through that
private channel.

## Trust model

Game source, native extensions and project assets must come from trusted authors.
Project-relative reads, mutation guards, resource caps and a Lua watchdog are useful
guardrails, not an OS sandbox. Native decoders can allocate before all size checks;
Lua callbacks can call native work the instruction watchdog cannot preempt. Local
filesystem races and hostile project authors are outside the sandbox guarantees.
Never run untrusted games with elevated privileges or valuable ambient access.

Kairo Link listeners/connections are inactive by default. Hosting/connecting is explicit; it uses a strong
OS-random shared key and authenticated encryption. A valid key grants participation,
not a distinct human identity. The host has code-update authority for the tester's
game APIs and configured save identity. Authentication cannot make a hostile host safe.
Keep credentials out of the project and source control. Restart the host to rotate
keys; simply disconnecting a peer does not revoke its key. Windows token files inherit
parent ACLs. Protect the chosen directory and avoid public listener exposure.

Protocol/path/size/time checks are implemented, but the network code has not been
compiled, executed, fuzzed or audited during authoring. Treat it as experimental.
There is no remote shell, arbitrary native binary transfer, automatic service startup
or external command API. The threat model and limits are in docs/kairo-link.md.

Replay does not rewind files or other external effects. Export can accidentally ship
sensitive data if you store it under ordinary permitted names; exclusions are not a
comprehensive secret scanner. Review all packages and third-party licenses before
sharing. Signed binaries, dependency audits and release acceptance remain future gates.

## 3.4 inspection and profiles

Protocol 2 rejects the old 3.3 wire protocol. The same bounded typed DebugCommand
model serves local editor stdin and authenticated Link transport. Inspector reads
require explicit exposure; writes additionally require writable metadata, a fresh
VM session ID, an expected-current-value match and type/range/shape validation.
Only supported primitive/table values are admitted. There is no eval expression,
shell command, arbitrary global walk or arbitrary file-request command.

Scene/mixer/debug/bookmark/localization commands are development authority granted
to a trusted session, not a multi-tenant permission model. Rate/queue/size limits
reduce accidental or malicious load but do not make shared-key hosts untrusted-safe.
The Release profile disables development tools and Link joining by default; an
author can explicitly change those defaults. Project profiles are not privilege
isolation or a replacement for running untrusted code in an OS sandbox.

Captured bookmark bytes can include registered private game data. Exports are
metadata-only, but labels/notes may still be sensitive. Ordinary project exports
can contain game data under names not recognized by exclusions; review every ZIP.
