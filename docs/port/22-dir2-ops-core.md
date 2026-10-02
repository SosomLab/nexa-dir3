# 22 · nexa-dir2 인벤토리 — 코어 · 작업 엔진 (OPS)

> 단계: 이해(인벤토리) · 읽기 전용 조사 결과. 이 문서의 `OPS-NNN` ID가 이후 구현 · 교차 검증의 체크리스트다.
> 원본: nexa-dir2 0.22.0(`89c635c`) · 대상: nexa-dir3(winit + softbuffer + nexa-ui `df75f5a`).
> 확인하지 못한 것은 **추정**이라고 적었다. "코드 추적"은 소스를 따라가 확인했지만 실행해 보지는 않았다는 뜻이다(이 단계는 빌드 · 실행 금지).

ID 대역: `OPS-001~153` 기능(§1) · `OPS-201~` nexa-ui 매핑(§3) · `OPS-301~` OS 분기점(§4) · `OPS-401~` 이식 주의(§6) · `OPS-501~` 회귀 테스트 후보(§7).

---

## 0. 범위 — 읽은 파일과 줄 수

| 구분 | 파일 | 줄 수 | 읽은 범위 | 이 문서의 줄 표기 |
|---|---|---|---|---|
| 담당 | `nexa-dir2/crates/nexa-ops/src/lib.rs` | 1134 | 전부(본문 1~590 · 테스트 592~1134) | `ops:N` |
| 담당 | `nexa-dir2/crates/nexa-ops/src/history.rs` | 535 | 전부(본문 1~312 · 테스트 314~535) | `hist:N` |
| 담당 | `nexa-dir2/crates/nexa-ops/src/batch_rename.rs` | 1438 | 전부(본문 1~959 · 테스트 961~1438) | `br:N` |
| 담당 | `nexa-dir2/crates/nexa-vfs/src/lib.rs` | 593 | 전부(본문 1~295 · 테스트 297~593) | `vfs:N` |
| 담당 | `nexa-dir2/crates/nexa-tree/src/lib.rs` | 1488 | 전부(본문 1~805 · 테스트 807~1488) | `tree:N` |
| 담당 | `nexa-dir2/crates/nexa-core/src/lib.rs` · `secret.rs` | 35 · 164 | 전부 | `core:N` · `secret:N` |
| 담당 | `nexa-dir2/crates/nexa-app/src/bulkrename.rs` | 2252 | 전부(로직 + 배치) | `bulk:N` |
| 담당 | `nexa-dir2/Cargo.toml` + 각 crate `Cargo.toml` 7개 | 46 + 118 | 전부 | 전체 경로 |
| 담당 | `nexa-dir2/docs/01-architecture.md` · `20-session-coalescing.md` · `22-batch-rename-v2.md` | 69 · 121 · 115 | 전부 | 전체 경로 |
| 접점 | `nexa-dir2/crates/nexa-app/src/win.rs` | 9951 | 엔진 호출부만: 1190~1235 · 1369~1379 · 2739~2755 · 2868~2912 · 3176~3200 · 3395~3524 · 3700~3800 · 4040~4066 · 4364~4380 · 4480~4779 · 6395~6476 · 7019~7056 | `win:N` |
| 접점 | `nexa-dir2/crates/nexa-app/src/config.rs` | — | 362~384 · 986~1011(`data_dir` · `load` · `save`) | 전체 경로 |
| 접점 | `nexa-dir2/crates/nexa-app/src/ctl/{spin,segmented,groupcard,menubutton,label,grid,button,iconbutton,style}.rs` | — | 머리말 계약 주석 + 공개 항목 Grep | 전체 경로 |
| 접점 | `nexa-dir2/crates/nexa-app/src/recycle.rs` | — | 1~32(머리말 · 공개 시그니처) | 전체 경로 |
| 접점 | `nexa-dir2/docs/23-cross-platform-feasibility.md` | 249 | 36~150 | 전체 경로 |
| 접점 | `nexa-dir2/.github/workflows/ci.yml` | — | 전부 | 전체 경로 |
| 접점 | `nexa-dir2/crates/nexa-app/lang/ko.lang` | — | `bulk.*` · `ops.*` · `op.*` · `history.*` 키 Grep | 전체 경로 |
| nexa-ui 확인 | `nexa-ui/crates/nexa-ctl/src/**` | — | `pub struct/enum/trait` 전수 Grep · `lib.rs` 1~80 · `controls/button.rs` 40~100 · `controls/mod.rs` 225~300 · `controls/radio.rs` 1~60 · `controls/tree.rs` 636~700 · `controls/listedit.rs` 1~60 · `controls/ctxmenu.rs` 99~136 · `controls/glyphs.rs` 10~40 | 전체 경로 |
| nexa-ui 확인 | `nexa-ui/crates/nexa-fs/src/lib.rs` | 957 | 1~250 · 336~565 + 공개 API Grep | 전체 경로 |
| nexa-ui 확인 | `nexa-ui/crates/nexa-conf/src/lib.rs` | — | 1~360 | 전체 경로 |
| nexa-ui 확인 | `nexa-ui/docs/21-grid-family.md` | 129 | 키워드 Grep(체크 · 정렬 · nexa-grid) | 전체 경로 |
| nexa-sql 확인 | `nexa-sql/crates/nexa-sql/src/{input_win.rs 1~30, grid.rs 1~25}` · `nsql-settings/src/lib.rs`(경로 Grep) | — | 부분 | 전체 경로 |

범위 밖(존재만 기록): `nexa-dir2/crates/nexa-vfs/src/archive/*`(7파일 2789줄 · 테스트 45개 · `cfg(windows)` 0건) — 압축 미리보기 인벤토리 소관. `nexa-tree`는 `nexa-dir3/docs/port/13-dir2-panel-filelist.md`(PANEL)가 화면 관점으로 전부 다뤘다 — 이 문서는 **이식성 · 엔진 접점** 관점만 적는다.

구조 파악 방법: 범위 크레이트 전체에 `cfg(` Grep(§1-10) → 각 파일을 구간별로 끝까지 읽음 → `nexa_ops::` · `nexa_vfs::` 호출부를 `nexa-app` 전체에서 Grep해 접점 확인.

---

## 1. 기능 목록

이식 분류: **N** = 플랫폼 중립(거의 그대로 이식) / **A** = nexa-ui 컨트롤 · 그리기로 교체 / **P** = OS별 구현 분기 필요 / **W** = Windows 전용 유지. `N*` = 컴파일 · 기존 테스트는 그대로 통과하지만 **비Windows에서 의미가 틀어지는 지점이 있어 패치가 필요**(§4의 해당 ID 참조).

### 1-1. 전송 엔진 — `nexa-ops/src/lib.rs`

