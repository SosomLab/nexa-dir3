# STATUS — 현황 한 장

> 최신 위. 상세는 [journal](journal/), 요약은 [DEVLOG](DEVLOG.md), 목표 대비는 [MILESTONES](MILESTONES.md) · [TODO](TODO.md).

## 10-03 9차 — M2 T-26·27 `nexa-explorer`(nexa-ui 106차)

- **한 일**: dir2 PathBar·InfoDock·OverlayBars를 nexa-ui `nexa-explorer` 크레이트로 이식(dir2 시험 25) · nexa-grid DrawCtx 어휘 보강. push 완료 · nexa-sql 빌드 유지.
- **지금 상태**: M2 🚧 — 남은 것 = T-28 Tooltip/Overlay → T-29 nexa-dlg 대화상자 → T-30 소형 컨트롤 → T-31 DrawCtx 보강 → T-32 FolderTree. 핵심 창 골격 컨트롤은 전부 준비됨 → M3 착수 가능.
- **걸린 것**: 없음.

→ [journal/2026-10-03 §11](journal/2026-10-03.md)

## 10-03 8차 — M2 T-25 `nexa-grid`(nexa-ui 105차)

- **한 일**: dir2 가상 행 그리드 엔진 전체를 nexa-ui `nexa-grid` 크레이트로 이식(dir2 테스트 52 green) + `draw::Adapt` + nexa-ctl `Invalidations` 틱 요청. push 완료.
- **지금 상태**: M2 🚧 — 남은 것 = T-26 PathBar → T-27 InfoDock → T-28 Tooltip/Overlay → T-29 nexa-dlg 대화상자 → T-30~32. 그 뒤 M3 앱 골격.
- **걸린 것**: 없음.

→ [journal/2026-10-03 §10](journal/2026-10-03.md)

## 10-03 7차 — M2 착수: nexa-ui 103·104차(T-20~T-24)

- **한 일**(형제 저장소 nexa-ui · push 완료): `InputEvent` 3변형 + 휠 줄 수 · `RecordCtx` · MenuBar 체크/라디오/단축키 열/활성/프로그램 열기 · Toolbar 토글 · TabBar 아이콘/툴팁/가운데 클릭 · **StatusBar 신규**. nexa-ctl 시험 391 · nexa-sql 빌드 유지.
- **지금 상태**: M2 🚧 — 다음 = T-25 `nexa-grid`(dir2 rows/columns/typeahead/fastscroll 이식 · 가장 큰 덩어리) → T-26 PathBar → T-27 InfoDock → T-28·29.
- **걸린 것**: 없음.

→ [journal/2026-10-03 §9](journal/2026-10-03.md) · nexa-ui [journal](../../nexa-ui/docs/journal/2026-10-03.md)

## 10-03 6차 — M1 T-17 명령 표·키맵 → **M1 완료**

- **한 일**: `ndir-settings::commands`(48 명령 · dir2 단축키 + macOS 대응안) · `keymap`(nexa-sql 엔진) · 레지스트리 `key.*` 49 · 시험 7(명령↔키 1:1 · 충돌 0 · dir2 31건 · macOS 13건). 전체 테스트 196.
- **지금 상태**: M1 ✅ → **M2 nexa-ui 보강 착수**(형제 저장소 · T-20 InputEvent → T-21 RecordCtx → T-22 MenuBar → T-24 StatusBar → T-25 nexa-grid …). 각 컨트롤은 nexa-ui 커밋/push 뒤 dir3에서 소비.
- **걸린 것**: 의도된 차이 3(단축키 페이지 신설 · `tab.prev` · F6 단일 표기) — 매트릭스 ⚠ 등재.

→ [journal/2026-10-03 §8](journal/2026-10-03.md)

## 10-03 5차 — M1 T-16 `ndir-license`

