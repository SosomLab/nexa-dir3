# nexa-dir.spec — RPM 명세(rpmbuild · 이미 빌드된 바이너리를 스테이징에서 담는다 · docs/33 §2 FHS 레이아웃).
#   build-rpm.sh 가 넘기는 매크로: _version · _stagedir(build-deb.sh와 같은 FHS 스테이징 = target/packaging/linux/deb-root/usr)
#   소스 tarball 없이 %install 에서 스테이징을 복사한다 — 빌드는 cargo가 이미 했고(deb와 같은 바이너리), spec은 포장만.
#   debuginfo 추출은 끄고(strip=symbols 이미 적용) 바이너리 재strip도 막는다(Rust 실행 파일을 rpm이 다시 만지지 않게).
%global debug_package %{nil}
%global __strip /bin/true
%global __os_install_post %{nil}

Name:           nexa-dir
Version:        %{_version}
Release:        1%{?dist}
Summary:        Cross-platform lightweight dual-panel file explorer
License:        PolyForm-Noncommercial-1.0.0
URL:            https://github.com/SosomLab/nexa-dir3
# 아키텍처는 build-rpm.sh의 `rpmbuild --target`(x86_64 · aarch64)이 정한다.
# 런타임 dlopen(Wayland/X11) — 빌드 시 링크 없음. 자동 의존성 탐지는 glibc만 잡는다.
Recommends:     libxkbcommon
Recommends:     google-noto-sans-cjk-fonts

%description
Nexa Dir is a lightweight dual-panel file explorer that looks and behaves the
same on Windows, macOS and Linux. It is a single executable written in Rust
that draws its own UI (no Qt, WebView or Electron).
Features: tabs, inline tree, preview pane (WASM plugins), built-in terminal,
batch rename, checksums, duplicate finder, folder compare and sync, favorites,
trash with restore. User data lives in ~/.config/nexa-dir.
License: PolyForm Noncommercial 1.0.0 (noncommercial use only).

%prep
# 소스 없음 — 스테이징 복사만.

%build
# cargo가 이미 빌드했다(build-rpm.sh).

%install
rm -rf %{buildroot}
mkdir -p %{buildroot}%{_prefix}
cp -a %{_stagedir}/. %{buildroot}%{_prefix}/

%post
command -v gtk-update-icon-cache >/dev/null 2>&1 && gtk-update-icon-cache -q -t -f %{_datadir}/icons/hicolor 2>/dev/null || :
command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database -q %{_datadir}/applications 2>/dev/null || :

%postun
command -v gtk-update-icon-cache >/dev/null 2>&1 && gtk-update-icon-cache -q -t -f %{_datadir}/icons/hicolor 2>/dev/null || :
command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database -q %{_datadir}/applications 2>/dev/null || :

