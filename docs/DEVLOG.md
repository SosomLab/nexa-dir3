# DEVLOG — 날짜별 요약

> 시간 역순. 항목당 1~2줄. 상세는 journal.

## 2026-10-03

- **M4 T-50 플랫폼 포트**: `platform/` 포트 9종 + `Platform::native/fake` + ADR-0001 · 폴링 감시 자동 재열람 · 외부 열기(Opener) · Windows 드라이브 용량 · selfcheck shell/open/fs 실제 항목 · 시험 +5 → [journal §20](journal/2026-10-03.md)
- **M3 T-43 2차 탭 메뉴·열 폭 동기**: 탭 잠금/고정/복제/분리·부착 + 우클릭 컨텍스트 메뉴(nexa-ctl) + 다른 패널로 이동 + 열 폭 동기 + 세션 잠금/고정 · 시험 +2 · 내 PC 용량 열은 M4 ⚠ → [journal §19](journal/2026-10-03.md)
- **M3 T-44 설정 창·단축키 창**: nexa-sql `prefs_win`/`keys_win` 복사(SQL 미리보기·확장·`Msg` 제거 · i18n 키) · 보조 창 호스트 배선(`app/windows.rs`) · `apply_setting` 한 길 + **적용 누락 감시 시험** · i18n +19×3 · 시나리오 `prefs-open` → [journal §18](journal/2026-10-03.md)
- **T-06 `ndir-check` 러너**: `.scn` 시나리오(격리 홈·샘플 트리·`@ready…assert…quit`·검사식) 5개 PASS · 의존 0 · CI Windows 단계 · 상대 경로 함정 적발 → [journal §17](journal/2026-10-03.md)
- **M3 T-46 기동 명령·패닉 훅**: `@ready/@idle/@after` · `quit:<코드>` · `assert.<대상>:<식>`(실패 = exit 3) · `ui.click:@영역` · 덤프 6(panel/list/tabs/status/menu/layout) · `crash.rs`(crash-<unix>.txt + 다음 기동 안내) · 창 실증 exit 5/3 → [journal §16](journal/2026-10-03.md)
- **M3 T-45 세션**: dir2 `session.cfg` 형식 그대로(`session.rs` · 미사용 키 보존) · 창 생성 전 복원(실패 탭 건너뜀 · 실행 인자 우선) · 디바운스 저장(nexa-conf SaveScheduler 1 s/5 s) · 종료 저장 · 창 2회 실행 실증 → [journal §15](journal/2026-10-03.md)
- **M3 T-43 1차 dir2 패널 구조**: `panel.rs`(패널별 탭 바·네비 4버튼·경로 바·목록 · 탭별 히스토리 · 위로 = 떠난 폴더 선택 · 홈 = 내 PC) + `nav.rs`(dir2 그대로) + 스플리터 드래그/50 % 스냅 + dir2 배치 수치(툴바 28 · 상태 22 · 열 5) · 시험 52 → [journal §14](journal/2026-10-03.md)
- **M3 T-41 창 없는 AppCore**: `layout_for`/`paint_into` 최소 분리 + 배치 골든 1장 + `RecordCtx`/입력/기동 명령 시험 5(창 0 · 0.1 s) · 적발 3(열 넘침·창 없는 layout·플랫폼 단언) → [journal §13](journal/2026-10-03.md)
- **M3 T-40 앱 골격**: nexa-sql 호스트 껍질 9파일 복사 + dir2 아이콘 + `App`/`layout`/이벤트 루프 + `ndir-tree` 파일 목록 + 메뉴 5·툴바 13·명령 한 길 + `NDIR_STARTUP_CMD`/`layout.dump` — **창이 뜨고 폴더를 다닌다**(Windows 실증 · 시험 +38) → [journal §12](journal/2026-10-03.md)
- **M2 T-26·27(nexa-ui 106차)**: `nexa-explorer` — dir2 PathBar·InfoDock·OverlayBars 이식(시험 25) → [journal §11](journal/2026-10-03.md)
- **M2 T-25(nexa-ui 105차)**: `nexa-grid` 크레이트 — dir2 VirtualRows 엔진 이식(테스트 52) + DrawCtx 어댑터 + Invalidations 틱 → [journal §10](journal/2026-10-03.md)
- **M2 T-20~24(nexa-ui 103·104차)**: InputEvent 확장 · RecordCtx · MenuBar/Toolbar/TabBar 보강 · StatusBar 신규 — nexa-ctl 391 · nexa-sql check ✓ → [journal §9](journal/2026-10-03.md)
- **M1 T-17 · M1 완료**: 명령 표 48 + 키맵 엔진 + `key.<id>` 전수 등재(설정 창 "단축키" 페이지) · 충돌 시험이 macOS ⌘Y 충돌 적발 → ⇧⌘Y → [journal §8](journal/2026-10-03.md)
- **M1 T-16**: `ndir-license` — nsql-license 복제(제품 `nexa-dir` · Feature 0 · 시험 8) + 자가 점검 license 5항목 → [journal §7](journal/2026-10-03.md)
- **M1 T-13~15**: `ndir-settings` — nexa-sql 엔진 복사 + dir2 86키 레지스트리(dir2 기본값) + 곁 표 + JSON + dir2 `settings.cfg` 가져오기 · 곁 표 무결성 시험 신설 · i18n 79키 추가 → [journal §6](journal/2026-10-03.md)
- **M1 T-12**: `ndir-i18n` — dir2 `.lang` 3종 + 빌드 시 키/자리표 검사 + 3-OS OS 언어 + 전역 표 · `{1}` 자원 결함 수정 · 테스트 166 → [journal §5](journal/2026-10-03.md)
- **M1 T-10·11**: dir2 코어 5크레이트(`ndir-core/vfs/tree/ops/term`) 전수 이식 — 테스트 156 green · 3-OS clippy ✓ · lint 적응만(Debug·expect) → [journal §4](journal/2026-10-03.md)
- **M0 착수**: 형제 저장소 최신화 + 복원 태그 · 이식 원장 23문서(docs/port) · 규칙 문서(CLAUDE·01·10·15·16·18) · 현황 4층 · 워크스페이스·bin 뼈대·CI 3-OS → [journal](journal/2026-10-03.md)
