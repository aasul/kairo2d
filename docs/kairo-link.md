# Kairo Link 3.4 (experimental)

Kairo Link is opt-in remote development, not game multiplayer or simultaneous text editing. It uses the existing encrypted/authenticated Noise-PSK TCP transport with a **protocol 2** preface/prologue. Both ends must use 3.4. Protocol 1 / 3.3 clients are rejected; there is no silent compatibility negotiation.

## Start from the visible editor controls

Open a trusted project, save its files, choose **Link > Host a session**, and choose a bind address. Loopback is appropriate for one-machine testing; bind a LAN address only deliberately. Export the private 32-byte token to a file **outside the project** and share it securely with trusted testers. Do not commit it or paste it into public logs.

The tester opens Kairo, chooses **Link > Join as tester**, enters the host address, selects the token file, explicitly accepts trust, and launches. The runtime stages host revisions in a temporary project; it does not overwrite the tester's source checkout. Direct IP/hostname remains supported. There is no rendezvous server, NAT traversal service, QR join, LAN discovery, Steam or Discord dependency. A VPN can supply reachability but is not required by the protocol.

## Per-tester controls

The host accepts at most four connected testers. This connection cap existed before 3.4; 3.4 adds per-peer controls, telemetry and inspection. Label a tester, toggle automatic reload, push the latest validated revision to one tester, or disconnect that connection. Each peer has independent logs, last revision/reload result, command response, last-seen time and measured exchange round trip. A reconnect creates a new peer identity; labels/settings are not a durable device registry. A holder of the shared session token can reconnect after being disconnected; rotate/stop the session to revoke trust.

Choose **Inspect** next to a tester or select the peer in **Live Inspector / Audio Mixer**. Only registered inspector fields may be read/edited. Scene commands, supported debug toggles, pause/step/replay/bookmarks, language and mixer operations use the same typed protocol as the local runtime. There is no `eval` command. The tester can disable tools or Link via its selected profile.

## Reload and delivery semantics

Saved Lua/assets are fingerprinted and staged as a bounded project revision. Syntax/asset validation precedes publication; the receiver builds a replacement VM, then swaps only on success. Explicit game.saveState/restoreState hooks can retain supported plain data; arbitrary Lua object identity is not preserved. Texture handling and asset restrictions remain documented in [Hot reload](hot-reload.md).

A peer's automatic reload toggle controls whether new revisions are sent; **Push latest** explicitly requests one. Controls are bounded, best-effort, ordered per connection, not durable transactions. Queued commands may be lost on disconnect/restart and are not blindly replayed. An acknowledgement means accepted by the runtime; deferred replay operations complete at the next frame boundary and may report a later error. Inspector compare-and-set prevents stale edits from overwriting changes.

Exchanges poll roughly every 500 ms. Unchanged inspector payloads are omitted. Runtime telemetry includes current scene/replay state and CPU/render samples. Round-trip measurements include protocol/transfer work and are not pure network ping. Best-effort logs may be dropped/truncated under load rather than blocking simulation.

## Security boundary and limits

Session tokens use OS randomness. Every connection authenticates before project/control traffic. The clear version preface identifies the protocol only; it is not authentication. Project paths are validated and transfers reject escapes, tokens, symlinks and unsupported/native files. Frames are at most 65535 bytes including encryption overhead, individual typed controls 4 KiB, telemetry 40 KiB, inspector values 24 KiB. Command queues are capped at 32 per peer and exchanged eight at a time. Socket deadlines and bundle/time/file limits remain enforced. Unknown versions, malformed fields, unsafe paths and invalid values fail.

Authenticated peers are trusted to send game code. Validation is not a proof that a project is benign; do not join strangers' sessions. No third-party security audit, public-service hardening, portable bug-state transfer, remote screenshot capture or filesystem race-proof OS sandbox is claimed. Keep Link disabled for player releases. See [Live Inspector](live-inspector.md) and [Security](../SECURITY.md).