| ID | 기능(사용자 관점) | 동작 상세(조건 · 예외 · 기본값) | 진입점 | Win32/OS 의존 | 분류 | 기존 테스트 |
|---|---|---|---|---|---|---|
| OPS-001 | 복사/이동 단일 경로 | `transfer(sources, dest_dir, op, resolve, on_event, cancel) -> Outcome`. 모든 진입점(붙여넣기 · DnD · 클립보드)이 이 함수로 수렴. 항목을 **입력 순서대로 순차** 처리. 시작 전에 전 항목 `size_of` 합산(동기) | `ops:484-590` | `std::fs`만 | N* | `cross_folder_copy_move_and_dir_recursive`(ops:663) |
| OPS-002 | 같은 폴더 규칙 | 원본 부모 == 대상 폴더(`path_equals` — 대소문자 무시)면: **이동 = 무동작**(`Ok(None)` → `skipped`에 기록 · `ItemEnd=Skipped`), **복사 = 순번 복제**(`unique_dest` · overwrite=false · 충돌 질문 없음) | `ops:507-540` | 경로 비교가 Windows 관례 | N*(OPS-303) | `same_folder_rules_move_noop_copy_duplicates`(ops:640) |
| OPS-003 | 이름 충돌 처리 | 대상에 같은 이름이 **있을 때만** `resolve(&natural)` 호출 → `Overwrite`(덮어씀) / `Skip`(건너뜀 → `skipped`). 충돌 없는 항목은 질문 없음. 순서는 항목 순서 | `ops:544-552` | — | N | `conflict_skip_and_overwrite_sequential`(ops:684) |
| OPS-004 | 충돌 대화상자(앱) | 4버튼 **덮어쓰기 / 모두 덮어쓰기 / 건너뛰기 / 취소**. "모두 덮어쓰기"만 이후 무확인(`decided`), 건너뛰기는 매번 다시 물음. 취소 · Esc = `cancel=true` + 그 항목 Skip. 문구는 UI 스레드에서 선확정(`ops.overwrite`의 `{0}` = 잎 이름). **워커 스레드에서 모달을 띄운다** | `win:4517-4577` | 워커 스레드 Win32 모달 | A + P(OPS-312) | 없음 |
| OPS-005 | 자기/하위 폴더 이동 금지 | 폴더 **이동**이고 `is_same_or_sub(src, dest_dir)`면 그 항목만 오류(`errors`에 기록) · 나머지 계속. `move_onto_with_progress`에도 같은 방어. **복사에는 이 방어가 없다** | `ops:541-543` · `ops:369-372` | 문자열 접두 비교(대소문자 무시) | N*(OPS-303 · OPS-404) | `cycle_move_is_isolated_error`(ops:725) |
| OPS-006 | 진행 이벤트 | `Event::Plan{sizes,total_bytes}` 정확히 1회 → 항목마다 `ItemStart{index,dest}`(실제 대상 경로 — 건너뜀/무동작은 미발생) → `Bytes(Progress{done_bytes,total_bytes,item_index,item_count})`(4MB 청크마다 · 전체 누적) → `ItemEnd{index,status}`(Done/Skipped/Failed · 취소도 Failed로 종결) | `ops:449-477` · `ops:495-578` | — | N | `progress_reports_bytes_up_to_total`(ops:744) · `item_events_report_skip_per_item`(ops:781) |
| OPS-007 | 취소 | `&AtomicBool`(Relaxed). 항목 시작 전 검사 + 청크마다 + 폴더 엔트리마다. 취소는 `io::ErrorKind::Interrupted`로 전파 → `Outcome.canceled=true` · 루프 중단. 이미 끝난 항목은 `transferred`에 남는다 | `ops:144-150` · `ops:502-505` · `ops:582-585` | — | N | `cancel_mid_copy_cleans_partial_and_reports`(ops:896) |
| OPS-008 | 실패 개별 격리 · 결과 | `Outcome{transferred:Vec<(원본,최종 대상)>, skipped, errors:Vec<(경로, 문자열)>, canceled}`. 한 항목 실패가 배치를 멈추지 않는다 | `ops:31-39` · `ops:573-588` | — | N | 위 테스트들 |
| OPS-009 | 파일 청크 복사 | 4MB 버퍼(`COPY_BUF`). overwrite=false면 `create_new`(존재 시 오류). 취소/실패 시 **부분 대상 파일 삭제**. 권한 · 시각 · 확장 속성은 **복사하지 않는다**(수동 read/write 루프) | `ops:41-42` · `ops:154-187` | — | N*(OPS-305) | `cancel_mid_copy_cleans_partial_and_reports` |
| OPS-010 | 폴더 재귀 복사 | `create_dir_all(dest)` → `read_dir` 엔트리별 재귀. **열거 `Err`는 오류로 전파**(종전 `flatten()`은 조용히 버림 → 이동 시 유실). 내부 파일은 overwrite=true. 판별은 `file_type().is_dir()`(링크 미추적) — 그 외는 전부 파일로 복사. `copied`가 있으면 복사 끝난 원본 경로를 **후위 순서**로 기록 | `ops:189-218` | 심링크 처리 OS 차이 | N*(OPS-306) | `move_by_copy_removes_only_copied_items_and_keeps_strays`(ops:982) |
| OPS-011 | 덮어쓰기 = 스테이징 후 교체 | 대상이 있고 overwrite면 같은 부모에 `.<이름>.nexa-tmp-<pid>-<seq>`로 **먼저 완성** → `commit_replace`. 파일: `rename`(원자 교체). 폴더: 옛 폴더를 `.<이름>.nexa-old-<pid>-<seq>`로 치우고 rename · 실패 시 원복 · 성공 후 옛 폴더 삭제. 파일 자리에 폴더 = 파일 삭제 후 rename. 취소/실패 시 **옛 대상 보존** + 스테이징 제거 | `ops:262-327` · `ops:331-339` | `fs::rename` 의미론(Windows = REPLACE_EXISTING) | N | `overwrite_keeps_old_dest_when_canceled_and_replaces_on_success`(ops:814) |
| OPS-012 | 이동 | 같은 볼륨 = `fs::rename`(끝난 뒤 `size_of(dest)` 1회 보고). overwrite + 대상 폴더 = 옛 폴더 옆으로 치운 뒤 rename · 실패 원복. 다른 볼륨 = 복사 후 **복사한 항목만** 원본에서 삭제(`remove_copied` — 남은 항목이 있으면 폴더를 지우지 않고 오류 문구 반환). 파일은 복사 후 `remove_file` | `ops:343-394` · `ops:223-259` | 볼륨 판정(OPS-013) | N*(OPS-301) | `move_by_copy_full_success_removes_source_tree`(ops:1022) · `cross_volume_move_never_loses_item_missed_by_enumeration`(ops:1086 — 둘째 볼륨 없으면 조기 반환) · `move_by_copy_locked_file_keeps_source_intact`(ops:1059 · `cfg(windows)`) |
| OPS-013 | 같은 볼륨 판정 | 첫 경로 구성요소 비교: `Prefix`(드라이브/UNC · 소문자화) 또는 `RootDir`("/"). 상대 경로 = 판단 불가 → `true`(보수). **비Windows는 절대 경로가 모두 "/" → 항상 true** | `ops:77-91` | Windows 드라이브 접두 전제 | **P**(OPS-301) | `size_of_recursive_and_same_volume`(ops:969) |
| OPS-014 | 충돌 없는 이름 만들기 | `unique_dest(dir, name, is_dir)`: 없으면 그대로. 있으면 `" (2)"`부터 증가. 파일 = 확장자 앞에 순번(`a (2).txt`), 폴더 = 확장자 분리 안 함(`v1.2 (2)`). 확장자 없는 파일 = 이름 끝 | `ops:93-119` | — | N | `unique_dest_numbering_file_and_dir`(ops:619) |
| OPS-015 | 크기 합산 | `size_of`: 폴더 재귀 합. 접근 실패 = 0(격리). 심링크 = 0(추적 안 함) | `ops:121-141` | — | N | `size_of_recursive_and_same_volume` |
| OPS-016 | 완전 삭제 | `delete_permanent`: 폴더 = `remove_dir_all` · 파일 = `remove_file` · 없으면 무동작. 휴지통 삭제는 엔진 밖(앱) | `ops:396-406` | — | N | `create_new_numbering_and_delete_permanent`(ops:945) |
| OPS-017 | 제자리 이름 바꾸기 | `rename(path, new_name) -> 새 경로`. 앞뒤 공백 trim · 빈 이름/`\`·`/` 포함 = `InvalidInput` · 루트 = 오류 · **대소문자만 다른 이름 포함 "동일" = 무동작(원래 경로 반환)** · 같은 이름 존재 = `AlreadyExists`. 금지 문자(`:*?"<>\|`) 검사는 없다(OS에 맡김) | `ops:408-430` | 경로 동등이 대소문자 무시 | N*(OPS-303 · OPS-401) | `rename_rules`(ops:924) |
| OPS-018 | 새 폴더 / 새 파일 | `create_new_dir(dir, base)` · `create_new_file(dir, "base.ext")` — `unique_dest`로 이름 확정 후 생성. 기본 이름은 앱 i18n(`new.folderBase` · `new.fileBase` + `.txt`) | `ops:432-447` · `win:4084-4086` | — | N | `create_new_numbering_and_delete_permanent` |
| OPS-019 | 경로 유틸 | `leaf_name`(잎 이름) · `exists`(`symlink_metadata` — 끊긴 링크도 존재) · `path_equals`(끝 `\`·`/` 제거 + ASCII 대소문자 무시 · 비공개) · `is_same_or_sub`(소문자화 + `\` 또는 `/` 접두) | `ops:44-75` | Windows 경로 관례 | N*(OPS-303) | 간접 |
| OPS-020 | 엔진 오류 문구 | 한국어 고정 문자열 5종: "자기 자신/하위 폴더로는 이동할 수 없음"(`ops:371` · `ops:542`) · "원본 폴더를 비울 수 없어 남겨 둠…"(`ops:254`) · "잘못된 이름"(`ops:413`) · "루트는 이름변경 불가"(`ops:417`) · "같은 이름이 이미 있음"(`ops:425`). 앱은 전송 오류는 **건수만**, 이름 바꾸기 오류는 `rename.fail`에 `e.to_string()`을 넣어 표시(`win:4062`) | 좌동 | — | N(i18n 누수 — §6 OPS-409) | 없음 |

### 1-2. 실행 취소 / 다시 실행 — `nexa-ops/src/history.rs` + 앱 계층

| ID | 기능(사용자 관점) | 동작 상세 | 진입점 | OS 의존 | 분류 | 기존 테스트 |
|---|---|---|---|---|---|---|
| OPS-030 | Undo/Redo 스택 | `OperationHistory`: 스택 2개 · 기본 상한 **100**(`with_capacity`는 최소 1) · `push` = redo 비움 + 상한 초과 시 가장 오래된 것 제거 · `undo()`/`redo()` = `Option<Result>`(빈 스택 = `None`) · **실패한 연산은 양쪽 스택에서 소실**(무결성 우선) · `clear`. **세션 한정(영속 없음)** | `hist:39-115` | — | N | `push_then_undo_then_redo_round_trip`(hist:368) · `push_clears_redo_stack`(385) · `undo_empty_returns_none`(395) · `capacity_drops_oldest`(402) · `failing_undo_drops_op_and_propagates`(415) |
| OPS-031 | 연산 계약 · 구조화 오류 | `trait ReversibleOp { description, undo, redo }`. `OpError::{Failed(n), MissingSource(잎), NameExists(잎)}` — 문구는 앱이 i18n 키로 변환 | `hist:18-37` | — | N | 간접 |
| OPS-032 | 이동 되돌리기 | `MoveBatchOp(pairs, description)`: undo = dest→src · redo = src→dest. `from` 없음 또는 `to` 존재 = 건너뛰고 실패 집계(**덮어쓰지 않음**). 이동은 `move_onto_with_progress(overwrite=false)`(교차 볼륨 = 복사 후 삭제) | `hist:117-171` | — | N | `move_batch_undo_moves_back_and_redo_moves_again`(438) · `move_batch_undo_skips_conflict_and_reports`(458) |
| OPS-033 | 복사 되돌리기 | `CopyBatchOp(pairs, description, delete_copy)`: undo = 사본을 **주입된 삭제 함수**로 제거(앱 = 휴지통 · 테스트 = 완전 삭제) · redo = 재복사(원본 소실/대상 존재 = 건너뜀) | `hist:173-229` | 삭제 수단 주입 | N(주입부 = P · OPS-307) | `copy_batch_undo_deletes_copy_and_redo_recopies`(476) |
| OPS-034 | 이름 변경 되돌리기 | `RenameOp(old, new, description)`: 소실 = `MissingSource` · 대상 존재 = `NameExists` · 그 외 I/O = `Failed(1)` | `hist:231-270` | — | N | `rename_op_round_trip`(501) |
| OPS-035 | 새로 만들기 되돌리기 | `CreateOp(path, description, delete, recreate)`: undo = 존재하면 주입 삭제 · redo = 없으면 주입 재생성 | `hist:272-312` | 삭제 수단 주입 | N | `create_op_undo_deletes_and_redo_recreates`(517) |
| OPS-036 | 휴지통 삭제 되돌리기(앱) | `DeleteBatchOp{paths, description}`: undo = `recycle::restore_by_original_paths`(휴지통 셸 폴더를 열거해 "원래 위치+이름" 일치 항목에 `undelete` 동사) · 복원 수 < 대상 수면 `Failed(차이)`. redo = 존재하는 것만 다시 휴지통 | `win:3412-3450` · `nexa-dir2/crates/nexa-app/src/recycle.rs:1-32` | `SHFileOperationW` · 휴지통 셸 폴더 COM | **P**(OPS-307) | 없음(실기) |
| OPS-037 | 가상 붙여넣기 되돌리기(앱) | `VPasteOp{paths, description}`: 원본 경로가 없어 `DeleteBatchOp`의 역방향 — undo = 생성물 휴지통 삭제 · redo = 휴지통 복원. DnD 스테이징 출신 전송도 이 연산으로 기록 | `win:3452-3491` · `win:4735-4744` | 좌동 | **P** | `split_staged_only_dnd_staging_sources`(win:9852) |
| OPS-038 | 오류 문구 변환(앱) | `Failed(n)` → `history.failedItems` · `MissingSource` → `history.missingSource` · `NameExists` → `history.nameExists` | `win:3502-3510` | — | N | 없음 |
| OPS-039 | 전송 완료 시 기록 규칙(앱) | `transferred`가 비지 않으면 기록(**취소돼도 수행분은 기록**). 이동 = `MoveBatchOp(op.moveCount)` · 복사 = `CopyBatchOp(op.copyCount, 휴지통 삭제 주입)`. 이름 바꾸기 = `RenameOp(rename.done)`(동일 이름 무동작은 제외) · 새로 만들기 = `CreateOp` · 일괄 이름 변경 = `MoveBatchOp` 1건(`bulk.done`) | `win:4729-4760` · `win:4047-4063` · `win:4099-4113` · `win:6456-6466` | — | N | 없음 |

### 1-3. 일괄 이름 변경 코어 — `nexa-ops/src/batch_rename.rs`

| ID | 기능(사용자 관점) | 동작 상세 | 진입점 | OS 의존 | 분류 | 기존 테스트 |
|---|---|---|---|---|---|---|
| OPS-050 | 파이프라인 미리보기 | `preview(items, ops, tz_min) -> Vec<String>`: 항목마다 (이름부, 확장자)로 나눠 블록을 **위→아래 순서로** 적용. 결과 = `이름부.trim() + 확장자`. 연번 순번 = 목록 인덱스(정렬은 호출자 책임) | `br:671-687` | — | N | `pipeline_applies_in_user_order`(br:974) · `op_order_matters`(1200) |
| OPS-051 | 적용 대상(스코프) | `Scope::{Name(기본), NameExt, Ext, ExtDot}`. NameExt = 합친 뒤 **마지막 `.` 기준 재분해**. Ext = 점 뺀 텍스트(결과가 비면 확장자 없음). ExtDot = 점 포함 텍스트 **결과를 그대로 채택** 후 전체 재분해(자동 점 복원 없음). **폴더**: Ext/ExtDot = 무변경 · NameExt = Name으로 수렴 | `br:19-50` · `br:498-548` | — | N | `scope_variants_select_working_text`(1003) · `extdot_insert_front_no_double_dot`(1074) |
| OPS-052 | 삽입 위치 | `InsertAt{offset, from_end}` — 선택한 끝에서 N **문자**(UTF-8 안전). 범위 초과 = 반대편으로 클램프(오류 아님). `InsertAt::edge(suffix)` = v1 호환 | `br:52-80` | — | N | `insert_at_arbitrary_position_with_clamp`(1111) |
| OPS-053 | 텍스트 치환 | `Replace{scope, find, with, match_case, regex:false, mode}`. 모드 All/First/Last/Entire(매치가 있으면 전체를 with로). 빈 find = 무변경, 단 **Entire + 빈 find = 무조건 교체**. 대소문자 무시는 문자 단위 `to_lowercase` 비교 · 비중첩 왼쪽부터 | `br:82-111` · `br:405-452` | — | N | `replace_modes_first_last_entire`(1038) |
| OPS-054 | 정규식 치환 | `regex:true` — 항상 전체 치환(`replace_all`) · `$1` 캡처 참조 · `match_case=false`면 패턴 앞에 `(?i)`. 엔진 = `regex-lite 0.1` | `br:550-558` · `br:598-602` | — | N | `regex_replace_with_captures_and_case_insensitive`(1218) |
| OPS-055 | 대소문자 변경 | `Case{scope, mode}`: Upper · Lower · Title(공백 `-` `_` `.` 뒤 첫 글자 대문자, 나머지 소문자) · Sentence(첫 글자만 대문자, 나머지 소문자) | `br:113-142` · `br:454-485` | — | N | `preview_case_modes_and_dir_whole_name`(1266) |
| OPS-056 | 텍스트 삽입 | `Insert{scope, text, at}` | `br:206-211` · `br:610-612` | — | N | `insert_at_arbitrary_position_with_clamp` |
| OPS-057 | 연번 붙이기 | `Number{scope, spec:NumberSpec{start, step, pad, at, prefix, suffix}}`. 값 = start + step×인덱스 · 0패딩(음수는 `-` 뒤 패딩) · `prefix+번호+suffix`를 한 덩어리로 삽입 | `br:144-155` · `br:613-622` | — | N | `number_with_wrapping_and_position`(1130) |
| OPS-058 | 날짜 삽입 | `Date{scope, spec:DateSpec{kind(Modified/Created), format, at, prefix, suffix}}`. 포맷 = `${토큰}`: `YYYY` `YY` `MMMM` `MMM` `MM` `M` `DDD`(요일 영문 3자) `DD` `D` `HH` `H` `mm` `m` `ss` `s` · 미지 토큰/닫힘 없음 = 리터럴. 기본 `${YYYY}-${MM}-${DD}`. **시각 0(미상) = 빈 문자열 → 그 항목 무변경**. 달력 계산은 자체 구현(`civil` — 외부 crate 0), `tz_min`(분) 가산. 월/요일 이름은 영문 고정 | `br:157-190` · `br:266-363` · `br:623-634` | 시각 원천 · TZ는 앱이 전달 | N(전달부 = P · OPS-308 · OPS-309) | `date_format_tokens_and_missing_time`(1151) |
| OPS-059 | 구식 날짜 포맷 이행 | `migrate_date_format`: `${`가 없으면 `yyyy`·`yy`·`MMM`·`MM`·`M`·`ddd`·`dd`·`d`·`HH`·`mm`·`ss`를 `${}` 문법으로 변환(긴 토큰 우선). 프리셋 로드와 폼 입력 양쪽에서 호출 | `br:365-401` · `bulk:335` | — | N | `date_format_tokens_and_missing_time` |
| OPS-060 | 구간 이동(코어만) | `Move{start(1기준), len, to_front}` — 이름부에서 잘라 맨 앞/뒤로. 범위 밖 = 무변경. **UI 카드는 07-18에 제거** — 코어 · 프리셋 파서는 유지, 복원 시 건너뜀 | `br:216-221` · `br:635-653` · `bulk:101-111` · `bulk:516-527` | — | N | `move_and_ext_edge_cases`(1241) |
| OPS-061 | 확장자 변경(코어만) | `ChangeExt{from, to}` — from 빈 값 = 모든 확장자 · 대소문자 무시 일치 · 폴더 제외 · 확장자 없는 파일 무변경 · to 빈 값 = 확장자 제거. UI 카드 제거됨(위와 동일) | `br:222-223` · `br:654-667` | — | N | `move_and_ext_edge_cases` |
| OPS-062 | 정규식 사전 검증 | `validate(ops) -> Result<(), (블록 순번, 메시지)>` — 정규식 블록의 빈 패턴/컴파일 오류 | `br:560-577` | — | N | `regex_replace_with_captures_and_case_insensitive` |
| OPS-063 | 충돌 검출 5종 | `conflicts(items:(부모, 현재, 새), exists) -> Vec<Conflict>`. 판정 순서: 무변경 → `None` · **Nested**(개명 예정인 조상 폴더가 같은 배치에 있음) → **Empty** → **Invalid**(`<>:"/\\\|?*` 포함 또는 끝이 `.`/공백) → **Duplicate**(같은 부모 · 대소문자 무시) → **Exists**(대소문자만 바뀐 경우는 제외하고 `exists(부모, 새)`) | `br:248-264` · `br:689-740` | 금지 문자 · 대소문자 무시가 Windows 규칙 | N*(OPS-303 · OPS-310) | `conflict_detection_four_kinds`(1378) · `nested_selection_blocks_descendants`(1409) |
| OPS-064 | 프리셋 직렬화 | 텍스트 1줄 = 블록 1개: `# nexa-dir rename preset v2` 머리말 + `op=<종류>\|k=v\|…`. 이스케이프 `\\` `\|` `\n`. 파싱은 관용(손상 줄 · 미지 종류 무시 · **상한 64블록**). v1 호환: `scope`/`mode`/`off`/`dir`/`pre`/`suf` 생략 = 기본 · `pos=prefix\|suffix` → `InsertAt::edge` · 연번 기본 start=1 · step=1 · pad=3 | `br:742-959` | — | N | `preset_round_trip_v2_and_v1_compat`(1287) |
| OPS-065 | 이름부/확장자 분리 규칙 | `split_stem`: 폴더 = 전체가 이름부 · 파일 = 마지막 `.` 기준(단, 맨 앞 `.`만 있는 숨김 파일은 전체가 이름부) | `br:487-496` | — | N | 간접 |

### 1-4. 일괄 이름 변경 대화상자 로직 — `nexa-app/src/bulkrename.rs`(+ `win.rs` 적용부)

| ID | 기능(사용자 관점) | 동작 상세 | 진입점 | OS 의존 | 분류 | 기존 테스트 |
|---|---|---|---|---|---|---|
| OPS-070 | 열기 · 반환 | `show(owner, targets:&[(경로, 폴더 여부)], font, tz_min) -> Option<Vec<(경로, 새 이름)>>`. 대상 없음/취소/변경 0 = `None`. 모달(소유 창 비활성화 → 자체 메시지 루프 → 복귀) | `bulk:2022-2252` | `CreateWindowExW` · `GetMessageW` 루프 · `EnableWindow` | A | 없음(파일 전체 테스트 0) |
| OPS-071 | 대상 수집 | 항목 = (부모, 이름, 폴더, 수정 ms, 생성 ms). `fs::metadata`의 `modified()`/`created()` 실패 = 0. 순서 = 호출자 전달 순(= `keyboard_targets`: 선택 삽입 순서, 선택 없으면 캐럿 1개) | `bulk:2054-2079` · `win:2739-2755` · `win:6426-6432` | `created()` 가용성 | N*(OPS-308) | 없음 |
| OPS-072 | 카드 스택 = 파이프라인 | 카드 1장 = 블록 1개. `harvest` = 카드를 위→아래로 `op_from_form` — **무효 폼은 건너뜀**: 치환(find 빈 값, 단 Entire는 허용) · 정규식(find 빈 값) · 삽입(text 빈 값). 대소문자 · 연번 · 날짜는 항상 유효. 날짜 포맷 빈 값 = 기본 포맷 | `bulk:254-382` | 컨트롤 메시지 | A(판정 로직 N) | 없음 |
| OPS-073 | 카드 추가 / 삭제 / 종류 변경 | `+` = 해당 카드 **아래에** 새 카드(종류 0 = 치환) + 보이게 스크롤 · `−` = 그 카드 삭제(**마지막 1장은 삭제 불가** — 버튼 비활성) · 종류 콤보 변경 = 그 카드 본문만 재구성 + 높이 재계산. 종류 6종 순서: 치환(텍스트) · 치환(정규식) · 텍스트 삽입 · 대소문자 변경 · 연번 붙이기 · 날짜 삽입 | `bulk:101-111` · `bulk:1835-1893` · `bulk:385-416` | — | A | 없음 |
| OPS-074 | 실시간 미리보기 | 폼 편집 통지마다 `refresh_preview`: 검증 오류 = `⚠ #순번: 메시지`(하단) · 충돌 행 = `⚠ 새 이름 (사유)` + 체크 없음 · 변경 행 = 새 이름 + 체크 · 무변경 = "이후" 빈 칸 + 체크 없음 · 건수 = `bulk.count`(체크된 변경 행 수). **[이름 변경] 활성 조건** = 블록 ≥1 AND 변경 ≥1 AND 검증 통과 AND 충돌 0. 존재 확인 = `Path::new(parent).join(name).exists()` | `bulk:418-514` | — | A(계산 N) | 없음 |
| OPS-075 | 행별 적용 제외 | 체크 열 클릭 = 그 항목 제외 토글(`excluded[item]`). 헤더 체크 = 전체 토글(그리드 행 상태로 전 항목 재동기). 표시 행 ↔ 항목 인덱스는 `order` 맵 경유 | `bulk:1937-1961` | — | A | 없음 |
| OPS-076 | 미리보기 정렬 | 헤더 정렬(다중 열): 열 1 = 이전 이름 · 열 2 = 이후 표시 문자열. 소문자 비교 · 안정 정렬. **표시 순서만** 바뀜 — 연번 순번은 항목 원래 순서 유지 | `bulk:471-497` · `bulk:1962-1966` | — | A(계산 N) | 없음 |
| OPS-077 | 프리셋 메뉴 | `…⌄` 메뉴 = 저장된 프리셋들 → 구분선(프리셋이 있을 때만) → "이름 변경 순서 저장…" → "이름 변경 순서 편집…". 프리셋 클릭 = 불러오기(카드 스택 전부 교체 · 카드 없는 종류는 건너뜀 · 결과가 비면 기본 카드 1장). 저장은 **블록이 1개 이상일 때만** 동작. 목록 = `data\renames\*.cfg` 이름순 · **최대 64개** | `bulk:233-236` · `bulk:1353-1422` · `bulk:1894-1936` | 파일 경로 구분자 | A + N | 없음 |
| OPS-078 | 프리셋 이름 입력 팝업 | 기본 이름 = `bulk.preset.savedSeq`(전체 선택 상태로 열림). 확인 = 금지 문자 `<>:"/\\\|?*` 제거 + trim · 빈 값이면 저장 안 함. 같은 이름 = 덮어씀(확인 없음) | `bulk:1424-1623` | 무캡션 팝업 · DWM 라운드 | A | 없음 |
| OPS-079 | 프리셋 관리 팝업 | 머리글 없는 지브라 목록 + 행별 빨간 ⊖. ⊖ = 화면에서 즉시 제거(스테이징) · [확인] = 스테이징된 파일 실제 삭제 · [취소]/닫기 = 폐기 | `bulk:1625-1809` | 좌동 | A | 없음 |
| OPS-080 | 날짜 포맷 도움말 | `?` 버튼 = 토큰 표 안내(고정 예시 + `bulk.date.fmtHelpNote`) — **OS 기본 `MessageBoxW`** | `bulk:1843-1865` | `MessageBoxW` | A(자체 그림 대화상자로 교체) | 없음 |
| OPS-081 | 카드 영역 스크롤 | 카드 합 높이 > 뷰포트일 때 세로 스크롤. 휠 48px/노치(분수 누적 + 고속 스크롤 가속) · 오버레이 썸(스크롤 직후 표시 → 900ms 뒤 소등 · 드래그 중 유지) · 썸 드래그. 커서가 폼 영역(x < 342)이면 대화상자로 버블된 휠도 호스트로 전달 | `bulk:542-806` · `bulk:1817-1827` | 자식 HWND 스크롤 | A | 없음 |
| OPS-082 | 적용 | [이름 변경] = 변경되고 제외되지 않은 항목만 `(경로, 새 이름)` 반환 → 앱이 **순차 `nexa_ops::rename`** · 실패는 개별 격리(건수만) · 성공분을 `MoveBatchOp` **1건**으로 기록(Ctrl+Z 한 번에 배치 전체 복귀) · 펼침 집합 접두 치환(`rename_expanded`) · 양쪽 재로드 · 타이틀 노트 `bulk.done`(+`bulk.fail`) | `bulk:1967-1994` · `win:6421-6472` | — | N(앱 배선) | 없음 |
| OPS-083 | 포커스 이동 · 닫기 | Tab = 컨트롤 **생성 순서**(`IsDialogMessageW`). 닫기 = 제목줄 X(`WM_CLOSE`) 또는 [취소]. Enter/Esc 전용 처리는 코드에 없다(**추정**: `IsDialogMessageW`가 보내는 IDOK/IDCANCEL은 통지 코드 0이라 `br_proc`의 `(id, 1)` 분기에 걸리지 않아 무동작) | `bulk:2232-2240` · `bulk:1828-2007` | `IsDialogMessageW` | A(§2-5 결정 필요) | 없음 |

### 1-5. 가상 파일시스템 — `nexa-vfs/src/lib.rs`

| ID | 기능 | 동작 상세 | 진입점 | OS 의존 | 분류 | 기존 테스트 |
|---|---|---|---|---|---|---|
| OPS-090 | 항목 모델 | `Entry{name, kind:FileKind, size, modified:Option<SystemTime>, attrs:u32, target:Option<String>}`. `attrs` = Windows 속성 비트(**비Windows = 0**). `target` = 표시명 ≠ 경로인 링크형 항목의 실경로 | `vfs:17-45` | 속성 비트 | N*(OPS-302 · OPS-304) | `entry_holds_kind`(vfs:333) |
| OPS-091 | 스트리밍 열거 | `read_dir_entries(path) -> io::Result<impl Iterator<Item=io::Result<Entry>>>`. 엔트리별 `Result`(한 건 실패가 전체를 막지 않음) · 메타데이터 실패 = 크기 0 · 시각 None · attrs 0. 메타데이터는 **링크 자체**(대상 미추적) | `vfs:74-101` | — | N* | `read_dir_entries_streams_local`(468) · `read_dir_entries_missing_path_errors`(589) |
| OPS-092 | 종류 판정 | `classify_kind(is_dir, is_symlink, attrs)`: `is_dir` 또는 `ATTR_DIRECTORY(0x10)` → `Dir`(폴더 정션/폴더 심링크 진입 가능) · 다음 `is_symlink` → `Symlink` · 나머지 `File`. 링크 표식은 `Entry::is_link()` = `ATTR_REPARSE_POINT(0x400)` | `vfs:33-60` | **비Windows는 attrs=0 → 폴더 심링크가 `Symlink`로 떨어짐** | **P**(OPS-302) | `classify_kind_dir_wins_over_link`(495) · `read_dir_entries_dir_junction_is_dir`(541 · `cfg(windows)`) |
| OPS-093 | 속성 비트 추출 | `#[cfg(windows)] file_attrs` = `MetadataExt::file_attributes()` / `#[cfg(not(windows))]` = **0 반환(스텁)** | `vfs:62-72` | `std::os::windows` | **P**(OPS-304) | 간접 |
| OPS-094 | 가상 최상위 "내 PC" | 센티널 `MY_PC = "::PC::"` · `is_virtual_root`. `drive_entries()` = `A:\`~`Z:\` metadata 프로브(이름 = `C:\` 절대 경로 → join이 부모를 대체). **비Windows = 빈 목록** | `vfs:103-132` | 드라이브 문자 | **P**(OPS-311) | `virtual_root_and_drive_entries`(312 — 드라이브 단언은 `cfg(windows)` 블록) |
| OPS-095 | 추가 루트(클라우드 연결 표시) | 전역 `EXTRA_ROOTS: RwLock<Vec<(라벨, 경로)>>` · `set_extra_roots`(전량 교체) · `extra_root_entries`(라벨 = name · 경로 = target). poison 내성(`into_inner`) | `vfs:134-162` | — | N | `extra_roots_roundtrip`(452) |
| OPS-096 | 클라우드 센티널 경로 | `::CLOUD:<인덱스>::<내부 경로>`. `cloud_parts` · `cloud_root` · `cloud_child` · `cloud_label` · `cloud_display`(센티널 → `라벨\Docs\a.txt` — **구분자 `\` 고정**) · `cloud_leaf` · `cloud_parent`(루트의 부모 = 내 PC) · `cloud_from_display`(역변환 — 긴 라벨 우선 · `\`·`/` 둘 다 허용 · 실경로 링크는 건너뜀) | `vfs:172-270` | 표시 구분자 | N*(OPS-313) | `cloud_path_parts_and_build`(349) · `cloud_display_leaf_and_parent`(367) · `cloud_from_display_roundtrip`(407) |
| OPS-097 | 클라우드 열거 콜백 | `set_cloud_lister(Box<dyn Fn(idx, inner) -> Option<Vec<Entry>> + Send + Sync>)` · `cloud_entries(path)`: 클라우드 경로가 아니면 `None` · 콜백 미등록/로딩 중 = 빈 목록 | `vfs:272-295` | — | N | `cloud_entries_without_lister_is_empty`(445) |
| OPS-098 | 저장소 공급자 추상(스텁) | `trait Provider { fn scheme(&self) -> &str }` — **구현체 · 사용처 없음**(Grep 결과 범위 크레이트 안에 구현 0) | `vfs:164-170` | — | N(미사용) | 없음 |
| OPS-099 | 압축 목록 계층(범위 밖) | `pub mod archive`(zip · tar · rar · 7z · cab · 스트림 — 압축 해제 없이 목록) · 외부 crate 0 · `cfg(windows)` 0 | `vfs:5-8` · `nexa-dir2/crates/nexa-vfs/src/archive/mod.rs:31-508` | 없음 | N | 45개(범위 밖) |

### 1-6. 인라인 트리 · 교차 선택 — `nexa-tree/src/lib.rs`(상세는 `13-dir2-panel-filelist.md`)

| ID | 기능 | 동작 상세(이식 관점) | 진입점 | OS 의존 | 분류 | 기존 테스트 |
|---|---|---|---|---|---|---|
| OPS-110 | 열기 + 가시성 필터 | `Tree::open`(전부 표시) · `open_filtered(path, show_hidden, show_dotfiles)`. 필터: 점 파일(`.` 시작) · 숨김 속성(`ATTR_HIDDEN 0x2`). 걸러진 항목은 노드 자체를 만들지 않는다 | `tree:144-213` | 숨김 = Windows 속성 비트 | N*(OPS-304) | `open_lists_top_level_folders_first`(991) · `open_filtered_excludes_dotfiles`(1186) · `open_missing_path_errors`(1203) |
| OPS-111 | 열거 3갈래 | 가상 루트 = 드라이브 + 추가 루트 / 클라우드 경로 = 콜백 캐시 / 그 외 = `read_dir_entries`(엔트리 오류는 건너뜀). 노드 경로 = `target` 우선, 없으면 `dir.join(name)` | `tree:225-286` | OPS-094 영향 | N* | `open_virtual_root_lists_drives`(1235 · `cfg(windows)`) |
| OPS-112 | 정렬 | `SortSpec{keys:Vec<(SortKey, desc)>, folders_first, case_sensitive}` · 기본 = 폴더 우선 + 이름 오름. 키: Name · Ext · Size(폴더 = 0) · Modified · Kind(종류 순위 → 확장자) · None(열거 순서). 동률 = 이름(대소문자 무시) → id. 대소문자 구분 = 코드포인트 순(대문자 그룹 상단). `set_sort` = 로드된 모든 폴더 재정렬 + 펼침 보존 | `tree:93-131` · `tree:288-383` · `tree:771-805` | — | N | 정렬 9개(`tree:1269-1382` · `1478`) |
| OPS-113 | 펼침 / 접힘 | `expand`(최초 = 지연 열거 · 하위 펼침 상태 복원) · `collapse` · `collapse_all` · `expand_path` · `is_expanded` · `loaded_child_count` · `filter_flags`. 반환 `RangeChange{start, removed, inserted}` | `tree:460-469` · `tree:559-649` | — | N | `expand_and_collapse_roundtrip`(1004) · `reexpand_restores_nested_expansion`(1041) · `expand_is_noop_on_file_or_twice`(1126) |
| OPS-114 | 교차 폴더 선택 | 삽입 순서 보존 집합. `select(Single/Toggle)` · `select_range`(anchor 유지) · `select_all_visible` · `clear_selection` · `selected_paths`(**작업 엔진 입력**) · `selected_path(i)` · 범위 밖 id 무시 | `tree:651-763` | — | N | `cross_folder_selection_ordered`(1139) · `range_and_select_all`(1163) |
| OPS-115 | 타입어헤드 접두 찾기 | `find_prefix(caret, prefix, FindScope::{GlobalFirst, CurrentLevel, VisibleStream})` — 대소문자 무시 · 할당 없는 문자 단위 비교 · wrap | `tree:133-142` · `tree:385-429` | — | N | `find_prefix_*` 3개(`tree:1401-1475`) |
| OPS-116 | 삭제 낙관 반영 | `remove_paths(paths) -> 매치 수`: 노드(+하위)를 roots/visible/children/선택에서만 제외(arena 유지). 경로 비교 = 끝 구분자 제거 + **소문자화** | `tree:471-509` | 대소문자 무시 | N*(OPS-303) | `remove_paths_hides_node_and_descendants`(1101) |
| OPS-117 | 경로 → 행 찾기 | `index_of_path(target)` — 끝 `\`·`/` 무시 + **ASCII 대소문자 무시** · `node_path` · `index_of` · `visible_id` | `tree:525-546` · `tree:759-762` | 대소문자 무시 | N*(OPS-303) | `index_of_path_and_expand_path`(1066) |
| OPS-118 | 가시 행 조회 · 규모 | `row`(클론) · `row_ref`(이름 빌림 — 셀 렌더 핫패스) · `visible_len` · 시각 = Unix ms(없음 = -1). 10만 노드 스케일 가드 | `tree:431-458` · `tree:511-523` · `tree:765-769` | — | N | `row_ref_matches_row_without_clone`(1253) · `large_tree_scale_ops_complete`(950) · `bench_100k_visible`(885 · `#[ignore]`) |

### 1-7. 공용 타입 — `nexa-core/src`

| ID | 기능 | 동작 상세 | 진입점 | OS 의존 | 분류 | 기존 테스트 |
|---|---|---|---|---|---|---|
| OPS-130 | 공용 타입 | `CORE_VERSION = env!("CARGO_PKG_VERSION")` · `enum FileKind{File, Dir, Symlink}` | `core:9-17` | — | N | `version_is_set` · `file_kinds_distinct` |
| OPS-131 | 비밀 문자열 | `Secret`: 직렬화 · `Display` 없음 · `Debug` = `Secret(***)`(길이도 숨김) · `Drop`에서 `write_volatile` 0 덮기 · `expose()`만 접근 · `take_from_string`(원본 소거) · `take_from_u16`(Win32 편집 컨트롤 버퍼용 — NUL까지 읽고 버퍼 소거) · `zeroize_bytes/u16/string`. 외부 crate 0 | `secret:20-115` | `take_from_u16`은 UTF-16 버퍼 전제(코드 자체는 중립) | N | 4개(`secret:121-163`) |

### 1-8. 빌드 구성

| ID | 항목 | 내용 | 근거 | 분류 |
|---|---|---|---|---|
| OPS-140 | 워크스페이스 선언 | `resolver = "2"` · 멤버 7(core · vfs · tree · gui · ops · term · app) · `version = "0.22.0"` · `edition = "2021"` · 라이선스 `LicenseRef-PolyForm-Noncommercial-1.0.0` · 린트(`unsafe_op_in_unsafe_fn = allow` · `redundant_clone` · `inefficient_to_string` = warn) · 릴리스 프로필 `opt-level=3` · `lto="fat"` · `codegen-units=1` · `panic="abort"` · `strip="symbols"` | `nexa-dir2/Cargo.toml:1-46` | N(라이선스 · 버전 · 프로필은 dir3 기준 문서에서 재결정) |
| OPS-141 | 크레이트 의존 그래프 | `nexa-core`: 의존 0 → `nexa-vfs`: core → `nexa-tree`: core + vfs · `nexa-ops`: `regex-lite = "0.1"`만(core 미의존) · dev-dependencies 없음. Windows crate는 `nexa-app`에만(`[target."cfg(windows)".dependencies]`) | `nexa-dir2/crates/nexa-{core,vfs,tree,ops}/Cargo.toml` · `nexa-dir2/crates/nexa-app/Cargo.toml:20-22` | N |
| OPS-142 | 크로스 플랫폼 CI | `core` 잡이 ubuntu · macOS에서 `cargo test --workspace` 실행(이미 회귀 감시 중). Windows 잡 = 테스트 + 릴리스 빌드 + 예산 게이트 | `nexa-dir2/.github/workflows/ci.yml:11-21` | N |

### 1-9. 문서에서 계승할 규약

| ID | 규약 | 내용 | 근거 | 분류 |
|---|---|---|---|---|
| OPS-150 | 이벤트 · 스레딩 모델 | UI 스레드 1개 + 워커(열거 · 전송 · 감시). 워커 → UI 통지는 **게시 메시지 단일화** · 취소 = 세대 카운터/`AtomicBool` · 상태의 진실 원천 = 코어(`nexa-tree`) | `nexa-dir2/docs/01-architecture.md:51-54` | P(게시 수단 — OPS-312) |
| OPS-151 | 포터블 영속 | 영속물은 exe 옆 `data\` 단일 원천(쓰기 불가 시 `%LOCALAPPDATA%\NexaDir\data`) · 아이콘/기본 언어팩은 `include_bytes!` | `nexa-dir2/docs/01-architecture.md:56-59` · `nexa-dir2/crates/nexa-app/src/config.rs:362-384` | P(경로 규칙 — 설정 인벤토리 소관) |
| OPS-152 | 세션 저장 코얼레싱 | 변경 지점은 **플래그만**(7곳) → 단일 길목 `update_status`에서 수거(`\|` 비단락 OR) → 1000ms 디바운스 재무장 → 만료 시 **현재 상태 1회** 직렬화 → 원자적 쓰기. 종료 저장과 같은 스냅샷 함수 공유 | `nexa-dir2/docs/20-session-coalescing.md:8-94` | N(§5-3 — `nexa-conf::SaveScheduler`가 대응) |
| OPS-153 | 일괄 이름 변경 v2 설계 | Path Finder 6동작 대조 설계. **문서와 코드가 달라진 곳**은 §6 OPS-410에 정리 | `nexa-dir2/docs/22-batch-rename-v2.md:1-115` | N |

### 1-10. 플랫폼 중립도 — `cfg(windows)` 전수 · 스텁 · 테스트 수

`cfg(` Grep 결과(범위 4크레이트 본문 · 테스트 전부):

| 위치 | 종류 | 내용 |
|---|---|---|
| `vfs:63` / `vfs:69` | **본문 분기(유일)** | `file_attrs` — Windows = 속성 비트 / 그 외 = `0` |
| `vfs:316` | 테스트 내부 블록 | 드라이브 1개 이상 단언 |
| `vfs:539` | 테스트 전체 | 폴더 정션(`mklink /J`) 열거 |
| `ops:1057` | 테스트 전체 | 독점 열기(`share_mode(0)`) 파일이 든 폴더 이동 |
| `tree:1233` | 테스트 전체 | 내 PC 드라이브 열거 |
| 그 외 `cfg(test)` | — | 모듈 게이트뿐 |

`nexa-ops` · `nexa-tree` · `nexa-core` 본문에는 `cfg(windows)`가 **0건**이다. 다만 "분기가 없다 = 중립"은 아니다 — 아래는 **분기 없이 Windows 의미가 박혀 있어 비Windows에서 비어 있거나 틀어지는 지점**이다(모두 §4에 대응 방안).

| 스텁/공백 | 비Windows에서의 결과 | 근거 |
|---|---|---|
| `file_attrs` = 0 | 숨김 속성 필터 무동작 · 링크 표식(`is_link`) 항상 false | `vfs:69-72` · `tree:160` · `vfs:42-44` |
| `classify_kind` | 폴더 심볼릭 링크가 `Symlink` → 펼침/진입 불가 | `vfs:52-60` · `vfs:84-90` |
| `drive_entries` | 빈 목록 → "내 PC"에 추가 루트만 표시 | `vfs:116-132` |
| `same_volume` | 항상 true → 다른 마운트로의 이동이 `rename` 실패(EXDEV)로 끝남(코드 추적) | `ops:79-91` · `ops:373-390` |
| 경로 대소문자 무시 비교 | 대소문자 구분 FS에서 다른 폴더/파일을 같은 것으로 취급 | `ops:57-75` · `tree:475-479` · `tree:540-546` · `br:697-735` |
| `Provider` 트레이트 | 구현체 0(전 플랫폼 스텁) | `vfs:167-170` |

기존 단위 테스트(범위 크레이트 · `#[test]` 개수):

| 크레이트/파일 | 테스트 수 | 비Windows 실행 수 | 범위 |
|---|---|---|---|
| `nexa-ops/src/lib.rs` | 16 | 15(1개 `cfg(windows)` · 교차 볼륨 1개는 둘째 볼륨이 없으면 조기 반환) | 순번 명명 · 같은 폴더 규칙 · 재귀 복사 · 충돌 · 순환 이동 · 이벤트 · 취소 · 스테이징 교체 · 이름 바꾸기 · 생성/삭제 · 크기 · 교차 볼륨 정리 |
| `nexa-ops/src/history.rs` | 10 | 10 | 스택 규약 5 · 연산 왕복 5 |
| `nexa-ops/src/batch_rename.rs` | 14 | 14 | 파이프라인 순서 · 스코프 · 모드 · 위치 클램프 · 연번 · 날짜 · 정규식 · 프리셋 왕복 · 충돌 5종 |
| `nexa-vfs/src/lib.rs` | 11 | 10 | 가상 루트 · 클라우드 경로 · 추가 루트 · 열거 · 종류 판정 |
| `nexa-tree/src/lib.rs` | 26 | 24(1개 `cfg(windows)` · 1개 `#[ignore]` 벤치) | 펼침 · 선택 · 필터 · 정렬 · 타입어헤드 · 경로 매칭 · 낙관 제거 · 스케일 |
| `nexa-core/src` | 6 | 6 | 버전 · 종류 · 비밀 소거 |
| **합계** | **83** | **79** | |
| `nexa-app/src/bulkrename.rs` | 0 | — | 대화상자 로직은 테스트 없음 |

복사 이식 판단:

| 크레이트 | 판단 |
|---|---|
| `nexa-core` | **그대로 복사.** 변경 불필요 |
| `nexa-ops` | **복사 + 패치 4건**: 볼륨 판정(OPS-301) · 경로 대소문자(OPS-303) · 복사 메타데이터/심링크(OPS-305 · OPS-306) · 자기 하위 복사 방어(OPS-404). `history.rs` · `batch_rename.rs`는 로직 변경 없이 복사(충돌 검출의 대소문자 규칙만 OPS-303과 함께 매개변수화) |
| `nexa-vfs` | **복사 + 패치 4건**: 심링크 종류 판정(OPS-302) · 숨김 속성(OPS-304) · 드라이브/볼륨 열거(OPS-311) · 표시 구분자(OPS-313). `archive/`는 그대로 |
| `nexa-tree` | **복사 + 패치 1건**: 경로 비교 대소문자(OPS-303). 숨김 필터는 vfs 패치에 따라온다 |
| `bulkrename.rs` | 화면부 전부 재작성(A). **옮겨 쓸 순수 로직**: `op_from_form`의 유효성 규칙 · `kind_index_of` · `card_h` · `refresh_preview`의 계산부 · `preset_names` · 프리셋 메뉴 인덱스 해석 — 화면과 분리해 테스트 가능한 모듈로 추출 권장 |

---

## 2. 화면 · 컨트롤 배치

담당 범위에서 화면이 있는 것은 일괄 이름 변경 대화상자 계열 3개 창뿐이다. 좌표는 96dpi 기준 클라이언트 px(코드 상수 그대로).

### 2-1. 일괄 이름 변경 창(`NexaBulkRename`)

창: 제목 `bulk.title` · 스타일 팝업 + 캡션 + 시스템 메뉴(크기 조절 없음) · 확장 스타일 대화상자 모달 프레임 · **클라이언트 880 × 620** · 소유 창 중앙 · 아이콘 32px(`bulk:42-47` · `bulk:2081-2112`). 배경 = 시스템 창 색(라이트 고정 — `bulk:2008-2013`).

상수: `PAD = 12` · `FORM_W = 320` · `HOST_BAR = 10` · 하단 기준선 `by = 620 − 12 − 21 = 587` · 우측 영역 시작 `lx = 12 + 320 + 12 = 344`(`bulk:42-46` · `bulk:548` · `bulk:2115` · `bulk:2152`).

| 영역 | 컨트롤 | 위치(x, y, w, h) | 비고 | 근거 |
|---|---|---|---|---|
| 좌 · 카드 영역 | 카드 호스트(스크롤 뷰포트) | (12, 12, 330, 567) | 폭 = `FORM_W + HOST_BAR`(오른쪽 10px = 썸 자리) · 높이 = `by − 8 − PAD` · 자식 클립 · Tab 진입 허용 | `bulk:748-780` · `bulk:2119` |
| 좌 · 카드 영역 | 카드(그룹 카드) N장 | 호스트 기준 (0, −scroll + 누적, 320, `card_h(kind)`) · 카드 간격 8 | 모서리 반경 8 · 타이틀 밴드 34 | `bulk:385-416` · `bulk:531-540` · `bulk:810-826` |
| 우 · 미리보기 | 그리드(체크 / 이전 / 이후) | (344, 12, 524, 567) | 열 폭: 체크 = `글꼴 높이(최소 10) + 14` · 이전/이후 = 나머지 반씩. 체크 열 머리글 = 전체 토글 체크박스(제목 없음). 행 높이 = `20 × dpi/96`(최소 14 — 파일 목록 고밀도와 동일). 열 경계 드래그 · 머리글 정렬 | `bulk:2165-2191` · `bulk:1476-1481` |
| 하단 좌 | 프리셋 메뉴 버튼 `… ⌄` | (12, 589, 48, 자동) | y = `CLIENT_H − PAD − 21 + 2`. 항목이 바뀌면 **컨트롤을 다시 만든다** | `bulk:1410-1421` |
| 하단 좌 | 변경 건수 글 | (70, 593, 262, 18) | x = `PAD + 48 + 10` · `bulk.count` | `bulk:2192-2204` |
| 하단 중 | 검증 오류 글 | (344, 593, 304, 18) | 폭 = `CLIENT_W − lx − PAD − 220` · `⚠ #n: 메시지` | `bulk:2153-2164` |
| 하단 우 | [취소] · [이름 변경] | y = 589 · 자동 크기 · 우측 정렬 | [이름 변경](기본 버튼 · accent)이 맨 오른쪽(`x = 880 − 12 − 폭`) · [취소]는 그 왼쪽 8px 간격. 버튼 높이 = 글꼴 + 상하 2px · 좌우 여백 16 | `bulk:2122-2149` · `bulk:1497-1517` · `nexa-dir2/crates/nexa-app/src/ctl/button.rs:1-57` |

카드 높이 `card_h = 34 + 8 + 28 × 마지막 행 + 23 + 8`(`bulk:531-540`):

| 종류 | 마지막 행 | 높이 |
|---|---|---|
| 0 치환(텍스트) | 4 | 185 |
| 1 치환(정규식) | 3 | 157 |
| 2 텍스트 삽입 | 2 | 129 |
| 3 대소문자 변경 | 1 | 101 |
| 4 연번 붙이기 | 4 | 185 |
| 5 날짜 삽입 | 4 | 185 |

카드 타이틀 밴드(높이 34 · 밴드 색 = `sel_bg`): 종류 콤보 (x = 8, 세로 중앙, **w 170 · h 24**) · `+` 버튼 (x = `우측 − 8 − 2×ib − 6`) · `−` 버튼 (x = `우측 − 8 − ib`) · `ib` = 글꼴 높이(최소 10) 지름 · 세로 중앙. ±는 PNG(32px 원본을 늘려 그림 — 추가 = 초록 · 삭제 = 빨강 · 비활성 = 회색 4종). 밴드 위 컨트롤 필 색 = `#D2D6DC`(밴드보다 한 단계 진한 회색)(`bulk:53-58` · `bulk:827-877`).

카드 본문 격자(`bulk:886-954`): `bx = 10` · `bw = 300` · 첫 행 y = 본문 상단 + 8 · **행 피치 28** · 라벨 열 폭 `lbl_w = clamp(현재 언어 라벨 15종 실측 최대, 56, 140) + 6` · 컨트롤 열 `cx = bx + lbl_w + 6` · `cw = bw − lbl_w − 6` · 둘째 라벨 폭 `lbl2_w = clamp(max(접두, 접미), 28, 90) + 6`. 라벨은 **우측 정렬**(콜론이 컨트롤에 붙는 구도) · 컨트롤 높이 자동(= 글꼴 높이 + 8 — `nexa-dir2/crates/nexa-app/src/ctl/style.rs:75-86`).

| 종류 | 행 0 | 행 1 | 행 2 | 행 3 | 행 4 |
|---|---|---|---|---|---|
| 공통 | `적용 대상:` 콤보(cx, cw) — 이름 / 이름+확장자 / 확장자 / 확장자(점 포함) · 기본 0 | | | | |
| 0 치환(텍스트) | 공통 | `모드:` 콤보 — 모든 일치 / 첫 번째 / 마지막 / 전체 교체 · 기본 0 | `대소문자 일치:` 체크박스(2상태 · 라벨 없음 · 기본 꺼짐) | `찾기:` 글상자(cx, cw) | `바꾸기:` 글상자 |
| 1 치환(정규식) | 공통 | `대소문자 일치:` 체크박스 | `정규식:` 글상자 | `바꾸기:` 글상자 | — |
| 2 텍스트 삽입 | 공통 | `위치:` 스핀(cx, w 70 · 초기 0 · 0~999) + 세그먼트(cx+76, w = cw−76 · `→ abc` / `← abc` · **기본 1 = 뒤에서**) | `텍스트:` 글상자 | — | — |
| 3 대소문자 변경 | 공통 | `케이스:` 세그먼트(cx, cw · `AB CD` / `Ab Cd` / `Ab cd` / `ab cd` · 기본 0) → Upper / Title / Sentence / Lower | — | — | — |
| 4 연번 붙이기 | 공통 | `자릿수:` 콤보 6항목(`1, 2, 3, 4…` ~ `000001…`) · **기본 인덱스 2 = 3자리** | `위치:` 스핀 + 세그먼트(기본 1) | `시작 값:` 스핀(w 70 · 초기 1 · −9999~9999) + `접두:` 라벨(x2 = cx+76, lbl2_w) + 글상자(x2+lbl2_w+6, w = cw−76−lbl2_w−6) | `증가 값:` 스핀(초기 1 · −9999~9999) + `접미:` 라벨 + 글상자 |
| 5 날짜 삽입 | 공통 | `종류:` 콤보 — 수정한 날짜 / 만든 날짜 · 기본 0 | `위치:` 스핀 + 세그먼트(기본 1) | `접두:` 글상자(cx, w 70) + `접미:` 라벨 + 글상자 | `포맷:` 글상자(cx, w = cw−ib−6 · 초기값 `${YYYY}-${MM}-${DD}`) + `?` 원형 버튼(cx+cw−ib, 세로 중앙, 지름 ib) |

근거: `bulk:955-1246`. 컨트롤 id는 카드 안 지역 id(`bulk:49-99`) — 통지는 "보낸 컨트롤의 부모 = 카드"로 카드를 식별한다(`bulk:1831-1833`).

카드 영역 스크롤(`bulk:548-746`): 휠 1노치 = 48px(분수 누적 + 고속 스크롤 가속기) · 썸 폭 6(드래그 중 10) · 오른쪽에서 2px 띄움 · 최소 길이 24 · 색 = `border` · 드래그 중 트랙 = `sel_bg` · 스크롤 직후 표시 후 **900ms** 뒤 소등(드래그 중 유지) · 썸은 표시 중일 때만 잡힌다 · 새 카드 추가 시 보이도록 스크롤 보정(`bulk:782-806`).

### 2-2. 프리셋 이름 입력 팝업(`NexaPromptName`)

창 **360 × 150** · 캡션 없음(팝업 + 1px 테두리) · 소유 창 중앙 · Win11 라운드 코너 · `PAD = 20`(`bulk:1519-1560`).

| 컨트롤 | 위치 | 비고 | 근거 |
|---|---|---|---|
| 라벨 | (20, 20, 320, 자동) | 문구 = `bulk.preset.savedSeq`(기본 이름과 같은 키를 라벨로도 쓴다) · 좌측 정렬 | `bulk:1562-1574` |
| 글상자 | (20, 48, 320, 자동) | 기본 이름이 **전체 선택**된 채 열림 · 초기 포커스 | `bulk:1575-1578` · `bulk:1609` |
| [취소] · [확인] | y = 106(`150 − 20 − 24`) · 우하단 정렬 | [확인](기본 버튼)이 맨 오른쪽 · 간격 8 | `bulk:1579-1606` |

### 2-3. 프리셋 관리 팝업(`NexaManagePresets`)

창 **340 × 360** · 캡션 없음 · 소유 창 중앙 · 라운드 코너 · `PAD = 20`(`bulk:1696-1738`).

| 컨트롤 | 위치 | 비고 | 근거 |
|---|---|---|---|
| 목록(그리드) | (20, 20, 300, 276) | 머리글 없음 · 지브라 · 외곽선 · 열 1개(폭 300) · 행 높이 = 파일 목록과 동일 · 행 오른쪽 빨간 ⊖ | `bulk:1740-1760` |
| [취소] · [확인] | y = 316(`360 − 20 − 24`) · 우하단 정렬 | [확인] = 삭제 확정 | `bulk:1761-1787` |

### 2-4. 탭 순서

`IsDialogMessageW` 기본 = **생성 순서**(`bulk:2232-2237`). 본 창의 생성 순서(`bulk:2119-2227`): 카드 호스트(카드별: 종류 콤보 → `+` → `−` → 본문 컨트롤을 행 순서대로) → [이름 변경] → [취소] → (검증 오류 글) → 미리보기 그리드 → (건수 글) → 프리셋 메뉴 버튼. 카드 종류를 바꾸면 본문 컨트롤이 다시 만들어져 그 카드의 ± 뒤에 온다. 라벨 · 글은 포커스를 받지 않는다. 팝업 2종: 글상자/목록 → [확인] → [취소].

### 2-5. 단축키 · 결정이 필요한 것

| 키 | 동작 | 근거 |
|---|---|---|
| Ctrl+Shift+R | 일괄 이름 변경 열기(선택 없으면 타이틀에 `bulk.noSelection`) | `nexa-dir3/docs/port/12-dir2-win-c.md`(WINC-121) · `win:6436-6441` |
| Ctrl+Z / Ctrl+Y | 실행 취소 / 다시 실행 | `win:3512-3524` |
| Del / Shift+Del | 휴지통 삭제 / 완전 삭제(확인 창 · 기본 = 취소) | `win:3714-3765` |
| 세그먼트 ←/→ · 스핀 ↑/↓ · 버튼 Space/Enter | 컨트롤 내부 키 | `nexa-dir2/crates/nexa-app/src/ctl/segmented.rs:13` · `spin.rs:11` · `button.rs:12` |

결정 필요: dir2의 세 창은 Enter(기본 버튼) · Esc(취소)를 창 차원에서 처리하지 않는다(OPS-083 — 추정). nexa-sql 기준 창은 "Esc = 닫기"가 골격이다(`nexa-sql/crates/nexa-sql/src/input_win.rs:5`). **dir3에서 Esc = 취소 · Enter = 기본 버튼을 넣을지**(기준 UI 따름) **dir2대로 둘지** 구현 전에 정한다.

---

## 3. nexa-ui 매핑

`nexa-ui/crates/nexa-ctl/src`의 `pub struct/enum/trait`를 전수 Grep해 확인했다.

| ID | dir2 컨트롤 · 호출 | nexa-ui 대응 | 상태 | 필요한 API / 비고 |
|---|---|---|---|---|
| OPS-201 | `ctl::combobox`(드롭 목록 · `NXCB_GETSEL/SETSEL`) | `Combo` — `nexa-ui/crates/nexa-ctl/src/controls/combo.rs:534`(`new(items, selected)` · `selected_index` · `select_value`) | **있음** | 인덱스 기반 선택 설정(`select_value`는 값 문자열 기준 — 항목 값에 인덱스 문자열을 쓰면 된다) |
| OPS-202 | `ctl::checkbox`(2상태 · 라벨 없음) | `Checkbox` — `controls/checkbox.rs:19`(`new(label, checked)` · `is_checked` · `set_checked` · `take_toggled`) | **있음** | — |
| OPS-203 | `ctl::textbox`(한 줄 글상자 · `EN_CHANGE`) | `TextBox` — `controls/textbox.rs:350`(기본 `multiline: false` — `textbox.rs:838`) | **있음** | 변경 통지 폴링 방식은 nexa-sql `input_win.rs` 사용례를 따른다 |
| OPS-204 | `ctl::spin`(숫자 스피너 · 범위 · ⌃⌄ · ↑/↓ · 타이핑 · 포커스 이탈 시 클램프) | 없음(Grep `spin\|stepper\|numeric` 0건) | **추가 필요** | `Spin::new(value, min, max)` · `value()` · `set_value()`(통지 없음) · `take_changed()` · 숫자 우측 정렬 · 분리된 ⌃⌄ 블록(폭 = 높이의 2/3 · 최소 14 · 간격 4) · 경계 도달 방향 비활성 · 반경 6 — 계약 `nexa-dir2/crates/nexa-app/src/ctl/spin.rs:1-48` |
| OPS-205 | `ctl::segmented`(가로 세그먼트 택일 · ←/→) | 없음. `RadioGroup`(`controls/radio.rs:38`)은 **세로 나열 라디오**라 모양이 다르다 | **추가 필요** | `Segmented::new(labels, selected, opts{corner=6, gap=0})` · `selected()` · `set_selected()`(통지 없음) · `take_changed()` · gap 0 = 연회색 컨테이너 + 선택 = accent 라운드 필(흰 글자) · 높이 자동 = 글꼴 + 상하 2px. 화살표 글리프는 dir2가 **Segoe MDL2 Assets**(Windows 전용 글꼴)로 그림 → 벡터 도형으로 교체 — 계약 `ctl/segmented.rs:1-60` |
| OPS-206 | `ctl::groupcard`(타이틀 밴드 + 본문 컨테이너 · 타이틀 자리에 자식 배치) | 없음 | **추가 필요**(또는 그리기 헬퍼) | `GroupCard{corner, title_h, body_h}` · `title_rect()` · `body_rect()` · 타이틀 밴드 = `sel_bg` + 하단 1px 구분선 · 본문 = `bg` · 외곽 1px. nexa-ctl은 자식 창 개념이 없으므로 **사각형 계산 + 페인트만** 제공하면 된다 — 계약 `ctl/groupcard.rs:1-50` |
| OPS-207 | `ctl::iconbutton`(원형 이미지 버튼 · PNG 2장[활성/비활성] · `?` 글리프 · 활성 토글) | `Button::icon(image)` + `ButtonMode::Image(ImageFit)` — `controls/button.rs:48-63` · `button.rs:165` · 활성 = `Control::set_enabled`(`controls/mod.rs:295`) | **부분** | ① 이미지 교체 setter 없음(`with_image` 빌더뿐 — `button.rs:244`) → 비활성 이미지 전환용 `set_image` 또는 비활성 이미지 슬롯 추가 ② `ImageFit`에 Stretch 없음(Contain/Cover) — 정사각 PNG라 Contain으로 동치 ③ `?` 도형은 `GlyphKind`에 없음(`controls/glyphs.rs:13-39`) → Help 글리프 추가 |
| OPS-208 | `ctl::label`(폼 라벨 · 우/좌 정렬 · 마우스 투명) | 컨트롤 없음 — `DrawCtx` 글자 그리기로 대체(nexa-sql 창들이 쓰는 방식) | **추가 불필요**(그리기) | 라벨 열 폭은 현재 언어 실측 최대치로 계산(§2-1) — `DrawCtx`의 글자 폭 측정 사용 |
| OPS-209 | `ctl::grid`(평면 그리드: 체크 열 + 머리글 전체 체크 · 열 리사이즈 · 다중 열 정렬 · 행 선택 · 지브라/머리글 숨김/외곽선 · 행 ⊖ 마크 · 오버레이 스크롤바) | `TreeGrid`(`controls/tree.rs:675`)는 트리 열 + 정렬 배지만 — **체크 열 · ⊖ 마크 · 지브라 없음**(`tree.rs` 안에 check/zebra 식별자 0건). 계획상 `nexa-grid` 크레이트(`nexa-ui/docs/21-grid-family.md:26-31` · `:87-91`의 `CellKind::Check`) — **미구현**(nexa-sql도 앱 쪽 `grid.rs`로 대신함: `nexa-sql/crates/nexa-sql/src/grid.rs:1-3`) | **추가 필요** | `GridRow{check: Option<bool>, cells}` · `GridOpts{no_header, zebra, outline, row_h, mark: Check\|Minus}` · `set_rows` · `row_check(i)` · `sort_spec()` · 통지: 토글(행 인덱스 또는 "전체") · 정렬 변경 · 선택 변경 · 열 최소 폭 40 · 썸 6/10 · 페이드 900ms — 계약 `nexa-dir2/crates/nexa-app/src/ctl/grid.rs:1-130`. 파일 목록 그리드(PANEL 문서)와 같은 엔진을 쓰도록 `nexa-grid`로 합치는 것이 nexa-ui 문서의 방향이다 |
| OPS-210 | `ctl::menubutton`(`… ⌄` 필 버튼 + 액션 드롭다운 · `"-"` = 구분선 · 선택 상태 없음) | 단일 컨트롤 없음. 조합 재료: `Button` + `ContextMenu`(`controls/ctxmenu.rs:272`) + `CtxItem::Separator`(`ctxmenu.rs:133`) | **추가 필요**(조합 컨트롤) | `MenuButton::new(items)` · `set_items()`(dir2는 항목 불변이라 재생성 — 새 컨트롤은 교체 가능하게) · `take_picked() -> Option<id>` · 팝업은 맨 위 레이어에서 다시 그림(콤보와 같은 규약) — 계약 `ctl/menubutton.rs:1-46` |
| OPS-211 | `ctl::button`(일반 / Default[accent] / 비활성 · 자동 폭) | `Button::new(label)` + `ButtonTone::Accent`(`controls/button.rs:75-87`) + `set_enabled` · `take_clicked`(`button.rs:324`) | **있음** | 자동 폭(라벨 + 좌우 16)은 호출 측에서 글자 폭 실측 후 bounds 지정 |
| OPS-212 | 카드 호스트(자식 스크롤 뷰포트 · 오버레이 썸 · 휠 가속) | `ScrollBars`(`controls/scroll.rs:320`) · `WheelAccum`(`nexa-ctl/src/event.rs`) · `ScrollAccel` · `FastScroll`(`scroll.rs:56` · `:258`) | **부분** | 스크롤바 · 휠 누적 · 가속은 있음. **내용을 오프셋 · 클립해 그리고 입력 좌표를 변환하는 컨테이너는 없다** → 대화상자에서 직접 구현하거나 `ScrollView` 헬퍼 추가 |
| OPS-213 | `MessageBoxW`(날짜 도움말 · 완전 삭제 확인) | 범용 메시지/확인 대화상자 없음 — `nexa-dlg`는 `FilePicker` 전용(`nexa-ui/crates/nexa-dlg/src/lib.rs:154`) | **추가 필요** | 제목 · 본문(여러 줄) · 버튼 N개 · 기본/취소 버튼 지정. 전송 충돌 4버튼 대화상자(OPS-004)와 같은 컨트롤로 통합 가능 — 다른 인벤토리(`11-dir2-win-b.md`)와 중복 확인 |
| OPS-214 | 네이티브 `STATIC`(건수 · 검증 오류 글) · `WM_CTLCOLOR*` 배경 맞춤 | `DrawCtx` 글자 그리기 + `Theme` | **추가 불필요** | 라이트 고정 → 테마 토큰으로 |
| OPS-215 | 모달 창 3종(자체 메시지 루프 · 소유 창 비활성) | 창 골격은 nexa-sql 방식: winit 창 + `Presenter` + 컨트롤 직접 배치(`nexa-sql/crates/nexa-sql/src/input_win.rs:1-30`) | **패턴 차용** | 모달성(소유 창 입력 차단) · 중앙 배치 · 캡션 없는 라운드 팝업은 OS별 창 속성 — 창 인벤토리 소관 |
| OPS-216 | 진행 창 세그먼트 바(`dialog::SegItem{size, done, status}`) ← `nexa_ops::Event` | 진행 바 컨트롤 없음(Grep `progress` = 토큰 보간 함수뿐) | **추가 필요** | 항목별 크기 비율 세그먼트 + 상태(대기/진행/완료/건너뜀/실패) 색. 엔진 쪽 계약은 OPS-006 그대로 — 화면은 `11-dir2-win-b.md` 소관 |
| OPS-217 | 원자적 파일 저장(`config::save`) · 디바운스 저장 | `nexa_conf::write_atomic`(`nexa-ui/crates/nexa-conf/src/lib.rs:125`) · `SaveScheduler`(`:174`) | **있음** | §5-3 |
| OPS-218 | 드라이브/볼륨 · 숨김 판정 · 이름 검증 · 로컬 시각 | `nexa_fs::drives`(`nexa-ui/crates/nexa-fs/src/lib.rs:416`) · `is_hidden_meta`(`:69-79`) · `naming::validate`(`:531`) · `local_time`(`:758`) | **있음(재사용 후보)** | nexa-fs는 dir2의 vfs/tree를 **중립화해 추출한 것**(`nexa-fs/src/lib.rs:9-10`). dir3는 Tree(arena) 모델을 유지해야 하므로 vfs/tree를 복사하되 OS 판정만 nexa-fs에 위임하는 구성이 가능(§4) |

---

## 4. OS 분기점

| ID | 분기점 | Windows 현 구현 | macOS | Linux | 비고 |
|---|---|---|---|---|---|
| OPS-301 | **볼륨(마운트) 판정 · 교차 볼륨 이동** | 드라이브/UNC 접두 비교(`ops:79-91`) → 다르면 복사 후 삭제 | `std::os::unix::fs::MetadataExt::dev()`로 원본과 **대상 부모**의 장치 번호 비교 | 동일(`dev()`) | 현 코드는 비Windows에서 항상 "같은 볼륨" → `/Volumes/USB` · `/mnt/x`로의 이동이 `rename` 오류(EXDEV)로 끝난다(코드 추적). 보강: `rename`이 교차 장치 오류(`raw_os_error() == 18`)면 `move_by_copy`로 폴백(판정 실패 대비 이중 방어). DnD 기본 동작(같은 볼륨 = 이동 · 다른 볼륨 = 복사 — `nexa-dir2/crates/nexa-app/src/dnd.rs:192-203`)도 이 함수에 의존 |
| OPS-302 | **심볼릭 링크 종류 판정** | 속성 `DIRECTORY`로 폴더 링크/정션을 `Dir`로(`vfs:52-60`) · 링크 표식 = `REPARSE_POINT` | `file_type().is_symlink()`면 `fs::metadata(경로)`(대상 추적)로 폴더 여부 확인 → `Dir` + 합성 비트 `0x10 \| 0x400` 설정 · 끊긴 링크 = `Symlink` | 동일 | nexa-fs가 이미 같은 방식(`link_aware_meta` — `nexa-ui/crates/nexa-fs/src/lib.rs:139-145`). 링크일 때만 추가 stat(성능). 크기 · 시각을 링크 자체 것으로 둘지 대상 것으로 둘지 정한다(dir2 = 링크 자체) |
| OPS-303 | **경로 대소문자 의미론** | 전부 대소문자 무시(`ops:57-75` · `tree:475-479` · `tree:540-546` · `br:697-735`) | 기본 APFS = 비구분(구분 볼륨도 존재) | ext4 등 = **구분** | 한 곳에 `fs_case_insensitive()`(Windows · macOS = true / 그 외 = false)를 두고 비교 함수 4곳을 이 값으로 분기. 볼륨별 정밀 판정(macOS `pathconf(_PC_CASE_SENSITIVE)`)은 후속. 같이 고칠 것: **대소문자만 바꾸는 이름 변경**(OPS-401). 끝 구분자 제거에 `\`를 넣는 것도 Unix에서는 틀림(`\`는 파일명 문자) → `std::path::MAIN_SEPARATOR` 기준 |
| OPS-304 | **숨김 판정** | 속성 `HIDDEN(0x2)`(`tree:144-165`) + 점 파일 별도 토글 | 점 파일 + `UF_HIDDEN`(`st_flags & 0x8000` — 예: `~/Library`) → 합성 `0x2` 비트 | 점 파일만(숨김 속성 개념 없음) | 현재 비Windows는 `attrs = 0`이라 "숨김 표시" 토글이 무동작. macOS에서 플래그를 읽는 표준 라이브러리 경로는 **추정**(`std::os::macos::fs::MetadataExt::st_flags`). Linux에서 "숨김" 토글과 "점 파일" 토글을 합칠지는 설정 인벤토리와 함께 결정 |
| OPS-305 | **복사 시 메타데이터 보존** | 보존 안 함(수동 read/write — `ops:154-187`). 수정 시각 · 읽기 전용 속성 · 대체 스트림 유실(dir2 기존 동작) | 권한(실행 비트) · 수정 시각 최소 보존: `fs::set_permissions(dest, 원본.permissions())` + `File::set_modified`. 확장 속성 · 리소스 포크까지는 `copyfile(3)`(후속) | 권한 · 수정 시각 동일 | Unix에서 실행 비트가 사라지면 복사한 스크립트/바이너리가 실행 불가 — **기능 결함**이므로 최소 권한 보존은 필수로 본다. Windows 동작을 같이 바꿀지(수정 시각 보존)는 dir2 계승 원칙과 충돌하므로 결정 필요 |
| OPS-306 | **폴더 안 심볼릭 링크 복사** | 폴더 링크/정션은 `file_type().is_dir()=false`라 파일로 열려다 실패(추정) | `read_link` → `std::os::unix::fs::symlink`로 **링크 자체를 재생성**(대상 추적 복사 금지 — 순환 · 번들 손상 방지) | 동일 | macOS `.app` 번들 · 프레임워크, `node_modules/.bin` 등 링크가 흔하다. 현 코드는 링크를 일반 파일로 열어 대상 내용을 복사하거나(파일 링크) 오류(폴더 링크 · 끊긴 링크)가 난다(코드 추적 — `ops:207-212`). `size_of`는 링크를 0으로 센다(`ops:131`) — 일관 유지 |
| OPS-307 | **휴지통 삭제 · 복원** | `SHFileOperationW(FO_DELETE + FOF_ALLOWUNDO)`(`win:3180-3200`) · 복원 = 휴지통 셸 폴더 열거 + `undelete` 동사(`nexa-dir2/crates/nexa-app/src/recycle.rs:1-32`) | `NSFileManager trashItemAtURL:resultingItemURL:error:` — 삭제 때 받은 **결과 URL을 기억**해 복원 시 원위치로 이동 | **XDG Trash 규격 직접 구현**: `$XDG_DATA_HOME/Trash/files/` + `info/<이름>.trashinfo`(`[Trash Info]` · `Path=` 퍼센트 인코딩 · `DeletionDate=`) · 다른 볼륨은 `<마운트>/.Trash-<uid>` · 복원 = info를 읽어 `rename` 후 info 삭제 | 엔진 계약은 그대로(`DeleteFn` 주입 · `ReversibleOp` 구현 — `hist:173-174`). **삭제 시점에 복원 토큰을 돌려받는** 형태로 앱 쪽 트레이트를 잡으면 세 OS가 같은 모양이 된다(Windows만 "원래 경로로 찾기"). nexa-ui에는 휴지통 API가 없다(`nexa-fs` · `nexa-sys` 공개 항목 Grep) → 추가 위치 결정 필요 |
| OPS-308 | **생성 시각** | `Metadata::created()` 항상 가능 | 가능(`st_birthtime`) | 커널 4.11+ · glibc 2.28+의 `statx`에서만 · 파일시스템 미지원이면 `Err` | 실패 = 0 → "만든 날짜" 날짜 삽입이 그 항목 무변경(`bulk:2055-2067` · `br:629-631`) — 오류가 아니라 조용한 무동작이므로 UI에서 알릴지 결정 |
| OPS-309 | **시간대 오프셋** | `GetTimeZoneInformation`의 **현재** bias(파일 날짜의 DST가 아니라 지금 기준 — `win:1369-1379`) | `localtime_r(now).tm_gmtoff / 60` | 동일 | 엔진은 분 단위 정수만 받는다(`br:673`). nexa-fs `local_time`이 이미 `localtime_r`를 쓴다(`nexa-ui/crates/nexa-fs/src/lib.rs:7` · `:758`) — 오프셋만 돌려주는 함수는 없으므로 추가 |
| OPS-310 | **파일명 금지 문자** | 엔진 `rename`은 `\` `/`만 거부(`ops:412`) · 일괄 이름 변경은 Windows 집합 `<>:"/\\\|?*` + 끝 `.`/공백(`br:689` · `br:723`) | OS는 `/`(와 Finder의 `:`)만 금지 | OS는 `/`와 NUL만 금지 | 선택지: ⓐ 세 OS 공통 최소 집합으로 통일(nexa-fs `naming::validate` — 예약어 CON 등 · 제어 문자 · 255바이트 포함, `nexa-ui/crates/nexa-fs/src/lib.rs:513-557`) ⓑ OS별 실제 규칙. nexa-fs의 방침은 ⓐ("한 OS에서 만든 파일이 다른 OS에서도 열리게"). 결정 필요 |
| OPS-311 | **가상 최상위 "내 PC"의 내용** | `A:\`~`Z:\` 프로브(`vfs:116-132`) | `/` + `/Volumes/*` | `/` + `/mnt/*` + `/media/$USER/*` + `/run/media/$USER/*` | nexa-fs `drives()`가 세 OS를 이미 구현(`nexa-ui/crates/nexa-fs/src/lib.rs:414-447`). Unix 항목은 `name` = 잎 이름 · `target = Some(전체 경로)`로 넣으면 트리의 기존 `target` 경로 규칙(`tree:243-246`)이 그대로 동작. 시작 경로 폴백(`%USERPROFILE%` → `C:\` — `win:1381-1388`)도 `HOME` → `/`로 |
| OPS-312 | **워커 ↔ UI 통지 · 워커 발 대화상자** | 통지 = `PostMessageW(WM_APP_TRANSFER, gen, 0/1)` · 종결 통지는 유한 재시도(`win:7048-7056`). 충돌 질문은 **워커 스레드가 직접 모달 창을 띄움**(`win:4539-4577`) | winit 창은 이벤트 루프(메인) 스레드에서만 만들 수 있다 → 워커는 질문을 채널로 보내고 **응답 채널에서 대기**, UI 스레드가 대화상자를 띄워 답을 돌려준다 | 동일 | 통지는 `EventLoopProxy`(nexa-sql의 `Wake` 패턴 — `11-dir2-win-b.md` 확인분). `resolve` 콜백 시그니처(`&mut dyn FnMut(&Path) -> Conflict`)는 그대로 두고 **콜백 안에서 블로킹 대기**하면 엔진 수정이 없다. nexa-sql에 같은 구조의 선례가 있다("워커는 답을 기다리며 멈춰 있고" — `nexa-sql/crates/nexa-sql/src/input_win.rs:2`). 대기 중 취소 = 응답으로 `Skip` + `cancel=true`(현행과 동일) |
| OPS-313 | **클라우드 표시 경로 구분자** | `라벨\Docs\a.txt`(`vfs:212-219`) | `/` | `/` | `cloud_display`만 `\` 고정 · `cloud_from_display`는 둘 다 받는다. 경로 바 세그먼트 분해 규칙과 같이 바꿔야 한다(PANEL 문서 접점). 기존 테스트 2개가 `\`를 단언(`vfs:375-378` · `vfs:423-428`) → OS별 기대값으로 |
| OPS-314 | **프리셋 · 세션 파일 위치** | exe 옆 `data\renames\*.cfg`(쓰기 불가 시 `%LOCALAPPDATA%\NexaDir\data`) | 앱 번들 안은 업그레이드 때 통째로 교체됨 → 사용자 설정 폴더 | XDG 설정 폴더 | nexa-sql 기준 = `user_config_dir("nexa-sql")` 또는 환경변수 재지정(`nexa-sql/crates/nsql-settings/src/lib.rs:4` · `:160`) · 판정 헬퍼 `nexa_conf::user_config_dir` · `is_replaced_on_upgrade`(`nexa-ui/crates/nexa-conf/src/lib.rs:314` · `:348`). 사용자 지시("설정 구조는 nexa-sql 차용")와 dir2 포터블 규율이 만나는 지점 — 설정 인벤토리에서 결정하고 프리셋은 그 아래 `renames/` |
| OPS-315 | **잠긴 파일** | 공유 위반(다른 프로세스가 열어 둠)으로 삭제/이동 실패 → 사전 프로브 · 재시도 UI(`win:3767-3800`) | 강제 잠금 없음 — 열린 파일도 삭제/이동 가능 | 동일 | 엔진 영향 없음. Windows 전용 테스트 1개(`ops:1057-1080`)와 사전 프로브 UI가 W |

---

## 5. 상태 · 영속 · 스레딩 · 메시지 흐름

### 5-1. 상태

| 상태 | 소유 | 수명 | 근거 |
|---|---|---|---|
| `OperationHistory`(undo/redo 스택 · 상한 100) | 창 `State.history` | **세션 한정** — 영속 없음 | `win:1069` · `win:1721` · `hist:1-10` |
| 전송 잡 `TransferJob{shared, gen, op, progress, item_count}` | `State.transfer: Option<…>` — **동시 1잡**(진행 중 재요청 = `ops.busy` 표시 후 무시) | 전송 1회 | `win:1224-1234` · `win:4374-4377` |
| 워커 공유 `TransferShared{cancel, done_bytes, total_bytes, outcome, items, in_flight}` | `Arc` — 원자 3개 + `Mutex` 3개. 워커는 `State`에 접근하지 않는다 | 전송 1회 | `win:1197-1222` |
| 세대 카운터 `transfer_gen` | `State` — 통지의 `gen`이 현재 잡과 다르면 무시(낡은 워커) | 창 수명 | `win:4504-4505` · `win:4660-4664` |
| 전역 `EXTRA_ROOTS` · `CLOUD_LISTER` | `nexa-vfs` 프로세스 전역(`RwLock`) | 프로세스 | `vfs:138` · `vfs:275` |
| 트리(노드 arena · 가시 목록 · 선택 · 정렬 사양 · 필터) | 패널/탭별 `Tree` | 탐색마다 재생성 | `tree:167-179` |
| 일괄 이름 변경 창 `BrState` | 모달 수명 | 창 1회 | `bulk:115-147` |

### 5-2. 영속 형식

| 파일 | 형식 | 근거 |
|---|---|---|
| `data\renames\<이름>.cfg` | UTF-8 텍스트. 1행 `# nexa-dir rename preset v2` · 이후 블록당 1줄 `op=<replace\|case\|insert\|number\|date\|move\|ext>\|키=값…`. 키: `scope`(name/nameext/ext/extdot) · `find` · `with` · `case`(0/1) · `regex`(0/1) · `mode`(all/first/last/entire 또는 upper/lower/title/sentence) · `text` · `off` · `dir`(start/end) · `start` · `step` · `pad` · `pre` · `suf` · `kind`(modified/created) · `fmt` · `len` · `dest`(front/end) · `from` · `to`. 값 이스케이프 `\\` `\|` `\n`. 읽기 상한 64블록 · 목록 상한 64개 · 저장 = 임시 파일 후 rename | `br:742-959` · `bulk:1379-1392` · `nexa-dir2/crates/nexa-app/src/config.rs:994-1011` |
| `data\session.cfg` | 탭 경로 · 활성 인덱스 · 펼침 집합 · 잠금 · 고정(스키마는 세션/설정 인벤토리 소관) | `nexa-dir2/docs/20-session-coalescing.md:86-94` |
| 설정 키 중 엔진 관련 | `transfer_close_ms`(0 = 진행 창 미표시 · 제목줄 %만) — 확인. `show_hidden` · `show_dotfiles` · 정렬 사양 · `case_sensitive`는 `Tree`에 전달되는 값(키 이름 · 기본값은 설정 인벤토리 소관 — 여기서는 미확인) | `win:4637-4648` · `win:6406` · `tree:189-193` · `tree:108-114` |

사용 중인 i18n 키(자원 그대로 이관): `ops.*` 19개 · `op.moveCount` · `op.copyCount` · `history.*` 3개 · `rename.done` · `rename.fail` · `new.*` · `bulk.*`(현행 UI가 쓰는 것 + **제거된 UI의 잔존 키** `bulk.kind.move` · `bulk.kind.ext` · `bulk.moveStart` 등 — `nexa-dir2/crates/nexa-app/lang/ko.lang:251-296` · `:444-495`). `ko.lang:118` 부근의 3버튼 문구(`ops.overwrite`의 "아니오=건너뜀 · 취소=전체 중단")는 현재 4버튼 대화상자와 맞지 않는 옛 문구다 — 그대로 옮기되 정리 후보.

### 5-3. 세션 저장 코얼레싱 → nexa-ui 대응

dir2: `Panel.session_dirty` 플래그 → `update_status`에서 수거 → `SetTimer(TIMER_SESSION_SAVE = 8, 1000ms)` 재무장 → 만료 시 1회 저장(`nexa-dir2/docs/20-session-coalescing.md:24-94`). 한계로 "변경이 1초 미만 간격으로 계속되면 저장이 무한히 밀린다 · 최대 지연 상한 하이브리드로 확장 가능"을 적어 두었다(`:107-111`).

dir3: `nexa_conf::SaveScheduler::new(quiet_ms, max_delay_ms)`가 **quiet 1초 OR 첫 미저장 변경 후 max_delay** 발화를 이미 구현한다(`nexa-ui/crates/nexa-conf/src/lib.rs:166-226`) — dir2 문서가 후속으로 남긴 하이브리드가 그대로 들어 있다. `mark(now)` = 플래그 · `tick(now)` = 수거 · `flush_now()` = 종료 저장 · `Store::save`는 직전 저장분과 같으면 쓰지 않는다(`:271-280`). 지켜야 할 dir2 규칙: ① 변경 지점 7곳 마킹(`move_tab` · `toggle_tab_lock` · `toggle_tab_pin` · `close_tab` · `switch_tab` · `new_tab` · `apply_source`) ② 두 패널 플래그는 **둘 다 항상 수거**(단락 평가 금지) ③ 자동 저장과 종료 저장이 같은 스냅샷 함수.

### 5-4. 전송 스레딩 · 메시지 흐름

```
[UI] start_transfer(sources, dest, op)                      win:4364
  ├ 진행 중이면 "ops.busy" 표시 후 반환                       win:4374
  ├ transfer_gen += 1 · TransferShared 생성                  win:4504-4514
  ├ 충돌 문구 · 버튼 라벨 · 글꼴을 UI 스레드에서 선확정         win:4517-4526
  ├ std::thread::spawn ───────────────────────────────┐    win:4527
  │                                                    │
  │   [워커] nexa_ops::transfer(...)                    │    win:4535
  │     resolve  → (충돌 시) 4버튼 모달 → Conflict       │    win:4540-4577
  │     on_event → Plan     : total · items 채움 + 게시  │    win:4586-4597
  │                ItemStart: in_flight = dest · 기준 바이트 기록(게시 없음)
  │                Bytes    : done/total 갱신 + 게시      │    win:4606-4614
  │                ItemEnd  : in_flight = None · 상태 + 게시
  │     outcome 저장 → 종결 통지(유한 재시도 50회 × 100ms) │    win:4633-4635
  │                                                    │
  ├ 진행 창 열기(transfer_close_ms > 0일 때만)            │    win:4637-4648
  └ State.transfer = Some(job)                          │
                                                       ▼
[UI] on_transfer_message(gen, done_phase)                   win:4660
  ├ gen 불일치 = 무시
  ├ 진행: 퍼센트 = done×100/total(total 0 = 100) → 진행 창 갱신 + 제목줄 "전송 n%"
  │       진행 창 [취소]/X → shared.cancel = true            win:4665-4703
  └ 종결: 진행 창 "완료" 표기 + 자동 닫힘 타이머(close_ms + 500)
          undo 기록(OPS-039) → 양쪽 패널 재로드
          제목줄 노트 "전송 a · 건너뜀 b · 실패 c · 취소됨"     win:4704-4778
```

지켜야 할 불변식: ① 워커는 `State`를 건드리지 않는다(공유 구조만) ② 진행 통지는 유실돼도 다음 통지가 덮는다 · **종결 통지만은 반드시 도달**해야 한다(유실 = `State.transfer` 영구 고착 → 이후 전송 전부 차단 — `win:7041-7056`) ③ `in_flight` 경로(와 그 하위)는 전송이 끝날 때까지 실행/드래그 차단(`win:7021-7029`) ④ 공유 `Mutex`는 poison 내성 잠금(`win:7031-7039`) ⑤ `ItemStart`는 게시하지 않는다(작은 파일 다수일 때 메시지 절감).

일괄 이름 변경 · 인라인 이름 바꾸기 · 새로 만들기 · 완전 삭제 · undo/redo는 **UI 스레드에서 동기 실행**한다(`win:6447-6471` · `win:4047` · `win:4084` · `win:3752-3758` · `win:3514-3524`). undo/redo의 교차 볼륨 이동/재복사도 동기라 큰 배치에서 UI가 멈춘다(기존 동작).

---

## 6. 이식 시 주의 — 실측 교훈 · 결함 이력 · 새로 확인한 문제

| ID | 구분 | 내용 | 근거 |
|---|---|---|---|
| OPS-401 | 새로 확인(기존 결함) | **대소문자만 바꾸는 이름 변경이 조용한 무동작**이다. `rename`이 대소문자 무시 동등이면 원래 경로를 돌려준다. 일괄 이름 변경은 대소문자만 바뀐 항목을 충돌 없음으로 통과시키므로(`br:733-736`) 적용 시 "성공"으로 세고 `(old, old)` 쌍을 `MoveBatchOp`에 기록한다 → 실제로는 안 바뀌었는데 완료 건수에 포함되고, undo 시 "대상 존재"로 실패 집계된다(코드 추적). 대소문자 변경 카드(종류 3)의 주 용도가 이 경로다. dir3에서 고칠지(권장: 대소문자만 다르면 존재 검사 없이 `fs::rename`) dir2 동작을 유지할지 결정 필요 | `ops:419-421` · `win:6449-6466` · `hist:143-146` |
| OPS-402 | 결함 이력 계승 | 덮어쓰기는 **스테이징 후 교체**. 종전(선삭제 후 복사)은 취소 시 옛 폴더가 소실됐다. 스테이징 이름 규약 `.<이름>.nexa-<tmp\|old>-<pid>-<seq>` · 같은 부모(같은 볼륨이라 rename이 원자적) | `ops:294-339` · 테스트 `ops:812-893` |
| OPS-403 | 결함 이력 계승 | 폴더 열거 `Err`를 **버리지 말고 전파**하고, 교차 볼륨 이동의 원본 정리는 `remove_dir_all`이 아니라 **복사 기록에 있는 항목만** 삭제한다. 종전은 열거에서 빠진 항목 · 복사 중 생긴 항목이 양쪽 어디에도 남지 않았다(재현 채록) | `ops:189-259` · `ops:341-358` · 테스트 `ops:979-1133` |
| OPS-404 | 새로 확인(위험) | **폴더를 자기 하위 폴더로 "복사"하는 것을 엔진이 막지 않는다**(방어는 이동에만). DnD는 호출부가 막지만(`nexa-dir2/crates/nexa-app/src/dnd.rs:227`) 붙여넣기 경로(`win:2887-2906`)에는 방어가 없다. `copy_dir_with_progress`는 대상을 먼저 만든 뒤 원본을 열거하므로 새로 만든 대상을 다시 만나 경로 길이 한계까지 중첩된다(코드 추적 — Windows 260자보다 Unix 4096바이트에서 훨씬 깊다). `transfer`에서 복사에도 같은 방어를 넣는 것을 권장 | `ops:541-543` · `ops:194-218` |
| OPS-405 | 결함 이력 계승 | 정렬 "종류" 키는 종류 순위만으로는 파일끼리 전부 동률이라 오름/내림이 같아진다 → 확장자 2차 비교. 폴더 크기는 정렬에서 0으로 정규화 | `tree:323-334` |
| OPS-406 | 결함 이력 계승 | ExtDot 스코프는 "점 유실 시 자동 복원"을 폐기하고 **결과 그대로 채택**(앞삽입 시 `.INS.txt` 이중 점 문제) | `br:535-546` · 테스트 `br:1073-1108` |
| OPS-407 | 결함 이력 계승 | `cloud_from_display`의 순회에서 `?`로 조기 반환하면 실경로 링크 항목 하나 때문에 다른 연결의 세그먼트 클릭이 전부 죽는다 → `continue` | `vfs:253-258` |
| OPS-408 | 테스트 교훈 | ① 전역 `EXTRA_ROOTS`를 만지는 테스트는 직렬화 잠금 필요(병렬 실행 시 간헐 실패 실측) ② `Secret` 소거 검사는 **살아 있는 버퍼**에서 — 해제된 메모리 재읽기는 macOS · Linux 할당자에서 실제로 실패했다(Windows만 우연히 통과) ③ 임시 폴더 이름에 pid를 넣어 격리 ④ 타이밍 단언 금지(벤치는 `#[ignore]`) | `vfs:301-309` · `secret:87-92` · `secret:155-158` · `tree:880-884` |
| OPS-409 | 주의 | 엔진 오류 문구가 한국어로 박혀 있다(OPS-020). 영어/일본어 UI에서 이름 바꾸기 실패 시 한국어가 노출된다. `OpError`처럼 구조화해 앱에서 번역하는 편이 맞지만 동작 변경이므로 결정 필요 | `ops:254` · `ops:371` · `ops:413-425` · `win:4062` |
| OPS-410 | 문서 ↔ 코드 차이 | `docs/22`는 설계 시점 문서다. 구현은 다음이 다르다: ① 날짜 토큰 = `yyyy-MM-dd` 문법이 아니라 **`${YYYY}` 문법**(구식은 자동 이행) ② ExtDot 규칙(OPS-406) ③ 위치 입력 = 숫자 EDIT + 라디오가 아니라 **스핀 + 세그먼트** ④ 치환이 **텍스트/정규식 두 카드로 분리** ⑤ **구간 이동 · 확장자 변경 카드는 UI에서 제거**(코어만 유지) ⑥ 미리보기 = ✓ 마커 열이 아니라 **행별 제외 가능한 체크 열 그리드** ⑦ `RenameInput`에 `is_dir` 포함 · 필드명 `modified_ms`/`created_ms`. 구현 기준은 **코드** | `nexa-dir2/docs/22-batch-rename-v2.md:60-105` 대 `br:309-316` · `br:228-234` · `bulk:101-111` · `bulk:1042-1057` · `bulk:450-470` |
| OPS-411 | 문서 ↔ 코드 차이 | `docs/01`의 `nexa-shell` 크레이트는 **존재하지 않는다**(워크스페이스 멤버 7개에 없음 — 셸 기능은 `nexa-app` 안). `nexa-term`도 문서는 "ConPTY 세션 + VT"지만 크레이트 설명은 VT 파서뿐이다 | `nexa-dir2/docs/01-architecture.md:39-40` 대 `nexa-dir2/Cargo.toml:5-13` · `nexa-dir2/crates/nexa-term/Cargo.toml:9` |
| OPS-412 | 주의 | 일괄 이름 변경의 조상 · 자손 동시 선택은 **감지 후 적용 차단**까지만 구현(경로 rebase는 후속으로 남음). 적용 순서는 선택 순서 그대로라 A→B, B→C 같은 연쇄는 배치 내 중복/존재 검사에 걸려 차단된다(임시 이름 2단계 적용 없음) | `br:260-263` · `br:705-719` · `win:6449-6454` |
| OPS-413 | 주의 | `regex-lite`는 `regex` 본가보다 기능이 작다(유니코드 클래스 등 — **추정**). nexa-sql은 `regex` + `fancy-regex`를 쓴다(`nexa-sql/crates/nexa-sql/Cargo.toml:25` · `:40`). 엔진을 바꾸면 사용자가 저장한 정규식 프리셋의 동작이 달라질 수 있으므로 **`regex-lite 0.1` 유지**를 권장 | `nexa-dir2/crates/nexa-ops/Cargo.toml:11-14` |
| OPS-414 | 주의 | 미리보기 정렬은 표시 순서만 바꾸고 연번 순번은 원래 항목 순서를 따른다 → 정렬 후 화면에서 번호가 뒤섞여 보인다(기존 동작). 표시 행 ↔ 항목 인덱스 변환을 빠뜨리면 다른 행이 제외된다 | `bulk:471-497` · `bulk:1943-1957` |
| OPS-415 | 주의 | 그리드에 빈 문자열 셀을 그리려다 크래시한 이력(빈 버퍼 포인터) — 새 그리드도 빈 셀은 그리기를 건너뛰는 분기를 둔다 | `nexa-dir2/crates/nexa-app/src/ctl/grid.rs:26-28` |
| OPS-416 | 주의 | 스테이징 임시 이름은 점으로 시작한다. Unix에서는 점 파일 필터가 꺼져 있으면 안 보이지만 Windows에서는 숨김 속성이 아니라 전송 중 목록에 보일 수 있다(기존 동작). 긴 파일명에 25자 안팎이 더해지므로 255바이트 한계 근처 이름은 덮어쓰기에서 실패할 수 있다(추정) | `ops:294-307` |

---

## 7. 회귀 테스트 후보

자동화 표기: **U** = 순수 단위(파일시스템 없음) · **F** = 임시 폴더 통합(자동) · **X** = OS별 조건부 자동(`cfg`) · **M** = 실기/수동.

| ID | 시나리오 | 대상 | 자동화 | 비고 |
|---|---|---|---|---|
| OPS-501 | 기존 83개 테스트를 dir3에서 세 OS 모두 통과 | OPS-001~131 | U/F/X | 이식 직후 첫 게이트. `cfg(windows)` 3개 · `#[ignore]` 1개 제외 |
| OPS-502 | 같은 폴더 이동 = 무동작 · 복사 = `이름 (2)` | OPS-002 · OPS-014 | F(기존) | |
| OPS-503 | 충돌 2건: 첫째 덮어쓰기 · 둘째 건너뛰기 → 질문 순서 · 결과 내용 | OPS-003 | F(기존) | |
| OPS-504 | 폴더 덮어쓰기 중 취소 → 옛 폴더 내용 보존 · `.nexa-` 잔여 0 | OPS-011 · OPS-402 | F(기존) | 9MB 파일로 2청크 이상 |
| OPS-505 | 복사 중 취소 → 부분 파일 없음 · 원본 무손상 · `canceled = true` | OPS-007 · OPS-009 | F(기존) | |
| OPS-506 | 이벤트 순서 Plan 1회 → ItemStart → Bytes → ItemEnd · 누적 바이트 = 총합 | OPS-006 | F(기존) | |
| OPS-507 | 교차 볼륨 이동: 열거에서 빠진 항목이 원본 또는 사본 한쪽에 반드시 남음 | OPS-012 · OPS-403 | F(기존 · 둘째 볼륨 필요) | CI에 둘째 볼륨이 없으면 조기 반환 — Unix는 tmpfs/램디스크 마운트로 재현(권한 필요 → M 가능성) |
| OPS-508 | **신규** 다른 장치로 이동 시 복사 후 삭제로 폴백 | OPS-301 | X(Linux: `/dev/shm` ↔ 임시 폴더가 다른 장치일 때만 실행) | 판정 함수는 U로 따로(장치 번호 주입) |
| OPS-509 | **신규** 대소문자 구분 FS에서 `Foo` → `foo` 폴더로 이동이 무동작 처리되지 않음 · 비구분 FS에서 `a.txt` → `A.txt` 이름 변경이 실제로 반영됨 | OPS-303 · OPS-401 | X | 임시 폴더에 `A`/`a` 동시 생성 가능 여부로 FS 성질을 먼저 탐지 |
| OPS-510 | **신규** 폴더 심볼릭 링크가 `Dir` + 링크 표식으로 열거되고 펼쳐짐 · 끊긴 링크 = `Symlink` | OPS-302 | X(unix) · Windows는 기존 정션 테스트 | |
| OPS-511 | **신규** 실행 비트(0755) 파일 복사 후 권한 유지 · 수정 시각 유지 | OPS-305 | X(unix) | |
| OPS-512 | **신규** 링크가 든 폴더 복사 → 링크가 링크로 재생성(대상 내용 복제 아님) · 끊긴 링크도 오류 없이 | OPS-306 | X(unix) | |
| OPS-513 | **신규** 폴더를 자기 하위로 복사 → 오류로 격리 · 중첩 폴더 생성 없음 | OPS-404 | F | |
| OPS-514 | **신규** 숨김 필터: Windows 숨김 속성 / macOS `UF_HIDDEN` / 점 파일 각각 | OPS-304 · OPS-110 | X | macOS는 `chflags hidden` 필요 |
| OPS-515 | **신규** 가상 최상위 열거: Windows = 드라이브 1개 이상 / Unix = `/` 포함 · 노드 경로가 실경로 | OPS-311 · OPS-111 | X | |
| OPS-516 | Undo/Redo 왕복: 이동 · 복사(주입 삭제) · 이름 변경 · 새로 만들기 · 실패 시 스택에서 소실 · 상한 초과 시 오래된 것 제거 | OPS-030~035 | F(기존) | |
| OPS-517 | **신규** 휴지통 삭제 → 복원 왕복(원래 위치 · 내용 동일) | OPS-036 · OPS-307 | X + M | Linux는 `XDG_DATA_HOME`을 임시 폴더로 돌려 자동화 가능 · macOS/Windows는 실제 휴지통을 건드리므로 수동 또는 전용 러너 |
| OPS-518 | 일괄 이름 변경 코어 14개(파이프라인 순서 · 스코프 4종 · 모드 4종 · 위치 클램프 · 연번 감싸기 · 날짜 토큰/TZ · 정규식 캡처 · 프리셋 v2 왕복 + v1 호환 · 충돌 5종 · Nested) | OPS-050~065 | U(기존) | 가장 값싼 회귀 방어선 |
| OPS-519 | **신규** 대화상자 로직(화면 분리 후): 폼 유효성(빈 find · Entire 예외 · 빈 text) · [이름 변경] 활성 조건 4개 · 건수 = 체크된 변경 행 · 정렬 후 제외 토글이 올바른 항목에 적용 · 프리셋 메뉴 인덱스 해석(구분선 유무) · 카드 없는 종류 건너뛰고 빈 결과 = 기본 카드 1장 | OPS-072~077 | U | 현재 테스트 0 — 로직을 화면에서 떼어내야 가능 |
| OPS-520 | **신규** 일괄 이름 변경 적용 통합: 일부 실패 시 성공분만 기록 · undo 한 번에 배치 전체 복귀 · 대소문자만 바뀌는 항목 | OPS-082 · OPS-401 | F | `11-dir2-win-b.md` T18과 같은 시나리오 — 중복 구현 금지 |
| OPS-521 | **신규** 전송 통지 계약: 종결 통지 1회 도달 · 세대 불일치 통지 무시 · 취소 요청 후 `Outcome.canceled` · 충돌 응답 왕복(워커 대기 ↔ UI 응답) | OPS-312 · §5-4 | U(채널 모의) | 창 없이 엔진 + 통지 어댑터만 구동 |
| OPS-522 | 프리셋 파일: 저장 → 목록 정렬 → 불러오기 → 삭제 스테이징 확정/취소 · 금지 문자 제거된 이름 | OPS-077~079 · §5-2 | F | 데이터 폴더를 임시 폴더로 주입 가능해야 한다 |
| OPS-523 | 세션 저장 코얼레싱: 연속 변경 N회 = 쓰기 1회 · 최대 지연 도달 시 강제 발화 · 종료 시 `flush_now` | OPS-152 · §5-3 | U | `SaveScheduler`는 시계를 주입받으므로 순수 단위 |
| OPS-524 | 10만 노드 스케일 가드(펼침 · 접힘 · 선택이 완료됨) | OPS-118 | U(기존) | |
| OPS-525 | 화면 배치 대조: 본 창 880×620 · 카드 높이 6종 · 라벨 열 폭 언어별(ko/en/ja) · 하단 버튼 우측 정렬 · 팝업 2종 | §2 | M(스크린샷 대조) | dir2 실기 캡처와 나란히 비교 |

점검 로직 제안(핵심 기능 오류 즉시 확인용): 기동 시 또는 `--selfcheck` 인자로 **임시 폴더 안에서** OPS-502 · 503 · 505 · 516 · 518의 축약판(각 1건)을 돌려 통과/실패를 한 줄로 내는 자가 점검을 둔다 — 전부 `std::fs`와 순수 로직이라 창 없이 1초 안에 끝난다. 휴지통 · 장치 경계는 사용자 환경을 건드리므로 자가 점검에서 제외하고 CI의 X 항목으로 둔다.
