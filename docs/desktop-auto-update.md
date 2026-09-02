# Zhimind desktop auto-update

Zhimind uses the **Tauri 2 updater** shape: signed desktop release artifacts, a
rolling `latest.json` endpoint on GitHub Releases, in-app
check/download/install, and a hard stop of managed agent / mirror / voice / IM
processes before binary swap. The desktop brand is Zhimind; the managed runtime
behind it remains the official `grok` CLI / Grok Build.

CLI updates are a separate channel and continue to use the official `grok
update` flow and its configured x.ai / GCS mirrors. They are not served by the
Zhimind desktop updater.

## Architecture

```
CI release
  ├── vX.Y.Z                 user-facing installers (DMG / AppImage / NSIS …)
  └── grok-desktop-latest    rolling updater release
        └── latest.json  + per-platform archive + .sig
                 ▲
                 │ check()
        GitHub  grok-desktop-latest/latest.json
                  ▲
                  │
        Desktop  tauri-plugin-updater  (release builds only)
                 │ prepare_for_app_update → stop agents / mirror / voice / IM
                 │ install + relaunch
        UI: Settings → About
```

GitHub Releases is both the signed updater store and the manual-download
fallback. Unsigned / local builds open the current repository's latest Release.

## App pieces

| Piece | Location |
|-------|----------|
| Build-time gate | `build.rs` → `cfg(grok_updater_enabled)` when both `GROK_UPDATER_*` env vars are set (crate always linked for ACL) |
| Release conf delta | `scripts/build-release-config.mjs` → `src-tauri/tauri.release.conf.json` (gitignored — always regenerate) |
| Plugin register | `src-tauri/src/lib.rs` (cfg + non-debug only) |
| Platform support | `is_auto_update_supported` — Linux requires AppImage (`APPIMAGE` env) |
| Pre-relaunch teardown | `prepare_for_app_update` — **only after** successful `install()`, never before |
| Frontend state machine | `src/hooks/useUpdater.ts` + `UpdaterProvider` (single path: plugin or GitHub) |
| Path honesty (copy / channel) | `src/lib/appUpdateHonesty.ts` — signed auto vs GitHub manual vs unsupported vs host-only; soft-fail error classes; agents stop only after install prepare |
| UI | Settings → About (`AboutUpdateRow`) |
| Capabilities | `updater:allow-*`, `process:allow-restart` |

Local `pnpm dev` / debug builds **never** enable the updater plugin (no
feature, no env), so dev binaries never hit a production endpoint.

### Install / teardown order (P0)

```
download → install() → prepare_for_app_update() → relaunch()
```

If `install()` fails, agents / voice / IM / mirror stay running.

## Secrets (GitHub Actions)

| Secret / variable | Purpose |
|-------------------|---------|
| `GROK_UPDATER_PUBLIC_KEY` | minisign public key embedded in the app |
| `TAURI_SIGNING_PRIVATE_KEY` | minisign private key for signing updater archives |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | password for the private key (empty string OK) |
| Apple signing / notarize secrets | codesign + notarize DMG / .app (recommended for macOS Gatekeeper) |

Generate a keypair once (see [Tauri updater](https://v2.tauri.app/plugin/updater/)):

```sh
pnpm tauri signer generate -w ~/.tauri/grok-app.key
# public key → GROK_UPDATER_PUBLIC_KEY
# private key file contents → TAURI_SIGNING_PRIVATE_KEY
```

### Maintainer production checklist

Before treating silent update as “on” for users:

1. **Secrets in the shipping repo** (Settings → Secrets and variables → Actions): all four rows above that you use. Empty `TAURI_SIGNING_PRIVATE_KEY` must not be set — omit or set a real key.
2. **Local dry-run (no secret values printed):**
   ```sh
   ./scripts/verify-updater-setup.sh
   ./scripts/verify-updater-setup.sh --fetch-latest
   ```
3. **Release cut:** tag `vX.Y.Z` so CI builds installers **and** refreshes `grok-desktop-latest` + `latest.json` + `.sig`.
4. **Smoke on a prior signed build:** Settings → About shows **Update channel: in-app (signed release)** → Check → Download → Install and restart → version matches tag.
5. **Failure path:** if install fails, agents / Remote IM / mirror must keep running (`prepare_for_app_update` only after successful `install()`).
6. **Unsigned / local builds:** About must show the **GitHub manual download** channel and open the latest Release / installer (no crash; never claims silent update).
7. **Linux non-AppImage:** About shows **unsupported** package-type channel + AppImage-only note when the plugin is compiled in.

In-app host command `updater_status` reports `{ channel, pluginEnabled, platformSupported, endpoint }` for Doctor / About (`channel` is `silent` | `github_manual` | `unsupported`).

## Rolling endpoint

```text
https://github.com/usertianziyang/Zhimind/releases/download/grok-desktop-latest/latest.json
```

Publish two GitHub Releases per formal version:

1. **`vX.Y.Z`** — human installers + notes + stable installer aliases + `downloads.json` for the Zhimind download page
2. **`grok-desktop-latest`** — updater archives + `latest.json` (clobber each release)

Do **not** point website download buttons at `grok-desktop-latest`; it is only
the signed in-app updater channel. First-time installs use the normal latest
Release and its stable installer aliases.
The historical `Grok_*` asset aliases may remain in CI for compatibility; they
are not desktop UI copy.

## Build steps (outline)

```sh
export GROK_UPDATER_PUBLIC_KEY=...
export GROK_UPDATER_ENDPOINT=https://github.com/usertianziyang/Zhimind/releases/download/grok-desktop-latest/latest.json
export TAURI_SIGNING_PRIVATE_KEY=...
export TAURI_SIGNING_PRIVATE_KEY_PASSWORD=...

# 1) Write tauri.release.conf.json (gitignored — required before --config)
node scripts/build-release-config.mjs

# 2) Same GROK_UPDATER_* env must still be set so build.rs enables registration
pnpm tauri build --config src-tauri/tauri.release.conf.json
```

Without step 1, `tauri build --config src-tauri/tauri.release.conf.json` fails with
file not found. The crate is always a hard dependency (Tauri ACL); only
**registration** is gated by the env cfg.

After all platforms upload assets to `vX.Y.Z`:

```sh
TAG=v0.1.9 REPO=<owner>/Zhimind bash scripts/assemble-updater-manifest.sh
```

Platform keys: `darwin-aarch64`, `darwin-x86_64`, `linux-x86_64`, `windows-x86_64`.

## Linux note

Only **AppImage** supports in-app update. `.deb` / `.rpm` installs report
channel **`unsupported`** (About: package-type cannot auto-update + AppImage
note) and surface `manual-required` when a newer build is known so the user can
open GitHub Releases.

## macOS note

Codesign + notarize the `.app` / DMG in CI when Apple secrets are present.
After notarization, rebuild the updater `.tar.gz` from the signed app and
re-sign with the Tauri updater key (same pattern as Buzz) if you notarize
post-build.

## Manual verification

1. `pnpm typecheck` / `pnpm test` — UI unit tests
2. `cargo test --manifest-path src-tauri/Cargo.toml updater::` — Rust helpers
3. Settings → About shows the **manual GitHub update path** on local builds (expected)
4. Release smoke: build with both env vars, confirm `is_updater_plugin_enabled`
   is true in a release binary, and that check hits GitHub `latest.json`
