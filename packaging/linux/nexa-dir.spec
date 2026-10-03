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
Nexa Dir는 Windows · macOS · Linux에서 같은 화면으로 동작하는 경량 듀얼 패널 파일 탐색기입니다.
전부 Rust로 만든 정적 링크 실행 파일이며 자체 래스터라이저로 그려 Qt·WebView·Electron을 쓰지 않습니다.
탭 · 인라인 트리 · 미리보기(WASM 플러그인) · 내장 터미널 · 일괄 이름 변경 · 휴지통 복원.
사용자 데이터는 ~/.config/nexa-dir 에 둡니다.

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
* Sat Oct 03 2026 Sangyong Bae <kiros33@gmail.com> - 0.23.0-1
- 첫 RPM 명세(nexa-dir3 T-82 · nexa-sql T-72 차용) — deb와 같은 FHS 스테이징을 포장.
