# STATUS — 현황 한 장

> 최신 위. 상세는 [journal](journal/), 요약은 [DEVLOG](DEVLOG.md), 목표 대비는 [MILESTONES](MILESTONES.md) · [TODO](TODO.md).

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