%files
%{_bindir}/nexa-dir
%{_bindir}/nexa-copy
# C ABI transfer engine (T-202 1, docs/26 7): private dir under /usr/lib (not %%{_libdir} = lib64 on Fedora - deb staging puts it in /usr/lib).
%dir %{_prefix}/lib/nexa-dir
%{_prefix}/lib/nexa-dir/libndir_xfer.so
%{_datadir}/nexa-dir/include
%{_datadir}/applications/nexa-dir.desktop
%{_datadir}/icons/hicolor/*/apps/nexa-dir.png
%dir %{_datadir}/nexa-dir
%{_datadir}/nexa-dir/plugins
%license %{_docdir}/nexa-dir/LICENSE.md
%doc %{_docdir}/nexa-dir/LICENSE.ko.md
%doc %{_docdir}/nexa-dir/README.md
%doc %{_docdir}/nexa-dir/README.ko.md
%doc %{_docdir}/nexa-dir/THIRD-PARTY-NOTICES.txt
%doc %{_docdir}/nexa-dir/copyright

%changelog
* Sat Oct 31 2026 Sangyong Bae <kiros33@gmail.com> - 0.28.0-1
- macOS: IOSurface presentation by default and partial redraws (idle frames ~15x cheaper),
  status-bar marker blink threshold, Finder-rule drag modifiers, drag-out to Finder and
  other apps, text drop (paths navigate, text goes to the terminal), .app bundles behave
  like files with "Show Package Contents", drop onto an app opens with it, OS row icons and
  kind names, Finder-style context menu, Terminal.app font profile, Disk status cell,
  in-process Quick Look (Cmd+Y, follows the selection), SSD-aware copy threads.
- Decomposed (NFD) file names: the search box and type-ahead match composed input; the
  status column no longer shows "NFD"; Info panel "Normalization" line; "NFD n" counter
  in the panel status strip that filters to those rows.
- Search box toggle tooltips show on hover without flicker; shorter tooltip texts.
- Self-check runs real probes for pty, fonts, preview and archive.
- nexa-copy: --job <file.json> (serializable job spec), --progress JSON lines plus a final
  report, --report <file>; new ndir-xfer engine facade crate.
- New settings: "Query OS icons", macOS presentation path, marker blink threshold.
- Fixes: memory window font bars show true resident bytes, launcher is scale-independent,
  drop highlight no longer sticks after a Finder drop, test harness condition waits.

* Mon Oct 26 2026 Sangyong Bae <kiros33@gmail.com> - 0.27.0-1
- Quick filter is now a search box (Ctrl+F): match-case, whole-word and regex toggles,
  all-words matching with Hangul jamo sequences, a progress ring on the frame, and a new
  "Search subfolders" toggle that finds names inside collapsed folders (folders with a
  match stay listed, nothing is expanded for you; folders you expand stay expanded).
- Built-in zip now compresses with DEFLATE (LZ77 + dynamic Huffman, no external crate);
  already-compressed formats are stored; 4 GiB guard; "Compress" is in the context menu on
  every OS; several archives extract in sequence; CRC-32 is verified on extract.
- Checksums: save GNU-style sums files (.sha256 and others) and verify them from the
  context menu; duplicate finder options (include subfolders; match by content, name+size
  or size).
- Folder compare: compare-by-content default and one-way sync "mirror" (remove items that
  exist only on the target, to the Trash; off by default); recompare after trash.
- Performance mode third pass (animations off, longer polling, smaller scrollback and
  caches, transparency 0) with locked settings; window transparency has separate focused
  and unfocused values (unfocused default 20 %).
- Terminal: cursor shape and padding follow the Windows Terminal profile; pixel-precise
  and fast wheel scrolling of the scrollback.
- Bulk rename: changed-span highlight, conflict rows, Tab cycling, resizable window, more
  card kinds, per-folder numbering, OS name validation.
- nexa-copy CLI: --verify, --mirror, --include/--exclude, --ask, dry-run, Ctrl+C cancel;
  bundled on every channel.
- Settings: "Changed" view, export/import, per-area fonts, type-ahead scope, text
  rasterization keys, favorites export/import, column-width sync per folder type, toolbar
  floating groups and reset.
- Lists: triangle sort marks, sort-click setting, select by pattern, copy-path variants,
  recent folders, folder size column, progress "current file", status bar transfer cell,
  undo from the completion toast.
- Linux: optional direct execution of ELF executables, xdg-open failure toast, zombie
  process reaping.
- Fixes: double/triple click inside the rename field selects text instead of opening the
  file; search box clipboard shortcuts and edit menu; git worktree watching; NFC
  composition order; right-button drop menu runs shell handlers asynchronously.

* Tue Oct 20 2026 Sangyong Bae <kiros33@gmail.com> - 0.26.0-1
- Responsiveness: folder listing, back/forward, refresh and change-watch reloads run on a worker
  (the old list stays until the new one is ready) · partial redraw (only changed regions are
  repainted) · self-change echo suppression for folder watching · trash deletion, drive capacity
  queries, dock preview, empty-folder probes and icon caching moved off the UI thread · idle
  memory trim every 300 s with a light/full scope · each change has a setting to switch it off.
- Menus: the menu bar pull-down, context menus and status bar cell popups are now mutually
  exclusive; a click outside an open menu closes it first and, by default, also acts right away
  (new settings "Close menus when another area is used" and "React immediately to the click
  that closes a menu"). Key presses still go to the open menu.
- Fixes: result collection race in background workers · empty-folder probe ordering.

* Sat Oct 18 2026 Sangyong Bae <kiros33@gmail.com> - 0.25.0-1
- Right-button drag and drop like File Explorer: a drop menu with copy, move, create shortcut,
  extract and the shell's own handlers (7-Zip and others; the intersection for multi-selection);
  copy/move go through the app's transfer engine with the same progress and conflict dialogs ·
  Linux XDND ask and macOS equivalents · macOS: NFC display, NFD marker and "Convert to NFC",
  Quick Look (Space) and the preview panel linked to Preview.app (Linux: Sushi) · batch rename:
  preset menu rebuilt (header, reset, save/edit/remove-all with in-place timed confirmation),
  sequence editor with drag ordering and per-item delete, card drag reordering with an insertion
  indicator, card header band, overlay scroll bar with fast scroll, input fields with shortcuts
  and a right-click edit menu, regex replacement "$2_$1" fixed, presets keep every card, several
  rename windows at once · flash messages (settings-driven) · toasts scoped to the panel that did
  the work · folder compare window fixes · rename retried on Windows sharing violations ·
  breadcrumb Space key · empty-folder chevron hidden · memory window grouped with a detail switch.
* Fri Oct 10 2026 Sangyong Bae <kiros33@gmail.com> - 0.24.0-1
- Linux drag and drop completed: drag files out to other apps (Nautilus, terminals) over X11/XWayland
  with a drag label showing copy/move, Ctrl/Shift honoured, drops land on the hovered folder row ·
  settings dependencies stage 2 (forced values, bound ranges) · sort state inherited by new tabs and
  kept in the session · Linux context menu: item icons, order editor, Nautilus scripts, copy/move to
  folder · trash across filesystems fixed · self-check menu hidden in release builds · bundled docs in
  English (Korean README added).
* Thu Oct 08 2026 Sangyong Bae <kiros33@gmail.com> - 0.23.3-1
- Tab status bar git summary: load control as advanced folder-watch settings (settle delay after
  changes, failure backoff, query time limit, no periodic polling while the window is inactive) and
  branch name re-read after an external checkout · natural sort is now a per-tab view option that
  follows the view-scope setting (setting value = default for new tabs).
* Thu Oct 08 2026 Sangyong Bae <kiros33@gmail.com> - 0.23.2-1
- Console helper ndir.exe (PowerShell CLI output/capture) · memory window 700x800 fit-to-content and
  redraw skip · system theme follows OS while running (Windows) · parallel multi-algorithm checksum ·
  streaming ZIP inflate · git status options · folder-size redraw loop fix · performance baseline tooling.
* Tue Oct 07 2026 Sangyong Bae <kiros33@gmail.com> - 0.23.1-1
- Memory window: private working set row (Task Manager axis) · flash message settings (ui.flash_*) ·
  system theme follows OS after dark start (Windows) · launcher folder items navigate the active panel.
* Sat Oct 03 2026 Sangyong Bae <kiros33@gmail.com> - 0.23.0-1
- 첫 RPM 명세(nexa-dir3 T-82 · nexa-sql T-72 차용) — deb와 같은 FHS 스테이징을 포장.
