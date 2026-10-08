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
%{_datadir}/applications/nexa-dir.desktop
%{_datadir}/icons/hicolor/*/apps/nexa-dir.png
%dir %{_datadir}/nexa-dir
%{_datadir}/nexa-dir/plugins
%license %{_docdir}/nexa-dir/LICENSE.md
%doc %{_docdir}/nexa-dir/LICENSE.ko.md
%doc %{_docdir}/nexa-dir/README.md
%doc %{_docdir}/nexa-dir/THIRD-PARTY-NOTICES.txt
%doc %{_docdir}/nexa-dir/copyright

%changelog
* Thu Oct 08 2026 Sangyong Bae <kiros33@gmail.com> - 0.23.2-1
- Console helper ndir.exe (PowerShell CLI output/capture) · memory window 700x800 fit-to-content and
  redraw skip · system theme follows OS while running (Windows) · parallel multi-algorithm checksum ·
  streaming ZIP inflate · git status options · folder-size redraw loop fix · performance baseline tooling.
* Tue Oct 07 2026 Sangyong Bae <kiros33@gmail.com> - 0.23.1-1
- Memory window: private working set row (Task Manager axis) · flash message settings (ui.flash_*) ·
  system theme follows OS after dark start (Windows) · launcher folder items navigate the active panel.
* Sat Oct 03 2026 Sangyong Bae <kiros33@gmail.com> - 0.23.0-1
- 첫 RPM 명세(nexa-dir3 T-82 · nexa-sql T-72 차용) — deb와 같은 FHS 스테이징을 포장.