- **한 일**: nsql-license 1:1 복제(제품 `nexa-dir` · `NDIR_BUILD_DATE` · Feature 변형 0 = dir2 게이트 0 정책) · 시험 8(타 제품 거부 포함) · 자가 점검 license 5항목 PASS.
- **지금 상태**: M1 남은 것 = T-17 명령 표(`commands.rs`). 그 뒤 M2(nexa-ui 보강 — 형제 저장소 작업).
- **걸린 것**: Q-1(Feature/Tier/Kind 구성)은 기본값(게이트 0)으로 진행.

→ [journal/2026-10-03 §7](journal/2026-10-03.md)

## 10-03 4차 — M1 T-13~15 `ndir-settings`

- **한 일**: nexa-sql 설정 엔진 복사 + dir2 86키 레지스트리(dir2 페이지 순·기본값) + 곁 표(OS_DEFAULTS·DEPENDS·HIDDEN·ADVANCED) + JSON + dir2 가져오기(`import_dir2`) + 자가 점검 `config` 3항목. 시험 +17.
- **지금 상태**: 다음 = T-16 `ndir-license` → T-17 명령 표 → M2(nexa-ui 보강).
- **걸린 것**: 열린 결정 Q-7(성능 거버너 `perf.*` 도입 여부 — dir2에 없음 · 보류).

→ [journal/2026-10-03 §6](journal/2026-10-03.md)

## 10-03 3차 — M1 T-12 `ndir-i18n`

- **한 일**: dir2 언어 자원 3종 임베드 + 빌드 시 파리티 검사(결함 6건 실제 적발 → 수정) + 3-OS OS 언어 감지 + 전역 표(워커 스레드 OK) · 자가 점검 `resources` 실제 항목. 테스트 **166 green** · 3-OS ✓.
- **지금 상태**: 다음 = T-13/14 `ndir-settings`(nexa-sql 엔진 복사 + dir2 키 표) → T-16 `ndir-license` → T-17 명령 표.
- **걸린 것**: 없음(OS 종속 문구 15건은 기능 이식 때).

→ [journal/2026-10-03 §5](journal/2026-10-03.md)

## 10-03 2차 — M1 착수: 코어 5크레이트 이식

- **한 일**: `ndir-core/vfs/tree/ops/term` = dir2 복사 + 크레이트 이름 치환 + lint 적응(Debug 16 · unwrap 6). 테스트 **156 green** · `check-3os.sh` ✓ · CI 1차 커밋 3-OS success.
- **지금 상태**: M1 🚧 — 다음 = T-12(`ndir-i18n`: dir2 `.lang` 3종 + 키 검사) → T-13/14(`ndir-settings`) → T-16(`ndir-license`) → T-17(명령 표).
- **걸린 것**: 없음.

→ [journal/2026-10-03 §4](journal/2026-10-03.md)

## 10-03 1차 — M0 골격 착수

- **한 일**: 형제 저장소 최신화(nexa-ui `df75f5a` ff · nexa-license `4f02524`) + 복원 태그 `baseline/pre-nexa-dir3-2026-10-03` push · nexa-ui 3-OS 검사 통과 · **이식 원장 23문서**(`docs/port/10~51` · ultracode 조사 19건 — 7건은 세션 한도로 중단 뒤 재개 중 사용자 지시로 중지, 산출물은 존재) · 규칙 문서(CLAUDE · 01 · 10 · 15 · 16 · 18) · 현황 4층 · 워크스페이스 + `nexa-dir` bin 뼈대(`--version`·`--smoke`·`--selfcheck`) · CI 3-OS.
- **지금 상태**: M0 🚧 → 다음 = T-05(check-all) → M1 T-10(코어 이식).
- **걸린 것**: 열린 결정 Q-1~Q-6([10 §4](10-decision-record.md)) — 기본값으로 진행 중. 누락 조사 = port/52(nexa-dlg·fs 카탈로그) · 98·99(대조) → T-91.
- **게이트**: 로컬 fmt/clippy/test/smoke/selfcheck → CI 결과는 journal에.

→ [journal/2026-10-03](journal/2026-10-03.md)
