# ADR-0001 — 플랫폼 포트(OS 분기는 `platform/` 한 층 · 포트 9종 · 운영/가짜 두 겹)

- 상태: 채택(2026-10-03 · M4 T-50) · 근거 원장: [DR-5](../10-decision-record.md) · [port/40 SKEL-404·425~432](../port/40-sql-app-skeleton.md) · [port/44 CI-103·110](../port/44-sql-ci-test-docs.md)

## 결정

1. OS 분기 코드는 `crates/nexa-dir/src/platform/{mod.rs, windows.rs, macos.rs, linux.rs, fake.rs}` **안에만** 둔다. 호출부(AppCore · 패널 · 명령)는 OS를 모르고 `Platform`의 trait 객체만 쓴다.
2. 포트(trait) = `Shell`(기본 셸 탐지) · `Pty` · `ContextMenuProvider` · `Trash` · `FileClipboard` · `DragSource` · `Watcher` · `Opener` · **`Disk`**(드라이브 용량 — dir2 "내 PC" 열 PANEL-044 때문에 DR-5의 8종에 더함). 기기 ID는 nexa-license 내장(별도 포트 없음).
3. 미지원·다른 OS = `Err(PlatformError::Unsupported("<무엇>"))` 또는 `None`. 호출부는 **사유 문구를 한 번만** 안내한다(docs/18 §7). 패닉·조용한 무시 금지.
4. OS 분기 **판정**은 순수 함수로 두고 주입해 시험한다(`pick_shell(candidates, exists)` · CI-038).
5. 시험은 `Platform::fake()`(`fake.rs` · 호출 기록 + 값 주입 · OS/프로세스/클립보드 0) · 실제 구현은 `--selfcheck`(T-54)가 사용자 기기에서 검증한다 — 두 겹.
6. 공용 폴백은 `mod.rs`에: `PollWatcher`(폴더 mtime+항목 수 스냅숏 비교 · 3-OS 동일) · `CommandOpener`(각 OS가 명령 이름만 준다) · `find_in_path`.

## 지금 상태(T-50)

| 포트 | Windows | macOS | Linux | 가짜 |
| --- | --- | --- | --- | --- |
| Shell | pwsh → powershell → cmd | $SHELL → zsh → bash → sh | $SHELL → bash → sh | 주입 |
| Opener | `cmd /C start` · `explorer /select,` | `open` · `open -R` | `xdg-open` | 기록 |
| Disk | `GetDiskFreeSpaceExW`(kernel32 수동 extern) | `statvfs`(Darwin 레이아웃) | `statvfs`(glibc 64비트 레이아웃) | 주입 |
| Watcher | PollWatcher | PollWatcher | PollWatcher | 주입 큐 |
| Trash | `SHFileOperationW`(ALLOWUNDO) | `~/.Trash` 이동(되돌리기 T-52) | freedesktop Trash 규격 | 기록 |
| FileClipboard | CF_HDROP + Preferred DropEffect | Unsupported(T-52) | Unsupported(T-53) | 메모리 |
| Pty · ContextMenuProvider · DragSource | Unsupported(T-51 B · M5) | Unsupported(T-52) | Unsupported(T-53) | 기록/echo |

## 대안과 기각

- 파일마다 `#[cfg] mod imp`(nexa-sql 방식): dir3는 분기가 넓어(터미널·셸 메뉴·휴지통·DnD·파일 클립보드) 흩어지면 OS별 누락을 못 센다 → 한 층.
- 외부 crate(trash · notify · open · portable-pty): DR-3 외부 crate 0 지향 · 단일 바이너리 용량 예산 → 수동 extern + 프로세스 실행(열기만).
