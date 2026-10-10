# Nexa Dir 3 — cross-platform native file explorer (all Rust)

> Cross-platform (Windows · macOS · Linux), single-binary, ultra-lightweight native file explorer — the cross-platform successor of [nexa-dir2](https://github.com/SosomLab/nexa-dir2). Korean guide: [README.ko.md](README.ko.md).

**Nexa Dir 3** reproduces the features, screen layout and resources of the Windows-only [nexa-dir2](https://github.com/SosomLab/nexa-dir2) (`0.22.0`) on all three operating systems. Windowing and input use `winit + softbuffer`, the screen is drawn by our own CPU rasterizer ([nexa-ui](https://github.com/SosomLab/nexa-ui)), settings follow the [nexa-sql](https://github.com/SosomLab/nexa-sql) model, and licensing uses [nexa-license](https://github.com/SosomLab/nexa-license). No OS-native controls are used — the same screen on every OS.

## Highlights

- Dual-panel explorer with tabs, tree/flat/tile views, multi-column sort, type-ahead and fast scrolling.
- File operations with progress, conflict handling, undo/redo, bulk rename, trash, checksums, duplicate finder, folder compare/sync, archive extract.
- Built-in terminal, info/preview dock (text, images, Markdown and archives via WASM plugins), favorites, launcher bar.
- OS integration through one `platform/` layer: shell context menus, trash, file clipboard, drag and drop, folder watching, default apps.
- Portable first: settings live next to the executable (`data/`) when present, otherwise in the user config directory. One-time import of nexa-dir2 settings.
- Free for personal and noncommercial use; commercial use requires a license (see below).

## Install

Installers and the portable zip are on [GitHub Releases](https://github.com/SosomLab/nexa-dir3/releases) (verify with `sha256sums.txt`).

```powershell
winget install SosomLab.NexaDir            # Windows MSI
winget install SosomLab.NexaDir.Portable   # Windows portable
choco  install nexa-dir                    # Windows MSI
```
```bash
brew install --cask kiros33/tap/nexa-dir   # macOS (Universal pkg)
sudo apt install ./nexa-dir_<version>_amd64.deb      # Linux (deb)
sudo dnf install ./nexa-dir-<version>-1.x86_64.rpm   # Linux (rpm)
```

Uninstall: Windows Apps & features · macOS `sudo "/Applications/Nexa Dir.app/Contents/Resources/uninstall.sh"` · Linux `dpkg -r nexa-dir` / `dnf remove nexa-dir`. User data (settings, sessions, favorites, rename presets) is kept on uninstall.

## Build and check

Keep the sibling repositories next to this one: `../nexa-ui` and `../nexa-license` (path dependencies).

```bash
cargo build --workspace
cargo run -p nexa-dir -- --smoke        # start without a window (startup check)
cargo run -p nexa-dir -- --selfcheck    # self-check (doctor); in a Windows console use `ndir --selfcheck`
scripts/check-3os.sh                    # 3-OS cross check before push
```

Procedures: [docs/18](docs/18-build-and-test.md) (Korean).

## Documentation — [docs home](docs/README.md) (Korean)

Shortcuts: [project memory CLAUDE.md](CLAUDE.md) · [decision record](docs/10-decision-record.md) · [development rules](docs/15-dev-methodology.md) · [doc and git conventions](docs/16-doc-git-conventions.md) · [port ledger](docs/port/00-index.md)

## Project / License

| Item | Value |
| --- | --- |
| Organization | **SosomLab** — <https://sosomlab.com> |
| Origin | <https://github.com/SosomLab/nexa-dir2> (feature source) · <https://github.com/SosomLab/nexa-dir> (original) |
| Developer | Sangyong Bae — kiros33@gmail.com |

**PolyForm Noncommercial 1.0.0** ([LICENSE.md](LICENSE.md) · Korean [LICENSE.ko.md](LICENSE.ko.md)) — free for personal and noncommercial use; commercial use requires a paid license (contact kiros33@sosomlab.com).
