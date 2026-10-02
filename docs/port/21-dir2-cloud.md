# 21 · nexa-dir2 인벤토리 — 클라우드(동기화 폴더 링크 · OAuth 직접 연결 · 클라우드 가상 FS)

> 접두사 **CLOUD-NNN**. 이 문서의 ID는 이후 구현·교차 검증의 체크리스트다.
> 대상 원본: nexa-dir2 0.22.0(Windows 전용). 작성 2026-10-03 · 읽기 전용 조사(소스·git·빌드 무변경).
>
> **줄 표기 약속**(저장소 접두 생략형 — 모두 `nexa-dir2/` 아래):
> - `cloud.rs:N` · `cloudfs.rs:N` · `oauth.rs:N` · `secret.rs:N` · `win.rs:N` · `config.rs:N` · `dialog.rs:N` · `panel.rs:N` = `nexa-dir2/crates/nexa-app/src/<파일>:N`
> - `vfs:N` = `nexa-dir2/crates/nexa-vfs/src/lib.rs:N` · `tree:N` = `nexa-dir2/crates/nexa-tree/src/lib.rs:N`
> - `doc26:N` = `nexa-dir2/docs/26-cloud-integration-study.md:N` · `adr6:N` = `nexa-dir2/docs/27-adr-0006-cloud-oauth.md:N`
> - `ko.lang:N` = `nexa-dir2/crates/nexa-app/lang/ko.lang:N`
> - 그 밖의 저장소는 `nexa-sql/…` · `nexa-ui/…` 전체 경로로 적는다.
>
> **비밀값 취급**: 원본 소스에는 서비스별 기본 `client_id`·`client_secret` 상수가 들어 있다(`oauth.rs:65` · `:90` · `:97` · `:116-117`). 이 문서에는 **값을 옮겨 적지 않고 줄만 가리킨다**(동봉 정책은 CLOUD-094).
> "추정"은 코드로 확인하지 못한 것(외부 API 규약·타 OS 동작 등)이다. "관찰"은 코드에서 직접 확인한 사실이다.

## 0. 범위 — 읽은 파일과 줄 수

| 대상 | 읽은 범위 | 비고 |
|---|---|---|
| `nexa-dir2/crates/nexa-app/src/cloud.rs` | **1~273줄 전부** | 동기화 클라이언트 탐지(X-36) |
| `nexa-dir2/crates/nexa-app/src/oauth.rs` | **1~1299줄 전부** | 서비스 정의 · PKCE · 루프백 · JSON 수제 파서 · WinHTTP |
| `nexa-dir2/crates/nexa-app/src/cloudfs.rs` | **1~1764줄 전부** | 목록 캐시 · 다운로드 · 쓰기 · 계정 간 복사 |
| `nexa-dir2/docs/26-cloud-integration-study.md` | **1~222줄 전부** | 검토서(Phase A/B) |
| `nexa-dir2/docs/27-adr-0006-cloud-oauth.md` | **1~155줄 전부** | ADR-0006 |
| `nexa-dir2/crates/nexa-app/src/secret.rs` | 1~255줄 전부 | 토큰 DPAPI 보관(담당 범위의 "토큰 저장" 실체) |
| `nexa-dir2/crates/nexa-app/src/win.rs` | `cloud\|oauth` Grep 325건 전수 + 98~280 · 384~660 · 860~880 · 1190~1222 · 1950~1966 · 2130~2200 · 2280~2364 · 2815~2830 · 3196~3313 · 3570~3585 · 3712~3735 · 4020~4084 · 4150~4514 · 5020~5040 · 5390~5840 · 7030~7100 · 7670~7705 · 9100~9290 · 9440~9456 | UI측 통합점(메뉴·명령·통지·진행 창). 나머지는 `10~12-dir2-win-*.md` 소관 |
| `nexa-dir2/crates/nexa-app/src/config.rs` | 30~62 · 196~248 · 355~373 · 524~548 · 786~832 · 985~1011 · 1356~1470 | `CloudConn` · `cloudN` 직렬화 · `data_dir` · 원자 저장 |
| `nexa-dir2/crates/nexa-app/src/dialog.rs` | 1~330 · 330~770 | `show_buttons` · `Progress`(배치 수치 확인) |
| `nexa-dir2/crates/nexa-app/src/panel.rs` | 66~80 · 534~546 · 984~1022 · 1368~1420 · 1512~1530 | 클라우드 경로 표시·탐색 분기 |
| `nexa-dir2/crates/nexa-vfs/src/lib.rs` | 20~60 · 100~300 + 테스트 Grep | 센티널 · 추가 루트 · 열거 콜백 |
| `nexa-dir2/crates/nexa-tree/src/lib.rs` | 232~286 | 열거 분기 |
| `nexa-dir2/crates/nexa-app/lang/{ko,en}.lang` | `cloud` 키 전수(ko 42~84) | 문자열 자원 |
| `nexa-dir2/scripts/budget-b3.ps1` | 12~24(Grep) | B3 화이트리스트 |
| `nexa-sql/crates/nsql-vault/src/{lib,devkey,sealed,session}.rs` | lib 1~140 · devkey 1~311 전부 · sealed 1~118 전부 · session 1~60 | 토큰 보관 기준 방식 |
| `nexa-sql/Cargo.toml` · `nexa-sql/crates/nsql-vault/Cargo.toml` · `nexa-sql/crates/nsql-settings/src/lib.rs`(1~76) | 의존 원장 · 설정 레지스트리 형태 | |
| `nexa-sql/crates/nexa-sql/src/{main.rs:1190-1203, clipboard.rs:1-40, about_win.rs:1-25, input_win.rs:1-40, toast.rs:1-25}` + `Wake` Grep | 3-OS 열기·클립보드·창 골격·워커 통지 선례 | |
| `nexa-ui/crates/nexa-ctl/src/**` | `lib.rs` 1~80 · `controls/mod.rs` 모듈 목록 · `pulldown.rs` 18~110 · `ctxmenu.rs` 95~150 · `timeout_button.rs` 1~70 · `progress` Grep | §3 실재 확인 |
| `nexa-ui/crates/{nexa-dlg,nexa-fs,nexa-conf,nexa-sys}` | 공개 API Grep · `nexa-conf/src/lib.rs` 302~334 | 대화상자·설정 폴더 |
| `nexa-license/crates/*/Cargo.toml` | 의존 Grep | `sha2`·`rand_core` 선례 확인 |

## 1. 기능 목록

이식 분류: **N**=플랫폼 중립(거의 그대로) / **A**=nexa-ui 컨트롤·그리기로 교체 / **P**=OS별 구현 분기 필요 / **W**=Windows 전용 유지(타 OS는 대체·비활성). 둘 이상이면 병기.

### 1.1 동기화 폴더 링크(X-36 — 네트워크 0)

| ID | 기능(사용자 관점) | 동작 상세(조건·예외·기본값) | 진입점 | Win32/OS 의존 API | 분류 | 기존 테스트 |
|---|---|---|---|---|---|---|
| CLOUD-001 | 설치된 동기화 클라이언트 폴더 자동 탐지 | `detect()` → `Vec<CloudCandidate{kind,label,path}>`. 순서 OneDrive → Google Drive → Dropbox. 마지막에 `path.is_dir()` 통과분만 남김(제거 잔재 방어). 비Windows는 빈 Vec(cfg 블록 통째 제거 — 타입 명시 필수 E0282 교훈) | `cloud.rs:15-20` · `:34-46` | — | **P** | 없음(통합) |
| CLOUD-002 | OneDrive 개인/비즈니스(다계정) 탐지 | `HKCU\Software\Microsoft\OneDrive\Accounts\*` 하위 키 열거(이름 버퍼 128) → `UserFolder`(없으면 skip). 라벨 = `OneDrive – <DisplayName > UserEmail > 키이름>`. 계정 키에서 하나도 못 얻으면 env `OneDrive` 폴백(라벨 `OneDrive`). 중복 경로 제거. 문자열 값 버퍼 512 UTF-16 | `cloud.rs:52-122` · `:126-148` | `RegOpenKeyExW` · `RegEnumKeyExW` · `RegGetValueW(RRF_RT_REG_SZ)` · `RegCloseKey` | **P** | 없음 |
| CLOUD-003 | Google Drive(DriveFS) 탐지 | `A:`~`Z:` 중 존재하는 드라이브의 **볼륨 라벨이 정확히 `Google Drive`** 인 것. 라벨 `Google Drive (X:)` · 경로 = 드라이브 루트 | `cloud.rs:153-184` | `GetVolumeInformationW` | **P** | 없음 |
| CLOUD-004 | Dropbox 개인/비즈니스 탐지 | `%APPDATA%\Dropbox\info.json` → 없으면 `%LOCALAPPDATA%\…`(먼저 읽힌 **첫 파일만**). `personal` → `Dropbox – Personal`, `business` → `Dropbox – Business`. 같은 경로 중복 제외 | `cloud.rs:189-211` | env 2종 | **P** | 없음 |
| CLOUD-005 | `info.json` 최소 파서 | `"<account>"` 뒤 첫 `"path"` 문자열 추출. 이스케이프 `\\`·`\/`·`\"` 해제, 그 외 `\x`는 x 그대로(`\uXXXX` 미해석). 빈 값·형식 밖 = `None` | `cloud.rs:216-242` | — | N | `cloud.rs:249-264` |
| CLOUD-006 | 종류별 웹 주소 | onedrive → `https://onedrive.live.com/` · googledrive → `https://drive.google.com/` · dropbox → `https://www.dropbox.com/home` · 그 외 `""` | `cloud.rs:24-31` | — | N | `cloud.rs:267-272` |
| CLOUD-007 | 미연결 후보만 추리기 | `cloud_candidates(conns)` = `detect()` 중 기존 연결의 `path`와 같지 않은 것. 호출 시점: 기동 · 연결 변경 · 언어 전환 · 내 PC 우클릭(매번 재탐지 — UI 스레드 동기) | `win.rs:541-550` · `:1558` · `:5578` · `:5812` · `:3212` | (CLOUD-002~004 경유) | N | 없음 |
| CLOUD-008 | 링크 추가 | 명령 `CMD_CLOUD_ADD_BASE(460)+i` — **메뉴 구성 시점 스냅숏**(`st.cloud_cands`) 인덱스로 해석. 연결 수 < 32일 때만. `CloudConn{kind, label(파이프→`/`), path, account:""}` push → CLOUD-016 → 상태 문구 `cloud.linked` | `win.rs:5527-5549` | — | N | 없음 |
| CLOUD-009 | 후보 0개일 때 "링크 추가하기…" | 하위 메뉴 없이 단독 항목 `CMD_CLOUD_ADD_NONE(299)` → 클릭 시 타이틀 꼬리에 `cloud.addNone`(비활성 항목 미지원 α의 대체) | `win.rs:522-527` · `:5429-5434` | — | A | 없음 |
| CLOUD-010 | 내 PC 배경 우클릭 → 클라우드 링크 메뉴 | 활성 패널 루트가 `::PC::`면 셸 배경 메뉴 대신 이 메뉴. 항목: 후보별 `cloud.connectTo({label})`(id 1+i · 활성) → 이미 연결된 항목(체크+회색, id 0) → 둘 다 없으면 `cloud.addNone`(회색). 선택 = CLOUD-008과 같은 경로. 표시 전 후보 재탐지 후 `st.cloud_cands` 동기 | `win.rs:2817-2826` · `:3205-3249` | `CreatePopupMenu` · `AppendMenuW` · `TrackPopupMenuEx(TPM_RETURNCMD)` · `GetCursorPos` | **A** | 없음 |
| CLOUD-011 | 내 PC 뷰의 "클라우드" 행 | `sync_cloud_roots`: 연결 목록 → `nexa_vfs::set_extra_roots([(라벨, 경로)])`. 동기화 폴더 = **실존(`is_dir`)만**, API 연결 = `::CLOUD:<i>::` 센티널. 열거 시 드라이브 목록 **뒤**에 합류(`FileKind::Dir`, size 0, 수정일 없음, `target=Some(경로)`) | `win.rs:557-573` · `:1446` · `vfs:138-162` · `tree:263-271` | — | N | `vfs:367-401`(간접) |
| CLOUD-012 | 연결 ▸ 바로 가기 | `CMD_CLOUD_GOTO_BASE(300)+i`. API = 센티널로 이동. 동기화 폴더 = `is_dir`면 이동, 아니면 `cloud.gone({label})` 안내 | `win.rs:5435-5455` | — | N | 없음 |
| CLOUD-013 | 연결 ▸ 온라인 보기 | `CMD_CLOUD_WEB_BASE(340)+i` → CLOUD-006 주소를 기본 브라우저로. 직전 `AllowSetForegroundWindow(ASFW_ANY)` | `win.rs:5456-5474` · `:7091-7094` | `ShellExecuteW("open")` · `AllowSetForegroundWindow` | **P** | 없음 |
| CLOUD-014 | 연결 ▸ URL 복사 | `CMD_CLOUD_COPYURL_BASE(380)+i` → 클립보드 텍스트 + `cloud.urlCopied`(프라이빗 창에서 다른 계정 접속용) | `win.rs:5475-5488` | `clipboard::write_text` | **P** | 없음 |
| CLOUD-015 | 연결 해제 / 링크 해제 | `CMD_CLOUD_DISC_BASE(420)+i`. 목록에서 제거. **API 연결이면** `secret::clear_from(i)`(i 이상 토큰 파일 전부 삭제). 캐시 `invalidate_all`. CLOUD-016. 문구: API = `cloud.removed`, 링크 = `cloud.unlinked`. 로컬 폴더·동기화 클라이언트는 무접촉. **확인창 없음** | `win.rs:5489-5515` | — | N | 없음 |
| CLOUD-016 | 연결 변경 공용 마감 | `apply_cloud_change`: 설정 저장 → vfs 루트 동기 → 후보 스냅숏 갱신 → 메뉴 전체 재구성 → **활성 탭이 내 PC인 패널** 재로드 → 전체 다시 그리기 | `win.rs:5809-5840` | `InvalidateRect` | A | 없음 |
| CLOUD-017 | Cloud 메뉴 구성 | 최상위 메뉴 4번째(File·Edit·View·**Cloud**·Help). 구조는 §2-1 | `win.rs:472-477` · `:489-538` | — | **A** | `nexa-dir2/crates/nexa-gui/src/widgets/menubar.rs:601-628`(하위 메뉴 키보드) |

### 1.2 연결 모델 · 설정

| ID | 기능 | 동작 상세 | 진입점 | OS 의존 | 분류 | 기존 테스트 |
|---|---|---|---|---|---|---|
| CLOUD-018 | 연결 1건 = `CloudConn{kind,label,path,account}` | 두 방식 공존: `path` 있음 = 동기화 폴더 링크 / `path` 빔 = API 직접 연결. **`is_api()` = `path.is_empty() && !kind.is_empty()`**(account 유무와 무관 — Dropbox 계정 조회 실패 교훈). 상한 32(`CLOUD_MAX`) | `config.rs:35-61` · `win.rs:278` · `:864-872` | — | N | `config.rs:1438-1448` |
| CLOUD-019 | 연결 영속 `cloudN=kind\|라벨\|경로\|account` | 저장: 인덱스 순, 라벨의 파이프 → `/`. 읽기: `splitn(4,'\|')`·trim, **kind·라벨 필수 + (경로 또는 account) 하나 이상**, 32개 초과 무시. N 값 자체는 순서에 쓰지 않음(등장 순 push) | `config.rs:527-536` · `:806-828` | — | N | `config.rs:1362-1381` · `:1438-1470` |
| CLOUD-020 | 서비스별 `client_id`/`client_secret` 재정의 | 키 `cloud_client_id_<kind>` · `cloud_client_secret_<kind>`(≤256바이트, 빈 값 미기록). **설정 창 UI 없음** — `settings.cfg` 직접 편집. 해석 = 설정값(trim) 우선 → `Service::default_*` → 둘 다 비면 안내 모달(CLOUD-023) | `config.rs:202-248` · `:538-548` · `:791-805` · `oauth.rs:137-160` | — | N | `oauth.rs:1284-1298` · `config.rs:1382-1383` · `:1458-1460` |

### 1.3 OAuth 직접 연결(X-37 1차 — 인증)

| ID | 기능 | 동작 상세 | 진입점 | OS 의존 | 분류 | 기존 테스트 |
|---|---|---|---|---|---|---|
| CLOUD-021 | 서비스 정의 3종 | `Service{kind,display,auth_url,token_url,scope,me_url,default_client_id,default_client_secret,redirect_ports,redirect_slash,me_post}`. 값은 §5-3 표. 메뉴 순서 = `SERVICES = [ONEDRIVE, GOOGLEDRIVE, DROPBOX]` | `oauth.rs:23-50` · `:54-126` · `:130-132` | — | N | `oauth.rs:1266-1280` |
| CLOUD-022 | Connect Cloud 시작 | `CMD_CLOUD_OAUTH_BASE(492)+서비스 인덱스` → `start_cloud_oauth`. client_id/secret 해석(CLOUD-020) → 세션 생성(CLOUD-024) → 안내 모달(CLOUD-028) → 워커 대기(CLOUD-029) | `win.rs:5516-5526` · `:5638-5736` | — | A · N | 없음 |
| CLOUD-023 | client_id 미설정 안내 | 모달(제목 `cloud.connect`, 본문 `cloud.err.noClientIdMsg({display},{kind})`, 버튼 `del.close` 1개). 본문에 설정 키·리디렉션 URI 등록값 안내 포함 | `win.rs:5647-5657` · `ko.lang:65` | `dialog::show_buttons` | **A** | 없음 |
| CLOUD-024 | 루프백 리스너 + 리디렉션 URI | 후보 포트: 서비스 `redirect_ports`가 있으면 **그 목록만**(Dropbox 53682/53683/53684 — 전부 점유 시 `Listener` 오류, 임의 폴백 금지), 없으면 `PREFERRED_PORT=42813` → 점유 시 OS 임의 포트. IPv4 `127.0.0.1` 필수 + 같은 포트 IPv6 `::1` 시도(실패 무시). URI 호스트는 **`localhost`** 고정. `redirect_slash`면 `http://localhost:{port}/`, 아니면 슬래시 없음. `state` = 난수 16바이트 base64url | `oauth.rs:344` · `:348-421` | `std::net::TcpListener`(Windows = ws2_32) | N | `oauth.rs:1190-1229` |
| CLOUD-025 | PKCE verifier / challenge | verifier = 난수 64바이트를 66자 문자표(`A-Za-z0-9-._~`)로 모듈로 매핑한 **64자**. challenge = base64url(SHA-256(verifier)), 패딩 없음, `S256` | `oauth.rs:204-209` · `:237-271` · `:279-295` | `BCryptOpenAlgorithmProvider("SHA256")` · `BCryptCreateHash` · `BCryptHashData` · `BCryptFinishHash` | **P** | `oauth.rs:1121-1128` · `:1139-1146` |
| CLOUD-026 | 암호학적 난수 | `BCryptGenRandom(BCRYPT_USE_SYSTEM_PREFERRED_RNG)`. **실패·비Windows 폴백 = 시각·주소 기반 xorshift(테스트 전용 품질)** — 타 OS에서 그대로 쓰면 안 됨 | `oauth.rs:212-234` | `BCryptGenRandom` | **P** | (CLOUD-025 테스트 간접) |
| CLOUD-027 | 인증 URL 조립 | `{auth_url}?client_id&response_type=code&redirect_uri&scope&state&code_challenge&code_challenge_method=S256`(값 전부 퍼센트 인코딩, challenge만 원문). Google `&access_type=offline&prompt=consent`, Dropbox `&token_access_type=offline` 추가 | `oauth.rs:312-331` | — | N | `oauth.rs:1232-1242` |
| CLOUD-028 | 인증 안내 모달(3버튼) | 제목 `cloud.connect` · 본문 `cloud.auth.prompt({display},{url})`(URL 전문 표시). 버튼 id 1 `cloud.auth.openBrowser`(기본) / 2 `cloud.copyUrl` / 3 `ops.cancel`. 결과 3 또는 0(닫힘) = 취소(세션 drop으로 리스너 닫힘). 2 = URL을 클립보드로(브라우저 미실행), 1 = 기본 브라우저 실행. 1·2 모두 이후 워커 대기 + 타이틀 `cloud.auth.waiting` | `win.rs:5665-5700` · `ko.lang:60-62` | `show_buttons` · `ShellExecuteW` · 클립보드 | **A · P** | 없음 |
| CLOUD-029 | 리디렉션 1회 수신 | 워커에서 `wait_and_exchange(300초)`. 리스너 논블로킹, IPv4·IPv6 번갈아 `accept` 폴링(120ms 간격). **첫 접속 1건만** 처리: 4096바이트 1회 read(읽기 타임아웃 5초) → 요청 줄의 쿼리에서 `code`·`state`·`error` 추출. 성공 조건 = `code` 있음 + `state` 일치. 브라우저에 200 HTML(성공/실패 2개 국어 문구) 응답 후 `Connection: close`. `error` 있으면 `Denied(error)`, 불일치 `Denied("state mismatch")`, 시간 초과 `Timeout` | `oauth.rs:425-426` · `:479-536` · `win.rs:5701-5735` | std 소켓 | N | `oauth.rs:1179-1184` |
| CLOUD-030 | 토큰 교환 | `POST token_url` form: `client_id&code&redirect_uri&grant_type=authorization_code&code_verifier`(+`client_secret`은 비어 있지 않을 때만). 응답 `access_token`·`refresh_token`·`expires_in`. access 비면 `Exchange(error_description > error > "no access_token")` | `oauth.rs:427-453` | HTTP(CLOUD-039) | N(HTTP는 P) | 없음 |
| CLOUD-031 | 계정 표시명 조회 | `me_url`이 있으면 `me_post ? 본문·Content-Type 없는 POST : GET`(Bearer). 키 우선순위 `userPrincipalName > mail > emailAddress > email > displayName > name`. 실패·빈 값 = 빈 문자열(연결은 진행) | `oauth.rs:454-475` | HTTP | N | 없음 |
| CLOUD-032 | refresh → access 재발급 | `POST token_url` form `client_id&refresh_token&grant_type=refresh_token`(+secret). 응답에 새 refresh 없으면 기존 유지. **목록 1회·전송 잡 1회마다 매번 호출**(access 캐시 없음) | `oauth.rs:541-571` · `cloudfs.rs:177-183` · `:450-455` | HTTP | N | 없음 |
| CLOUD-033 | 인증 완료 처리 | 실패: 타이틀 + **모달**(`{display} — {오류문구}` + 빈 줄 + 상세). 성공: 32개 초과 = `cloud.err.full`. 라벨 = `{display} – {account 또는 cloud.account.unknown}`(파이프→`/`). **같은 kind의 API 연결 중 account 같거나 빈 것**이 있으면 그 행 갱신(재인증·유령 행 흡수), 없으면 push(`path=""`). 토큰 저장 실패 = `cloud.err.tokenSave` 안내(연결은 유지). CLOUD-016 → `cloud.connected` | `win.rs:5623-5630` · `:5739-5805` · `:9280-9288` | — | A · N | 없음 |
| CLOUD-034 | 인증 오류 → i18n 키 | `NoClientId`→`cloud.err.noClientId` · `Listener`→`.listener` · `Timeout`→`.timeout` · `Denied`→`.denied` · `Exchange`→`.exchange` | `oauth.rs:177-201` | — | N | 없음 |

### 1.4 토큰 보관

| ID | 기능 | 동작 상세 | 진입점 | OS 의존 | 분류 | 기존 테스트 |
|---|---|---|---|---|---|---|
| CLOUD-035 | refresh 토큰 저장/로드/삭제 | 파일 `data\secrets\cloud<N>.tok`(N = **연결 인덱스**). 내용 = DPAPI 블롭의 **소문자 hex 1줄**. 저장은 `config::save`(임시 파일+`sync_all`+rename — 원자적). 로드 실패(부재·hex 손상·복호 실패·비UTF-8) = `None` | `secret.rs:14-63` · `config.rs:994-1011` | `CryptProtectData`/`CryptUnprotectData`(엔트로피·플래그 없음) · `LocalFree` | **P** | `secret.rs:174-200`(Windows 전용) · `:220-224` |
| CLOUD-036 | 꼬리 토큰 정리 | `clear_from(i)` = 슬롯 `i..32` 토큰 파일 전부 삭제(부재 무시) | `secret.rs:39-41` · `:66-71` | — | N | `secret.rs:205-216`(Windows 전용) |
| CLOUD-037 | 손상 토큰 파일 방어 | hex 디코드: 앞뒤 공백 허용 · 빈 문자열/비ASCII(BOM·멀티바이트)/홀수 길이/비hex = `None`(문자 경계 panic 방지 G10-04) | `secret.rs:76-85` | — | N | `secret.rs:228-254` |
| CLOUD-038 | 포터블 안전 특성 | DPAPI = 사용자+PC 바인딩 → `data\`를 타 PC로 옮기면 복호 실패 = 재로그인(**의도된 동작**). 토큰 없으면 목록은 빈 목록, 전송은 `cloud.err.noToken` | `secret.rs:1-9` · `adr6:50-55` · `win.rs:637-640` · `:4225-4228` | DPAPI | **P** | 없음 |

### 1.5 HTTP 스택

| ID | 기능 | 동작 상세 | 진입점 | OS 의존 | 분류 | 기존 테스트 |
|---|---|---|---|---|---|---|
| CLOUD-039 | 단일 요청 코어 `winhttp_request` | 인자: url · method · body · content_type · extra_headers(CRLF 종단 문자열) · bearer · limit · on_progress(누적, 전체[0=미상])→false면 취소 · out_location. 동작: URL 분해(호스트 256/경로 2048 UTF-16) → 세션(UA `NexaDir`, **자동 프록시**) → **항상 HTTPS**(`WINHTTP_FLAG_SECURE`) → 헤더 = `Content-Type` + `Authorization: Bearer` + extra → 본문 일괄 전송 → 상태 코드 · `Content-Length`(u32) · (요청 시) `Location` 조회 → 가용 데이터 반복 read, 누적이 `limit` 초과면 `"response too large"`, 콜백 false면 `CANCELLED` → **비2xx는 `HTTP {status}: {message > error_description > 본문 앞 200자}`** 오류. 타임아웃·리디렉션은 WinHTTP 기본값(명시 설정 없음). 핸들 RAII | `oauth.rs:922-1114` | `WinHttpCrackUrl` · `WinHttpOpen` · `WinHttpConnect` · `WinHttpOpenRequest` · `WinHttpSendRequest` · `WinHttpReceiveResponse` · `WinHttpQueryHeaders` · `WinHttpQueryDataAvailable` · `WinHttpReadData` · `WinHttpCloseHandle` (TLS = schannel) | **P** | 없음 |
| CLOUD-040 | 용도별 래퍼 8종 | `http_post_form`(form POST→문자열) · `http_get`(Bearer GET) · `http_post_bearer`(본문·Content-Type **없는** POST) · `http_get_bytes`(bearer 선택, 미사용 예비) · `http_get_bytes_progress`(진행·취소) · `http_send`(임의 메서드+본문+추가 헤더→문자열) · `http_start_session`(응답 `Location` 반환, 없으면 오류) · `http_send_bytes`(임의 메서드→바이트). 비Windows는 전부 `Err("windows only")` | `oauth.rs:738-898` · `:907-918` | (CLOUD-039) | **P** | 없음 |
| CLOUD-041 | 상한·취소 상수 | 텍스트 응답 상한 `TEXT_LIMIT = 8MiB`. 취소 센티널 문자열 `CANCELLED = "__nexa_cancelled__"`(호출자가 실패와 구분) | `oauth.rs:901-905` | — | N | 없음 |

### 1.6 수제 JSON · 인코딩 유틸(crate 0 — DR-8)

| ID | 기능 | 동작 상세 | 진입점 | 분류 | 기존 테스트 |
|---|---|---|---|---|---|
| CLOUD-042 | `json_str(json,key)` | **문서 전체에서 처음 나오는** `"key"` 뒤 값. 문자열: `\n \t \r \b \f` · `\uXXXX`(서러게이트 쌍 결합, 짝 없으면 그대로/U+FFFD) · 그 외 `\x`→x. 숫자: 숫자·`.`·`-` 연속. 중첩 구조 인식 없음 | `oauth.rs:621-684` | N | `oauth.rs:1149-1176` |
| CLOUD-043 | `json_objects(json,key)` | `"key"` 뒤 첫 `[`의 **원소 객체를 문자열로 분리**(중첩 중괄호·문자열 안 괄호 존중). 배열 부재 = 빈 Vec | `oauth.rs:689-727` | N | `oauth.rs:1246-1263` |
| CLOUD-044 | `json_has(json,key)` | `"key"` 부분 문자열 존재 여부(Graph `folder` 판별) | `oauth.rs:730-732` | N | `oauth.rs:1254`·`:1260` |
| CLOUD-045 | 퍼센트 인코딩/디코딩 · 쿼리 파라미터 | `percent`: unreserved(`A-Za-z0-9-._~`) 외 전부 `%XX`(대문자). `percent_decode`: `%XX` + `+`→공백, 잘못된 hex는 원문 유지, lossy UTF-8. `query_param`: `?` 뒤 `&` 분해 | `oauth.rs:298-309` · `:574-616` | N | `oauth.rs:1131-1136` · `:1179-1184` |
| CLOUD-046 | base64url(패딩 없음) | 3바이트 청크 수제 인코더 | `oauth.rs:279-295` | N | `oauth.rs:1121-1128` |
| CLOUD-047 | JSON 문자열 이스케이프 2종 | `json_escape`(본문용: `"` `\` `\n` `\r` `\t` + 0x20 미만 `\u00XX`) · `json_escape_hdr`(**헤더용**: 추가로 비ASCII 전부 `\uXXXX`, BMP 밖은 서러게이트 2개) | `cloudfs.rs:1455-1490` | N | `cloudfs.rs:1727-1743` |
| CLOUD-048 | RFC3339 → 시각 | `YYYY-MM-DDThh:mm:ss` 고정 자리 파싱(길이 19 미만 = None), 율리우스일 공식, **시간대 오프셋 무시(UTC 가정)**, 음수 = None | `cloudfs.rs:1633-1651` | N | `cloudfs.rs:1658-1666` |
| CLOUD-049 | Graph 경로 인코딩 | `/a/b` → 선행 `/` 제거 후 세그먼트별 퍼센트 인코딩해 `/`로 재결합 | `cloudfs.rs:830-837` | N | 없음 |

### 1.7 가상 FS 연결(센티널 경로 · 열거 · 캐시)

| ID | 기능 | 동작 상세 | 진입점 | OS 의존 | 분류 | 기존 테스트 |
|---|---|---|---|---|---|---|
| CLOUD-050 | 센티널 경로 규약 | `::CLOUD:<연결 인덱스>::<내부 경로>`. 내부 경로 = `""`(루트) 또는 `/`로 시작. `cloud_parts`→`(idx, inner)`, `cloud_root(i)`, `cloud_child(i,parent,name)` = `…::{parent}/{name}` | `vfs:172-195` | — | N | `vfs:349-363` |
| CLOUD-051 | 사람이 읽는 표기 ↔ 센티널 | `cloud_label`(추가 루트에서 라벨 조회) · `cloud_display`(`라벨` + 내부 경로의 `/`→**`\`**, 미등록 = `Cloud {idx}`) · `cloud_leaf`(탭 제목: 마지막 세그먼트 / 루트면 라벨) · `cloud_parent`(루트의 부모 = `::PC::`) · `cloud_from_display`(경로 바 조각 클릭 역변환 — **라벨 긴 것부터** 대조, 링크(X-36) 항목은 건너뜀, `\`·`/` 모두 허용) | `vfs:197-270` | 경로 구분자 표기 | **P**(표기) | `vfs:367-439` |
| CLOUD-052 | 열거 콜백 주입 | `set_cloud_lister(Box<dyn Fn(idx,&inner)->Option<Vec<Entry>>>)` 전역 1개. `cloud_entries(path)`: 클라우드 경로가 아니면 None, 콜백 미등록/None = 빈 목록. 트리 열거 분기 1곳 | `vfs:272-295` · `tree:272-277` | — | N | `vfs:445-448` |
| CLOUD-053 | 앱측 열거 구현 | `install_cloud_lister`(창 생성 시 1회): 캐시 히트 → 즉시 반환. 미스 → 연결 정보 스냅숏(`ConnInfo`) 만들어 워커 요청 + **플레이스홀더 1행**(`cloud.loading`, File, target = 안내 행 경로) 반환. API 연결 아님/정보 없음 = 빈 목록, **토큰 없음 = 빈 목록(요청 안 함)** | `win.rs:611-656` · `:7676-7679` | `state_of(hwnd)` | A(통지 경로) · N | 없음 |
| CLOUD-054 | 목록·ID 캐시 · 진행 중 집합 | 전역 3개: `CACHE[(idx,inner)]→Vec<Entry>` · `IDS[(idx,항목 inner)]→서비스 ID` · `INFLIGHT`. `invalidate(idx)`(그 연결의 목록+ID) · `invalidate_all` · `is_busy(idx)` · `claim/release`(중복 워커 방지). **메모리 전용**(재시작 시 비어 있음) · 만료·용량 상한 없음 | `cloudfs.rs:24-104` | `win::plock`(poison 내성 lock) | N | `cloudfs.rs:1746-1763` |
| CLOUD-055 | 비동기 목록 요청 | `request`: 캐시 있음/진행 중이면 무동작. 워커: refresh → 목록 API → 파싱 → 캐시. **실패도 캐시에 담는다** — `[!] {사유}` 1행(File, target = 안내 행 경로) → 재요청 폭주·영구 로딩 방지(F5 = 재시도). 완료 통지 `ListResult{idx,inner,err}` | `cloudfs.rs:114-163` · `:177-188` | `PostMessageW`(재시도 게시) | A(통지) · N | 없음 |
| CLOUD-056 | 안내 행 전용 경로 | `notice_row_path` = `cloud_child(idx, inner, "\u{1}notice")` — 부모 폴더와 같은 경로가 되지 않게(선택·펼침 복원 엉킴 방지) | `cloudfs.rs:106-111` | — | N | 없음 |
| CLOUD-057 | 목록 도착 처리 | 두 패널의 **클라우드·내 PC 탭 전부**(비활성 포함) 재열기(캐시 기반이라 네트워크 재호출 없음). 오류면 타이틀 꼬리 `{오류} ({inner 또는 "/"})` | `win.rs:9107-9133` · `panel.rs:1376-1392` | `WM_APP_CLOUD_LIST` | A(통지) · N | 없음 |
| CLOUD-058 | 기동 시 세션 복원 탭 깨우기 | 콜백 등록이 패널 복원 **뒤**라 클라우드 탭이 비어 있음 → 등록 직후 두 패널 `reopen_cloud_tabs` 1회 | `win.rs:7676-7702` | — | N | 없음 |
| CLOUD-059 | F5 = 강제 재조회 | 활성 패널 루트가 내 PC면 `invalidate_all`, 클라우드 경로면 `invalidate(idx)` 후 재로드 | `win.rs:5396-5405` | — | N | 없음 |
| CLOUD-060 | "불러오는 중…" 패널 배지 | `sync_cloud_badges`: 패널 루트가 내 PC면 아무 연결이나 진행 중일 때, 클라우드 경로면 그 연결이 진행 중일 때 배지 문구 `cloud.loading`(패널 우하단 플로팅 배지 `set_busy`). 경로 변경·재로드 길목에서 호출 | `win.rs:580-605` · `:5033-5038` · `panel.rs:495-520` | — | A | 없음 |
| CLOUD-061 | 탐색 UI 통합 | 상위 이동(`cloud_parent`, 떠난 폴더 선택) · 홈(내 PC로, 연결 루트 행 선택) · 탭 제목(`cloud_leaf`) · 경로 바(`cloud_display`) · 창 타이틀 · 정보 패널(경로 = 표기 경로, 로컬 메타 조회 생략) · 경로 바 조각 클릭(`cloud_from_display` 선행) — **센티널을 화면에 노출하지 않는다** | `panel.rs:74-76` · `:541-546` · `:992-996` · `:1018-1021` · `:1522-1525` · `win.rs:1959-1960` · `:2287-2299` · `:2360-2363` | — | N | (vfs 테스트) |
| CLOUD-062 | 감시·프로브 제외 | 클라우드 센티널·내 PC는 셸 변경 구독 해제, 폴더 프로브 미무장, 폴더 소실 시 조상 순회 비대상 | `win.rs:3577-3585` · `:5024-5031` · `panel.rs:1381` · `:1409` · `nexa-dir2/crates/nexa-app/src/fsprobe.rs:62-64` | — | N | `nexa-dir2/crates/nexa-app/src/fsprobe.rs:245` |

### 1.8 목록 조회(서비스별)

| ID | 기능 | 동작 상세 | 진입점 | 분류 | 기존 테스트 |
|---|---|---|---|---|---|
| CLOUD-063 | OneDrive 목록 | 루트 `GET graph…/v1.0/me/drive/root/children`, 하위 `…/root:/{enc}:/children`. 쿼리 `$select=id,name,size,folder,file,lastModifiedDateTime&$top=500`. **다음 페이지(`@odata.nextLink`) 미처리**(관찰) | `cloudfs.rs:193-213` | N | `cloudfs.rs:1670-1691` |
| CLOUD-064 | Google Drive 목록 + 폴더 ID 해석 | `GET …/drive/v3/files?q='{부모ID}'+in+parents+and+trashed%3Dfalse&fields=files(id,name,size,mimeType,modifiedTime)&pageSize=500`. 부모 ID: 루트 = `root`, 하위 = ID 캐시 → **미스면 루트부터 한 단계씩 목록 조회해 되짚음**(재시작 직후 하위 경로 복원 대비). 못 찾으면 `folder not found: {경로}`. `nextPageToken` 미처리(관찰) | `cloudfs.rs:214-227` · `:249-277` | N | `cloudfs.rs:1695-1705` |
| CLOUD-065 | Dropbox 목록 | `POST api.dropboxapi.com/2/files/list_folder` JSON `{"path":"{inner}","limit":500}`(루트 = `""`). `has_more`/cursor 미처리(관찰) | `cloudfs.rs:228-242` | N | `cloudfs.rs:1709-1722` |
| CLOUD-066 | 응답 → `Entry` | 배열 키 `value`/`files`/`entries`. 폴더 판별: Graph = `folder` 키 존재 · Google = `mimeType == application/vnd.google-apps.folder` · Dropbox = `.tag == folder`. 크기 = `size`(문자열·숫자 모두, 실패 0). 수정일 키 `lastModifiedDateTime`/`modifiedTime`/`server_modified`. `id`가 있으면 ID 캐시에 `{inner}/{name}`로 적재. `attrs=0`, `target` = 센티널 자식 경로 | `cloudfs.rs:280-324` | N | `cloudfs.rs:1670-1722` |

### 1.9 다운로드(X-37 3차·5차)

| ID | 기능 | 동작 상세 | 진입점 | OS 의존 | 분류 | 기존 테스트 |
|---|---|---|---|---|---|---|
| CLOUD-067 | 다운로드 시작(UI측) | `start_cloud_download(sources, dest_dir)`. 원본 중 **첫 연결의 항목만**(혼합은 무시). 폴더 여부·크기는 **열린 트리 노드에서** 읽음(`cloud_row_info` — 추가 요청 없음). 연결 정보 없음 = `cloud.err.noToken`. 타이틀 `cloud.downloading({n})`. 진행 창은 **대상 폴더 지정 + `transfer_close_ms > 0`** 일 때만(임시 폴더로 받는 더블클릭 열기는 창 없음) | `win.rs:4161-4252` · `:4314-4322` | — | A · N | 없음 |
| CLOUD-068 | 다운로드 워커 | refresh 1회 → 폴더는 로컬 폴더 생성 후 재귀 전개해 파일 목록으로 평탄화 → 세그먼트·총 바이트 **선확정**(크기 0은 1로) → 항목 순차 다운로드. 항목 시작 전 취소 검사. **첫 실패에서 중단**(성공분은 `done`에 남음). 완료 통지 `DownloadResult{done,err,open_after}` | `cloudfs.rs:333-354` · `:357-447` · `:1539-1564` | `PostMessageW` | A(통지) · N | 없음 |
| CLOUD-069 | 항목 1개 다운로드 | OneDrive: `GET …/root:/{enc}?$select=@microsoft.graph.downloadUrl` → 그 URL을 **인증 헤더 없이** GET(진행·취소). Google: `GET …/files/{id}?alt=media`(Bearer, 진행·취소; ID 없으면 "file id unknown — 목록을 새로 고치세요"). Dropbox: `POST content.dropboxapi.com/2/files/download` + 헤더 `Dropbox-API-Arg: {"path":…}`(헤더용 이스케이프), **진행 통지·취소 없음**(관찰). 상한 `DOWNLOAD_LIMIT = 512MiB`(전량 메모리 적재). 기록: 부모 폴더 생성 → `<이름>.nexadl.part`(확장자 치환) 쓰기 → rename(실패 시 임시 삭제). `half=true`면 진행을 절반으로 축척(계정 간 복사 전반부) | `cloudfs.rs:329` · `:459-547` | — | N(HTTP는 P) | 없음 |
| CLOUD-070 | 클라우드 파일 열기(더블클릭·Enter) | 임시 폴더 `%TEMP%\NexaDir\cloud\<이름>`으로 받은 뒤 연결 프로그램으로 실행(진행 창 없음) | `win.rs:4182-4184` · `:7059-7067` · `:9218-9230` | `ShellExecuteW("open")` · `AllowSetForegroundWindow` | **P** | 없음 |
| CLOUD-071 | 다운로드 완료 처리 | 열기 요청이면 성공 파일 각각 실행, 아니면 양쪽 패널 재로드. 문구: 취소 = `ops.canceled` / 오류 = 사유 원문 / 성공 = `cloud.downloaded({n})`. 진행 창 마감(CLOUD-091) | `win.rs:9214-9247` | — | A · N | 없음 |

### 1.10 쓰기(X-37 4~6차)

| ID | 기능 | 동작 상세 | 진입점 | 분류 | 기존 테스트 |
|---|---|---|---|---|---|
| CLOUD-072 | 쓰기 작업 종류 | `WriteOp::{Upload{src,dest_inner}, Delete{inner}, Rename{inner,new_name}, NewFolder{parent_inner,name}, UploadTree{src,dest_inner}, CopyWithin{inner,dest_parent_inner}, MoveWithin{…}}` | `cloudfs.rs:556-573` | N | 없음 |
| CLOUD-073 | 쓰기 시작(UI측) | `start_cloud_write(idx, ops, busy_key)`. 빈 ops = false. 연결 정보 없음 = `cloud.err.noToken`. 타이틀 `{busy_key}({n})`. 진행 창은 **Upload/UploadTree가 하나라도 있을 때만**(+`transfer_close_ms > 0`) — 메타 작업은 바이트가 없어 창을 띄우지 않음 | `win.rs:4326-4362` | A · N | 없음 |
| CLOUD-074 | 쓰기 워커 | 세그먼트 = 작업별 실제 바이트(업로드 = 파일 크기 / 폴더 = `nexa_ops::size_of` / 메타 = 0→1). refresh 1회 → 작업 순차, 작업 전 취소 검사, 청크 콜백으로 누적 갱신, **첫 실패에서 중단**. 끝나면 `invalidate(idx)`. 완료 통지 `WriteResult{idx,done,err}` | `cloudfs.rs:576-582` · `:750-827` · `:966-972` | A(통지) · N | 없음 |
| CLOUD-075 | OneDrive 업로드 | ≤ `SIMPLE_PUT_MAX = 4MiB`: `PUT …/root:/{enc}:/content`(전량 메모리). 초과: `POST …:/createUploadSession`(`conflictBehavior=replace`) → `uploadUrl`에 **인증 헤더 없이** `PUT` + `Content-Range: bytes a-b/total`, 청크 `CHUNK = 320KiB×16 = 5MiB`(320KiB 배수 규약). 청크마다 진행·취소 | `cloudfs.rs:552-554` · `:1568-1629` | N | 없음 |
| CLOUD-076 | OneDrive 메타 작업 | 삭제 `DELETE …/root:/{enc}` · 이름 변경 `PATCH {"name":…}` · 새 폴더 `POST …/children {"name","folder":{},"conflictBehavior":"rename"}` · 같은 연결 복사 `POST …:/copy {"parentReference":{"path":"/drive/root:{dest}"},"name"}`(**비동기 202 — 수락까지만 확인**) · 이동 `PATCH {"parentReference":{…}}` | `cloudfs.rs:995-1100` | N | 없음 |
| CLOUD-077 | OneDrive 폴더 재귀 업로드 | 대상 폴더 `POST`(`conflictBehavior=fail` — 이미 있으면 오류 무시 = 멱등. **`replace` 금지** — 기존 폴더 통째 교체 위험) → 하위 폴더 재귀·파일 업로드. 심볼릭 링크 등은 건너뜀. 진행 콜백 없음(항목 단위) | `cloudfs.rs:1494-1535` | N | 없음 |
| CLOUD-078 | Dropbox 쓰기 전종 | 전부 `POST`+JSON(경로 기반). 업로드: ≤ `140MiB` 단순 `files/upload`(`mode:overwrite`, 인자는 헤더), 초과 = `upload_session/start`(`close:false`) → `append_v2`(8MiB 청크, cursor offset) → `finish`(**빈 본문 + `application/octet-stream`**, commit `mode:overwrite`). 폴더 업로드: `create_folder_v2`(기존 무시) 후 재귀. 삭제 `delete_v2` · 이름 변경 `move_v2` · 새 폴더 `create_folder_v2`(`autorename:true`) · 복사/이동 `copy_v2`/`move_v2`(`autorename:true`) | `cloudfs.rs:1105-1262` | N | `cloudfs.rs:1727-1735` |
| CLOUD-079 | Google Drive 쓰기 전종 | ID 기반. 업로드: ≤ 4MiB `multipart/related`(경계 `nexadirBOUNDARY7f3a`, 메타 `{"name","parents":[id]}`), 초과 = **재개 업로드**(세션 `Location` → 5MiB 청크 `PUT` + `Content-Range`, **중간 청크 오류는 무시**·마지막만 판정 — 308 때문). 새 폴더: `POST files`(mimeType 폴더) → 응답 `id`를 ID 캐시에 심음. 삭제 `DELETE files/{id}` · 이름 변경 `PATCH {"name"}` · 복사 `POST files/{id}/copy {"parents"}` · 이동 `PATCH ?addParents&removeParents`(본문 `{}`). 폴더 업로드 = 폴더 생성 + 재귀. 항목 ID가 캐시에 없으면 "item id unknown … 목록을 새로 고치세요" | `cloudfs.rs:842-895` · `:1265-1452` | N | 없음 |
| CLOUD-080 | 대상 폴더 보장(멱등) | `ensure_folder`: Google = ID 해석되면 통과, 아니면 생성 / OneDrive = `conflictBehavior=fail` POST(결과 무시) / 그 외 = 새 폴더 시도(오류 무시) | `cloudfs.rs:901-962` | N | 없음 |
| CLOUD-081 | 계정 간(다른 연결) 복사 | 서버 사이드 불가 → `%TEMP%\NexaDir\xcopy` 경유. 원본 폴더는 재귀 전개(상대 경로 유지) → **대상 폴더를 깊이 오름차순으로 먼저 생성** → 파일마다 다운로드(세그먼트 앞 절반) → 업로드(뒤 절반) → 임시 파일 삭제(성패 무관). 끝나면 스테이징 폴더 삭제 + 대상 캐시 무효화. 통지는 쓰기 완료(`WriteResult`)로 합류. 진행 창 **항상**(`transfer_close_ms > 0`) | `cloudfs.rs:584-747` · `win.rs:4391-4448` | N | 없음 |
| CLOUD-082 | 전송 분기(붙여넣기·DnD 공용) | `start_transfer`: ① 대상이 클라우드 + 원본 클라우드·같은 연결 = `CopyWithin`/`MoveWithin`(서버 사이드) ② 다른 연결 = CLOUD-081(**이동이어도 복사**) ③ 원본 로컬 = `Upload`/`UploadTree`(이름 = 원본 파일명, **이동이어도 원본 미삭제**), ops 0건 = `cloud.err.filesOnly` ④ 원본 클라우드 → 로컬 = 다운로드(**복사·이동 무관**). 타이틀 `cloud.copying`/`cloud.uploading` | `win.rs:4364-4503` | N | 없음 |
| CLOUD-083 | 클라우드 삭제(Del·Shift+Del) | 첫 대상이 클라우드면 그 연결의 대상만 `Delete` 묶음 → `cloud.deleting`. **확인창·undo 없음, 영구/휴지통 구분 없음** | `win.rs:3716-3731` | N | 없음 |
| CLOUD-084 | 클라우드 이름 변경(인라인) | `Rename` 1건 → `cloud.renaming`. undo 기록 없음 | `win.rs:4027-4035` | N | 없음 |
| CLOUD-085 | 클라우드 새 폴더 / 새 파일 | 새 폴더 = `NewFolder{name: tr("new.folderBase")}` → `cloud.creating`(생성 후 인라인 이름변경 진입 없음). **새 파일은 미지원** → `cloud.err.foldersOnly` | `win.rs:4068-4082` | N | 없음 |
| CLOUD-086 | 가상 파일 붙여넣기 → 클라우드 | 대상이 클라우드면 `%TEMP%\NexaDir\vpaste-<pid>-<seq>`에 동기 추출 후 업로드 경로(CLOUD-082 ③)로 연계. **가상 파일 드롭**의 클라우드 대상은 무동작 | `win.rs:2141-2155` · `:2186-2194` · `:3300-3302` | P(가상 파일 추출은 타 인벤토리) | 없음 |
| CLOUD-087 | 쓰기 완료 처리 | 두 패널의 클라우드·내 PC 탭 전부 재열기. 문구: 성공 `cloud.writeDone({done})` / 취소 `ops.canceled` / 오류 문자열에 `403` 또는 `401` 포함 = `cloud.err.reauth`(scope 상향 전 토큰) / 그 외 사유 원문 | `win.rs:9250-9278` | A · N | 없음 |

### 1.11 진행·취소·통지

| ID | 기능 | 동작 상세 | 진입점 | OS 의존 | 분류 | 기존 테스트 |
|---|---|---|---|---|---|---|
| CLOUD-088 | 진행 창 재사용 · 동시 1개 | 로컬 복사와 **같은 진행 창**(`dialog::Progress`: 세그먼트 바·취소). 슬롯 `st.cloud_progress`/`st.cloud_shared` 1개를 클라우드 전송·가상 붙여넣기가 공용. 공유 상태 `TransferShared{cancel, done_bytes, total_bytes, items, in_flight}` | `win.rs:873-879` · `:1198-1222` · `:4308-4310` | `NexaProgress` 창 | **A** | 없음 |
| CLOUD-089 | 진행 갱신 | `on_cloud_progress`: 공유 상태 스냅숏 → `Progress::update(done,total,items,cur,count)`(cur = Active 세그먼트 번호, 없으면 비Pending 개수) → 취소 눌림이면 `shared.cancel = true` → 타이틀 `ops.progress({pct})` | `win.rs:4255-4287` · `:9135-9140` | `WM_APP_CLOUD_PROGRESS` | A | 없음 |
| CLOUD-090 | 취소 폴링 타이머 | `TIMER_CLOUD_POLL(13)` 200ms — 워커 통지와 **무관하게** 취소 버튼을 읽는다(청크 없는 단일 요청 구간 대비). 공유 상태 없으면 스스로 해제 | `win.rs:102-106` · `:4290-4293` · `:9445-9456` | `SetTimer`/`KillTimer` | **P**(타이머) | 없음 |
| CLOUD-091 | 진행 창 마감 | `finish_cloud_progress(note)`: 공유 상태 해제 · 타이머 정지 · 창을 완료 표기로 전환(`set_done(note, transfer_close_ms)` = [닫기 (N)] 카운트다운) · 호스트 백스톱 타이머 `ms+500` | `win.rs:4296-4305` · `dialog.rs:754-765` | 타이머 | A | 없음 |
| CLOUD-092 | 워커 → UI 통지 | 종결 통지(목록·다운로드·쓰기·인증) = `post_final_notify`(실패 시 100ms 간격 최대 50회 재시도, 페이로드 `Box` 원시 포인터를 lparam으로). 진행 통지 = 단발(유실 허용). 메시지 번호 `0x800E`(AUTH) `0x800F`(LIST) `0x8010`(DOWNLOAD) `0x8011`(WRITE) `0x8012`(PROGRESS) | `win.rs:156-203` · `:7048-7056` | `PostMessageW` | **P** | 없음 |

### 1.12 문서상 정책 · 미구현 항목

| ID | 항목 | 내용 | 근거 | 분류 |
|---|---|---|---|---|
| CLOUD-093 | 임포트 화이트리스트(B3) 확장 | `winhttp.dll` · `crypt32.dll` · `ws2_32.dll` · `bcrypt.dll` 4개 추가(전부 OS 인박스). B1(RSS ≤30MB)·B2(exe ≤10MB) 불변 | `adr6:39-48` · `nexa-dir2/scripts/budget-b3.ps1:20-24` | W(정책은 dir3에서 재정의 필요) |
| CLOUD-094 | 시크릿 동봉 정책 — "유지 확정" | client_id는 비밀 아님. Google은 PKCE만으로 대체 불가(secret 요구), Dropbox는 가능하나 사용자 결정으로 유지, MS는 없음. 방어선 = 리디렉션 URI 화이트리스트 + 브라우저 로그인 + 세션별 verifier. 재검토 금지 명시 | `adr6:72-95` · `oauth.rs:35-39` · `:89-97` | N(정책 계승) |
| CLOUD-095 | 서비스별 배포 한도 | Dropbox: 500명·50명 시 2주 내 프로덕션 신청. OneDrive: 개인 계정 무제한, **조직 계정은 게시자 확인 전 동의 불가**. Google: 테스트 사용자 누적 100명·테스트 모드 refresh 7일 만료 | `adr6:66-70` · `:97-124` · `oauth.rs:79-83` | N(운영 정보) |
| CLOUD-096 | 파일 온디맨드 방어·상태 표시(β) | 플레이스홀더(`RECALL_ON_DATA_ACCESS 0x40_0000`·`OFFLINE`) 미리보기 지연, 상태 컬럼, pin/unpin 일괄, cldapi 즉시 하이드레이션 — **검토서 제안**. 구현 확인된 것은 정보 패널 상세 생략뿐(`nexa-dir2/crates/nexa-app/src/fileinfo.rs:12` · `:39` · `:57`), 나머지는 미구현(Grep 0건) | `doc26:67-98` | W |
| CLOUD-097 | 클라우드 공급자 WASM 플러그인화 | `nx_meta` v2 `kind:"cloud"` · `nx_http`(토큰 비노출·호스트 헤더 주입·도메인 허용목록) — **후속, 미구현**. 1차는 호스트 내장으로 확정 | `doc26:100-146` · `:182-187` · `adr6:129-135` | N(미구현 — 이식 대상 아님) |
| CLOUD-098 | iCloud Drive 탐지 | `%USERPROFILE%\iCloudDrive` 프로브 — 검토서 표에만 있고 **코드 없음** | `doc26:49` · `cloud.rs:34-46` | — |
| CLOUD-099 | 문자열 자원 | `menu.cloud` 1 + `cloud.*` 41 = 42키(ko 42~84줄 — 58줄은 주석, en 43~85줄, ja는 Grep 42건으로 동수 확인·줄 번호 미확인) + 재사용 키 `ops.progressTitle`·`ops.cancel`·`ops.canceled`·`ops.progress`·`ops.fileCount`·`ops.close`·`del.close`·`nav.mypc`·`new.folderBase` | `ko.lang:42-84` · `:111` · `:116` · `:137` · `:220-221` · `:414` · `:416` · `:497` | N |

## 2. 화면·컨트롤 배치

### 2-1. 메뉴 바 — Cloud 메뉴(`win.rs:489-538`)

최상위 순서: 파일 · 편집 · 보기 · **클라우드**(`menu.cloud`) · 도움말(`win.rs:436-483`). 내용(위→아래):

```text
┌ <연결 0 라벨> ▸ ─ 바로 가기        (CMD 300+i  cloud.goto)
│                  온라인 보기      (CMD 340+i  cloud.web)
│                  URL 복사         (CMD 380+i  cloud.copyUrl)
│                  ─────────
│                  연결 해제 | 링크 해제 (CMD 420+i  API면 cloud.disconnect / 링크면 cloud.unlink)
├ <연결 1 라벨> ▸ …                 (최대 32개)
├ ───────── (연결이 1개 이상일 때만)
├ 링크 추가하기… ▸ <후보 0 라벨>   (CMD 460+i)   ← 후보 0개면 하위 메뉴 없는 단독 항목 CMD 299
├ ─────────
└ 클라우드 직접 연결 ▸ OneDrive (CMD 492) / Google Drive (493) / Dropbox (494)
```

- 하위 메뉴 깊이 = **1단계**. 단축키 표시 없음(전 항목 `""`). 체크·비활성 상태 없음.
- 명령 id 대역은 연속(`460+32 = 492`)이며 `run_command`는 **OAuth 대역을 ADD 대역보다 먼저** 검사한다(`win.rs:5516` < `:5527`).

### 2-2. 내 PC 배경 우클릭 팝업(`win.rs:3205-3249`)

- 위치 = 커서(좌상단 정렬). 항목: `{라벨} 링크`(후보별) → 연결된 항목(체크 표시 + 회색·선택 불가) → 둘 다 없으면 회색 1줄 `cloud.addNone`. 구분선 없음.

### 2-3. 내 PC 뷰의 클라우드 행(`tree:263-271` · `vfs:147-162`)

- 드라이브 행들 **뒤**에 연결 라벨 행. 종류 = 폴더, 크기 0, 수정일 빈 셀. 정렬은 트리 공통 정렬을 탄다(`tree:284`).
- 진입 경로 = `Entry.target`(동기화 폴더 실경로 또는 `::CLOUD:<i>::`).

### 2-4. 목록 안의 안내 행 · 배지 · 타이틀

- 로딩 중: 파일 1행 `불러오는 중…`(`win.rs:645-652`) + 패널 우하단 배지 `cloud.loading`(`win.rs:602-603`).
- 실패: 파일 1행 `[!] {사유}`(`cloudfs.rs:143-150`) + 타이틀 꼬리 ` · {사유} ({폴더})`(`win.rs:9120-9128`).
- 상태 안내는 전부 **창 타이틀 꼬리**(`update_title(" · …")`)로 나간다 — 링크됨/해제됨/URL 복사됨/대기 중/진행률 등.

### 2-5. 인증 안내 모달 · 오류 모달(`dialog::show_buttons` — 상세 배치는 `16-dir2-app-controls-dialogs.md` DLG-057)

| 요소 | 값 | 근거 |
|---|---|---|
| 창 | 캡션+시스템 메뉴(닫기 X) 팝업, 모달 프레임, 소유자 중앙에서 **+110px 아래**, 소유자 입력 차단 | `dialog.rs:206-213` · `:226-242` · `:281` |
| 여백·간격 | `PAD = 12` · 버튼 간 `GAP = 6` | `dialog.rs:56-57` |
| 본문 | 좌상단 (12,12), 폭 = 클라이언트 폭 − 24, 워드랩, 높이 실측(최소 16) | `dialog.rs:197` · `:218-223` · `:87-102` |
| 클라이언트 폭 | `max(버튼 폭 합 + 간격 + 24, 380)` | `dialog.rs:195-196` |
| 버튼 | 폭 `max(글자 폭+20, 56)` · 높이 `줄 높이+10` · 본문 아래 12px · **우측 정렬**(왼→오 순서 유지) · 첫 버튼 = 기본 강조 | `dialog.rs:191-198` · `:245-280` |
| 인증 안내 | 제목 `cloud.connect`. 버튼 순서 [브라우저 열기](기본) [URL 복사] [취소]. 본문에 **인증 URL 전문**(수백 자 — 워드랩으로 여러 줄) | `win.rs:5666-5682` |
| 미설정/실패 | 제목 `cloud.connect`. 버튼 [닫기] 1개 | `win.rs:5649-5655` · `:5749-5757` |
| 키 | Enter/Esc/Tab 처리 코드 없음(X = 0 반환 = 취소 취급) | `dialog.rs:283-287` · `win.rs:5683` |

### 2-6. 전송 진행 창(`dialog::Progress` — DLG 문서 2-5절과 같은 창)

| 요소 | 값 | 근거 |
|---|---|---|
| 창 | 비모달, 제목 `ops.progressTitle`, 클라이언트 폭 **400** 고정, 소유자 중앙에서 **−110px 위** | `dialog.rs:644-674` |
| 클라이언트 높이 | `12 + 줄×2 + 4 + 6 + 줄 + 8 + (줄+10) + 12` | `dialog.rs:648` |
| 1행 | 라벨(`cloud.progressLabel` → 완료 시 결과 문구) — (12,12) | `dialog.rs:549-556` · `win.rs:4238` |
| 2행 | `{완료 바이트} / {전체 바이트}  ({pct}%)  ·  파일 {cur}/{count}` — y = 12+줄+4 | `dialog.rs:558-578` |
| 바 | 2행 아래 6px, 높이 = 줄 높이, 테두리 1px 회색(`#808080`), 바탕 `#F0F0F0` | `dialog.rs:580-598` |
| 세그먼트 | 항목 크기 비례 폭 · 5색 순환 · 최소 3px · 건너뜀 회색/실패 적색 · 구간 경계선 · 항목 0개/512개 초과면 단색 폴백 | `dialog.rs:366-480` |
| 버튼 | 우하단 [취소] 폭 `max(글자+20, 64)`·높이 줄+10 → 완료 후 `[닫기 (N)]` 1초 단위 카운트다운, 0에서 자동 닫힘 | `dialog.rs:645-646` · `:693-718` · `:357-364` · `:513-530` · `:754-765` |
| 닫기 X | 진행 중 = 취소 요청 / 완료 후 = 즉시 닫기 | `dialog.rs:502-512` |

### 2-7. 브라우저에 돌려주는 페이지(`oauth.rs:507-521`)

- 성공: `<h3>Nexa Dir</h3>` + "인증이 완료되었습니다. 이 창을 닫으세요." + 영문 1줄. 실패: "인증에 실패했습니다. 앱으로 돌아가세요." + 영문 1줄. `Content-Type: text/html; charset=utf-8`.

### 2-8. 관련 키

| 키 | 클라우드 경로에서의 동작 | 근거 |
|---|---|---|
| F5 | 캐시 무효화 + 재조회 | `win.rs:5396-5405` |
| Del / Shift+Del | API 삭제(구분 없음) | `win.rs:3721-3731` |
| 인라인 이름변경 확정 | API 이름 변경 | `win.rs:4027-4035` |
| Ctrl+Shift+N | API 새 폴더 | `win.rs:443` · `:4071-4081` |
| Enter·더블클릭(파일) | 임시 폴더 다운로드 후 열기 | `win.rs:7061-7066` |
| Alt+↑ / 홈 | `cloud_parent` / 내 PC | `panel.rs:985-1022` |
| Ctrl+C·X·V, DnD | CLOUD-082 분기 | `win.rs:4364-4503` |

## 3. nexa-ui 매핑

실재 확인 = `nexa-ui/crates/nexa-ctl/src` Grep.

| dir2 요소 | nexa-ui 대응 | 실재 | 비고 / 추가 필요 API |
|---|---|---|---|
| 메뉴 바 Cloud 메뉴 + 1단계 하위 메뉴 | `MenuBar` · `MenuDef` · `MenuEntry::{Item, Sub, Separator, Disabled}` | 있음(`nexa-ui/crates/nexa-ctl/src/controls/pulldown.rs:25-37` · `:58-64` · `:84`) | 하위 메뉴 **1단계만** 지원 — Cloud 메뉴 구조와 정확히 맞는다. 후보 0개 항목은 `Disabled`로 표현 가능(dir2는 비활성 미지원이라 클릭 안내였음 — 동작 유지 여부 결정 필요) |
| 내 PC 우클릭 팝업(네이티브 `TrackPopupMenuEx`) | `ContextMenu` · `CtxItem::Item{enabled, mark, …}` | 있음(`nexa-ui/crates/nexa-ctl/src/controls/ctxmenu.rs:101-131` · `:272`) | 체크+회색 = `enabled:false` + `mark:Some(true)` |
| 메시지 모달 `show_buttons`(본문 워드랩 + N버튼) | — | **없음**(`nexa-ui/crates/nexa-dlg/src/lib.rs`는 `FilePicker`뿐 — `:154`) | **추가 필요**: 메시지 대화상자(제목·워드랩 본문·버튼 목록·기본 버튼·결과 id). 클라우드 요구: **긴 URL 본문**(공백 없는 수백 자 → 글자 단위 줄바꿈 필요), 본문 텍스트 **선택·복사 가능**하면 [URL 복사] 버튼과 중복 없이 유용(선택 사항). 창 골격은 nexa-sql `about_win.rs`(winit 창 + `Presenter` + `Button`) 선례(`nexa-sql/crates/nexa-sql/src/about_win.rs:1-25`) |
| 버튼(브라우저 열기/URL 복사/취소/닫기) | `Button` | 있음(`nexa-ui/crates/nexa-ctl/src/controls/button.rs:90`) | — |
| 진행 창의 [닫기 (N)] 카운트다운 | `TimeoutButton` | 있음(`nexa-ui/crates/nexa-ctl/src/controls/timeout_button.rs:33-61`) | 남은 초 표기·만료 자동 발화. 호스트가 `tick` 주입 |
| 세그먼트 진행 바 | — | **없음**(`progress` Grep = `tokens.rs` 애니메이션 변수뿐) | **추가 필요**: `SegmentedProgress`(항목별 `size/done/status`, 5색 순환, 최소 3px, 상태색, 단색 폴백) — DLG 문서와 공용 요구 |
| 진행 창(비모달, 라벨 2줄 + 바 + 버튼) | 조립(winit 창 + 위 컨트롤) | 조립 필요 | 클라우드는 로컬 전송 진행 창과 **같은 구현을 공유**해야 한다(CLOUD-088) |
| 패널 "불러오는 중…" 플로팅 배지 | — | 없음(추정 — 패널 인벤토리 `13-dir2-panel-filelist.md` 소관) | 패널 컨트롤의 배지 API로 흡수 |
| 목록 안내 행(로딩/오류) | 파일 목록 모델의 일반 행 | 데이터만 | 별도 컨트롤 불요 |
| 상태 안내(타이틀 꼬리) | 창 타이틀 문자열 | — | nexa-sql은 토스트(`nexa-sql/crates/nexa-sql/src/toast.rs:1-10`)도 보유 — dir2 배치 유지 원칙상 타이틀 꼬리를 그대로 재현 |
| 클립보드 텍스트 쓰기 | 앱측 구현(nexa-ui에 없음) | nexa-sql 선례(`nexa-sql/crates/nexa-sql/src/clipboard.rs:1-22`) | 3-OS 분기(§4) |

## 4. OS 분기점

### 4-1. 한눈표

| 영역 | Windows(현 구현) | macOS | Linux | 관련 ID |
|---|---|---|---|---|
| 동기화 클라이언트 탐지 | 레지스트리 · 볼륨 라벨 · `%APPDATA%\Dropbox\info.json` | `~/Library/CloudStorage/` 하위 폴더 스캔(이름 접두 `OneDrive-…`, `GoogleDrive-…`, `Dropbox`) + 구형 위치(`~/OneDrive`, `/Volumes/GoogleDrive`) + **`~/.dropbox/info.json`**(Dropbox 공식 위치 — 파서 CLOUD-005 그대로) + iCloud `~/Library/Mobile Documents/com~apple~CloudDocs`(전부 추정 — 실기 확인 필요) | `~/.dropbox/info.json`(파서 그대로). OneDrive·Google Drive는 공식 클라이언트 없음 → 비공식(`~/OneDrive`, GVfs `/run/user/<uid>/gvfs/google-drive:*`)은 선택(추정) | 001~004 |
| HTTP/TLS | WinHTTP + schannel | §4-2 | §4-2 | 039~040 |
| 토큰 보관 | DPAPI hex 파일 | §4-3 | §4-3 | 035~038 |
| SHA-256(PKCE) | CNG `bcrypt.dll` | 공통 구현 권장: 계열에 이미 있는 `sha2`(`nexa-sql/crates/nsql-vault/Cargo.toml` · `nexa-license/crates/nexa-license/Cargo.toml:30`) — OS 분기 제거 | 좌동 | 025 |
| 암호학적 난수 | `BCryptGenRandom` | 공통: `getrandom`(nsql-vault 선례 `nexa-sql/crates/nsql-vault/src/devkey.rs:17-21`) 또는 OS 직접(`getentropy`) | 좌동(또는 `/dev/urandom`) — **현 xorshift 폴백 제거 필수** | 026 |
| 루프백 리스너 | `std::net`(ws2_32) | `std::net` 그대로 | 그대로(IPv6 비활성 환경은 `::1` 바인딩 실패 = 이미 무시 처리) | 024 · 029 |
| 브라우저/URL 열기 | `ShellExecuteW("open", url)` | `open <url>` | `xdg-open <url>` | 013 · 028 |
| 파일 열기(연결 프로그램) | `ShellExecuteW("open", path)` | `open <path>` | `xdg-open <path>` | 070 |
| 포그라운드 양도 | `AllowSetForegroundWindow(ASFW_ANY)` | 불요(LaunchServices가 앞으로 가져옴 — 추정) | 불요(창 관리자 소관) | 013 · 070 |
| 클립보드 텍스트 | Win32 클립보드 | nexa-sql 선례: `pbcopy`(`nexa-sql/crates/nexa-sql/src/clipboard.rs:6-9`) | `wl-copy` → `xclip` → `xsel` 또는 X11 직접(`nexa-sql/crates/nexa-sql/src/clipboard_x11.rs`) | 014 · 028 |
| 워커 → UI 통지 | `PostMessageW(WM_APP_*)` + Box 포인터 + 재시도 | 채널 + `EventLoopProxy::send_event(Wake)`(nexa-sql 선례 `nexa-sql/crates/nexa-sql/src/app/files.rs:535-546` · `app/event_loop.rs:187`) — 3-OS 동일 | 좌동 | 092 |
| 취소 폴링 타이머 | `SetTimer` 200ms | 이벤트 루프 틱(winit `WaitUntil`) — 3-OS 동일 | 좌동 | 090 |
| 모달·진행 창 | Win32 창 + `BUTTON` | nexa-ui 조립(§3) — 3-OS 동일 | 좌동 | 023 · 028 · 088 |
| 팝업 메뉴 | `TrackPopupMenuEx` | `ContextMenu` — 3-OS 동일 | 좌동 | 010 |
| 표시 경로 구분자 | `\`(`vfs:218`) | `/`로 표기(역변환은 이미 둘 다 허용 `vfs:263`) | `/` | 051 |
| 센티널 충돌 가정 | "콜론은 파일명에 불가"(`vfs:175`) | 콜론이 파일명에 **가능**. 다만 실경로는 항상 `/`로 시작하는 절대 경로라 `::CLOUD:` 접두와 겹치지 않음 → 판정은 유지 가능. **상대 경로를 탐색 입력으로 받는 곳**에서만 주의 | 좌동 | 050 |
| 임시 스테이징 | `%TEMP%\NexaDir\{cloud,xcopy,vpaste-*}` | `std::env::temp_dir()` 그대로(`$TMPDIR`) | 그대로(`/tmp`) | 070 · 081 · 086 |
| 데이터 폴더 | exe 옆 `data\` → 쓰기 불가면 `%LOCALAPPDATA%\NexaDir\data`(`config.rs:358-373`) | nexa-sql 규칙으로 교체: `nexa_conf::user_config_dir`(`nexa-ui/crates/nexa-conf/src/lib.rs:310-334`) + 관리형 설치 자리 회피(`:336-348`) | 좌동 | 035 |
| 임포트 게이트 | B3 화이트리스트(`budget-b3.ps1`) | 해당 없음(동적 로드 대상 목록으로 재정의) | 좌동 | 093 |

> **브라우저 열기 주의(관찰)**: nexa-sql의 `open_external`은 Windows에서 `cmd /C start "" <p>`를 쓴다(`nexa-sql/crates/nexa-sql/src/main.rs:1190-1203`). 인증 URL은 `&`가 다수 들어 있어 `cmd` 해석에 깨진다 — **Windows는 `ShellExecuteW` 직접 호출을 유지**해야 한다.

### 4-2. HTTP/TLS — 선택지 비교

**교체 대상이 지켜야 할 계약**(CLOUD-039에서 추출): 메서드 GET/POST/PUT/PATCH/DELETE · 임의 요청 헤더(`Content-Range`, `Dropbox-API-Arg`) · 본문 없는 POST(Content-Type도 없음) · **빈 본문 + Content-Type 있는 POST**(Dropbox finish) · 응답 상태 코드(2xx 판정, Google 308) · 응답 헤더 `Location`·`Content-Length` · 응답 본문 상한 · 수신 진행 콜백과 취소 · 시스템 프록시 · HTTPS 전용 · User-Agent `NexaDir`.

| 안 | 구성 | 장점 | 단점·위험 |
|---|---|---|---|
| **A. OS 네이티브 API별 구현** | Windows = WinHTTP(현 코드. dir3는 `windows` crate 대신 **수동 `extern`** 10개 함수로 — nexa-sql DPAPI 바인딩 관례 `nexa-sql/crates/nsql-vault/src/devkey.rs:126-163`). macOS = `NSURLSession`(objc 런타임 + 블록 ABI 수작업) 또는 시스템 **libcurl**(`/usr/lib/libcurl.4.dylib` — OS 제공 공개 라이브러리. 최신 macOS는 dyld 공유 캐시에서 로드 — 추정·실기 확인 필요). Linux = 단일 네이티브 TLS API가 없음 → 시스템 `libcurl.so.4`(없으면 `libcurl-gnutls.so.4`) 동적 로드 | 외부 crate 0 · TLS·인증서 저장소·프록시가 OS 관리(보안 갱신 자동) · Windows 회귀 0(현 코드 직이식) · 스트리밍/진행/취소를 콜백으로 제대로 구현 가능 | OS별 구현 2~3벌 + 테스트 3벌 · `NSURLSession` 순수 FFI는 비용 큼(블록·델리게이트) → 실질적으로 mac/Linux는 libcurl FFI(가변 인자 `curl_easy_setopt`) · Linux 최소 설치에 libcurl 없을 수 있음(추정) → 부재 시 기능 비활성 안내 필요 |
| **B. `curl` 프로세스** | Windows 10 1803+ `System32\curl.exe`(schannel) · macOS `/usr/bin/curl` · Linux `curl`(PATH) | 구현 1벌 · crate 0 · FFI 0 | **토큰이 명령줄에 실리면 프로세스 목록에 노출** → 반드시 `--config`(stdin 또는 0600 임시 파일)로 헤더 전달 · 요청마다 프로세스 생성(폴더 진입마다 2회: refresh+목록) · 진행은 출력 파일 크기 폴링/`--progress-bar` 파싱 · 대용량 본문은 임시 파일 경유(청크 업로드마다 파일 조각 필요) · Windows 콘솔 창 깜빡임 방지 플래그 필요 · Linux에 `curl` 미설치 가능(Ubuntu 데스크톱 기본 미포함 — 추정) · 외부 실행 파일 의존(dir2 검토서가 rclone 동봉을 기각한 근거와는 다름 — OS 제공분 사용) |
| **C. 최소 crate** | `ureq`(블로킹) + TLS(`rustls` 또는 `native-tls`) | 구현 1벌 · 3-OS 동일 동작(회귀 테스트 단순) · 스트리밍·진행·취소 자연스러움 · 계열 선례 있음: nexa-sql은 "암호화 자체 구현 금지 부류"를 원장 예외로 허용(`nexa-sql/crates/nsql-vault/Cargo.toml` 주석 · `nexa-sql/Cargo.toml:4`)하고 `rustls`도 이미 mssql 드라이버 경유로 들어와 있음(`nexa-sql/crates/nsql-driver-mssql/Cargo.toml:22`) | 의존 트리 증가(수십 crate — 추정) · 바이너리 +1.5~3MB(추정 — 실측 필요, dir2 B2 예산 10MB) · `rustls`면 인증서 루트 전략(내장 vs OS 저장소) 결정 필요 · 사내 TLS 검사 프록시 환경에서 OS 저장소 미사용 시 실패 · ADR-0006이 기각했던 안(`adr6:35`)이라 **결정 번복 = ADR 개정** 필요 |

**권고(구현 지침 — 최종 선택은 프로젝트 결정 사항)**

1. 어느 안이든 **포트(트레이트) 먼저**: `HttpTransport::request(HttpRequest{method,url,headers,body}, limit, progress, want_location) -> Result<HttpResponse{status,location,content_length,body}, HttpError>`. `oauth.rs`·`cloudfs.rs`의 서비스 로직(CLOUD-021~034, 063~081)은 이 포트만 보게 분리 → **가짜 전송 계층으로 서비스 로직 전부를 3-OS에서 단위 테스트**(§7).
2. 1차 구현 = **A안**: Windows WinHTTP(직이식) + macOS·Linux 시스템 libcurl 동적 바인딩(둘이 한 구현을 공유 — 실질 2벌). Linux에서 libcurl 로드 실패 시 **B안(curl 프로세스)을 폴백**으로 두거나 "클라우드 직접 연결 사용 불가" 안내.
3. libcurl 바인딩 최소 표면: `curl_global_init` · `curl_easy_init` · `curl_easy_setopt` · `curl_easy_perform` · `curl_easy_getinfo` · `curl_easy_cleanup` · `curl_slist_append` · `curl_slist_free_all` · `curl_easy_strerror`. 필요한 옵션: URL · CUSTOMREQUEST · HTTPHEADER · POSTFIELDS/POSTFIELDSIZE_LARGE · WRITEFUNCTION/WRITEDATA · HEADERFUNCTION/HEADERDATA(`Location` 수집) · XFERINFOFUNCTION/NOPROGRESS(진행·취소 — 0이 아닌 값 반환 = 중단) · USERAGENT · NOSIGNAL · 응답 코드 조회(상수 값은 구현 시 `curl/curl.h`로 대조).
4. 동작 동등성 주의: WinHTTP는 리디렉션 자동 추종·기본 타임아웃을 쓴다(명시 설정 없음 — `oauth.rs:963-1014`). 대체 구현도 리디렉션 추종을 켜고, **연결/수신 타임아웃을 명시**해 워커가 영구 대기하지 않게 한다(현 구현의 암묵 의존 제거).

### 4-3. 토큰 보관 — 선택지

**nexa-sql `nsql-vault` 방식 요약**(기준):

- 키 계층: `device.key`(32바이트 난수) 1개 + 비밀값마다 **봉투**. 보호 수단을 바꿔도 비밀값 재암호화가 필요 없게 마스터 키를 파일로 둔다(`nexa-sql/crates/nsql-vault/src/devkey.rs:1-9`).
- `device.key`: Windows = `"NSDK"`‖ver‖DPAPI 블롭(앱 엔트로피 + `CRYPTPROTECT_UI_FORBIDDEN`), macOS·Linux = **평문 32바이트·파일 모드 0600**("폴더째 복사에는 못 버틴다 — Keychain·Secret Service 결합은 후속"이라고 명시). 동시 첫 실행 경쟁은 `create_new`로 한 프로세스만 생성(`devkey.rs:14-124` · `:126-235`).
- 봉투: `"NSSE"`‖ver‖salt16‖nonce12‖암호문+태그. ChaCha20-Poly1305, 키 = SHA-256(라벨‖도메인‖salt‖기기 키), AAD = 헤더‖도메인 → **다른 자리로 옮겨 붙이면 열리지 않음**(fail-closed)(`nexa-sql/crates/nsql-vault/src/sealed.rs:1-85`).
- 외부 crate: `chacha20poly1305` · `sha2` · `getrandom`(원장 예외 — `nexa-sql/crates/nsql-vault/Cargo.toml`).

| OS | 1차(권고 — nsql-vault 방식 그대로) | 2차 강화(선택) |
|---|---|---|
| Windows | `device.key` DPAPI 봉투 + 토큰 봉투. dir2와 같은 보호 수준(같은 Windows 계정만 복호) | — |
| macOS | `device.key` 평문 0600 + 토큰 봉투(정직한 한계 명시) | **Keychain**: Security.framework `SecItemAdd` / `SecItemCopyMatching` / `SecItemUpdate` / `SecItemDelete`, `kSecClassGenericPassword`, 서비스명·계정명으로 **기기 키 1개만** 보관(토큰마다 넣지 않음 — 프롬프트 최소화). 주의: 서명되지 않은/빌드마다 서명이 바뀌는 바이너리는 접근 허용 프롬프트가 반복됨(추정). `security` CLI는 비밀값이 명령줄에 실려 부적합 |
| Linux | `device.key` 평문 0600 + 토큰 봉투 | **Secret Service**(D-Bus `org.freedesktop.secrets`): `libsecret-1.so.0` 동적 로드(`secret_password_store_sync` / `_lookup_sync` / `_clear_sync`) 또는 `secret-tool store/lookup/clear`(비밀값은 stdin). 데스크톱 세션·키링 데몬이 없으면 실패 → **파일 봉투 폴백 유지** |

- **봉투 도메인 = 연결의 안정 식별자**(예: `cloud-v1/<kind>/<account>` 또는 연결 고유 id)로 한다 → dir2의 "토큰 파일 = 연결 인덱스" 결합에서 오는 결함(§6 L-13)이 구조적으로 사라진다.
- **포터블 특성 차이(결정 필요)**: dir2는 "`data\`를 옮기면 복호 실패 = 재로그인"이 의도된 안전 특성이다(CLOUD-038). macOS·Linux 1차안은 `device.key`가 같은 폴더에 평문으로 있으면 폴더 복사로 토큰이 따라간다. 완화: `device.key`를 **데이터 폴더가 아닌 사용자 설정 폴더**에 두거나(포터블 폴더만 옮기면 열리지 않음), 2차 강화를 적용.
- dir2 토큰 파일(`cloud<N>.tok` = 엔트로피 없는 DPAPI hex)을 dir3가 이어받을지 여부는 설정 이관 정책 소관(추정: 재로그인으로 충분).

### 4-4. 그 밖의 분기 메모

- **PKCE·난수**는 OS 분기 대신 계열 crate(`sha2`·`getrandom`)로 단일화하는 편이 단순하다 — 둘 다 nexa-license/nsql-vault로 이미 워크스페이스에 들어온다(라이선스 적용 시 동반).
- **JSON**: 수제 파서(CLOUD-042~044)는 그대로 이식 가능. nexa-sql에도 자체 JSON 모듈이 있다(`nexa-sql/crates/nsql-settings/src/json.rs` 509줄 — 용도는 설정 내보내기, 재사용 가능성은 미확인/추정).

## 5. 상태·영속 · 스레딩·메시지 흐름

### 5-1. 영속

| 대상 | dir2 형식 | 위치 | dir3 제안(설정 구조 = nsql-settings 차용) |
|---|---|---|---|
| 연결 목록 | `cloudN=kind\|라벨\|경로\|account` | `data\settings.cfg`(`config.rs:527-536`) | `cloud.conn{N}`(PREFS-167 제안과 동일 — `nexa-dir3/docs/port/15-dir2-prefs-config.md:372`). 레지스트리는 고정 키 표라 **가변 개수 항목**의 표현(번호 키 vs 별도 파일)을 정해야 함. 안정 id 필드 추가 권장 |
| client_id 재정의 | `cloud_client_id_<kind>=…` | 〃(`config.rs:538-542`) | `cloud.client_id.<kind>`(PREFS-168) — 설정 화면 노출 여부 결정(dir2는 UI 없음) |
| client_secret 재정의 | `cloud_client_secret_<kind>=…` | 〃(`config.rs:544-548`) | `cloud.client_secret.<kind>`(PREFS-169) |
| refresh 토큰 | DPAPI hex 1줄 | `data\secrets\cloud<N>.tok`(`secret.rs:14-21`) | §4-3 |
| 목록·ID 캐시 | 없음(메모리) | — | 그대로(메모리) |
| 다운로드 임시 파일 | `<대상>.nexadl.part` → rename | 대상 폴더(`cloudfs.rs:540-545`) | 그대로 |
| 스테이징 | `NexaDir\cloud`(열기용 — 정리 코드 없음, OS 임시 정리 의존/추정) · `NexaDir\xcopy`(잡 종료 시 삭제 `cloudfs.rs:738`) · `NexaDir\vpaste-<pid>-<seq>` | `%TEMP%` | 그대로(`temp_dir()`) |

런타임 상태(`State`): `cloud_conns` · `cloud_cands`(메뉴 구성 시점 후보 스냅숏) · `cloud_client_ids` · `cloud_client_secrets` · `cloud_progress` · `cloud_shared`(`win.rs:864-879`). 전역: `CACHE`·`IDS`·`INFLIGHT`(`cloudfs.rs:28-33`) · `EXTRA_ROOTS`·`CLOUD_LISTER`(`vfs:138` · `:275`).

### 5-2. 스레딩 원칙

- **UI 스레드는 네트워크를 타지 않는다**(`cloudfs.rs:3-13` · `oauth.rs:8-10`). 워커는 `State`에 접근하지 않고 `ConnInfo` 스냅숏(kind·client_id·client_secret·refresh)만 받는다(`cloudfs.rs:165-174`).
- 워커는 잡마다 `std::thread::spawn` 1개(목록 요청은 폴더마다 1개 — 동시 다수 가능, 전송 잡은 슬롯 1개).
- 예외(관찰): `cloud_candidates`(레지스트리·볼륨·파일 읽기)와 `secret::load_token`(DPAPI 복호)은 UI 스레드 동기 호출(`win.rs:634` · `:4168` · `:541-550`).

### 5-3. 서비스 정의 값(비밀값 제외)

| | OneDrive | Google Drive | Dropbox |
|---|---|---|---|
| kind / display | `onedrive` / `OneDrive` | `googledrive` / `Google Drive` | `dropbox` / `Dropbox` |
| auth_url | `https://login.microsoftonline.com/common/oauth2/v2.0/authorize` | `https://accounts.google.com/o/oauth2/v2/auth` | `https://www.dropbox.com/oauth2/authorize` |
| token_url | `https://login.microsoftonline.com/common/oauth2/v2.0/token` | `https://oauth2.googleapis.com/token` | `https://api.dropboxapi.com/oauth2/token` |
| scope | `offline_access Files.ReadWrite User.Read` | `https://www.googleapis.com/auth/drive` | `files.metadata.write files.metadata.read files.content.write files.content.read account_info.read` |
| me_url(방식) | `https://graph.microsoft.com/v1.0/me`(GET) | `https://www.googleapis.com/drive/v3/about?fields=user`(GET) | `https://api.dropboxapi.com/2/users/get_current_account`(**POST**) |
| 기본 client_id | 있음(`oauth.rs:65`) | 있음(`oauth.rs:90`) | 있음(`oauth.rs:116`) |
| 기본 client_secret | 없음 | 있음(`oauth.rs:97`) | 있음(`oauth.rs:117`) |
| 리디렉션 포트 | 42813 → 임의 | 42813 → 임의 | **53682·53683·53684만** |
| URI 끝 슬래시 | 있음 | 있음 | **없음** |
| 콘솔 등록값 | `http://localhost` | 데스크톱 클라이언트(루프백 임의 포트) | `http://localhost:5368{2,3,4}` |
| 근거 | `oauth.rs:54-70` | `oauth.rs:74-101` | `oauth.rs:104-123` |

### 5-4. 흐름

```text
[인증]  메뉴 492+i ─ start_cloud_oauth(UI) ─ AuthSession::begin(리스너·URL, 네트워크 0)
          └ 모달(브라우저 열기/URL 복사/취소) ─ thread: wait_and_exchange(300s)
               └ accept 폴링 120ms → code → POST token → (GET/POST me)
               └ post_final_notify(WM_APP_CLOUD_AUTH, Box<CloudAuthResult>)
          UI: on_cloud_auth → 연결 등록/갱신 → secret::save_token → apply_cloud_change

[목록]  트리 열거(UI) ─ vfs::cloud_entries ─ lister(idx, inner)
          ├ cache_get 히트 → 즉시 반환
          └ 미스 → ConnInfo 스냅숏 → cloudfs::request ─ thread: refresh → fetch_list → parse_list → cache_put
                    └ [불러오는 중…] 1행 반환          └ post_cloud_list(Box<ListResult>)
          UI: WM_APP_CLOUD_LIST → 두 패널 reopen_cloud_tabs(이번엔 캐시 히트)

[전송]  start_cloud_download / start_cloud_write / start_cross_copy(UI)
          ├ Progress::open(조건부) + begin_cloud_progress(공유 상태 + 200ms 타이머)
          └ thread: token_for → 항목 루프(취소 검사 · 세그먼트 갱신)
                 ├ post_cloud_progress(단발)  → UI: on_cloud_progress(창 갱신 · 취소 → shared.cancel)
                 └ post_cloud_download / post_cloud_write(재시도 게시)
          UI: 완료 처리 → finish_cloud_progress([닫기 (N)] 카운트다운) → 패널 재로드
```

## 6. 이식 시 주의 — 실측 교훈 · 결함 수정 이력 · 관찰

### 6-1. 원본 주석·문서에 남은 교훈(회귀 방지 필수)

| # | 교훈 | 근거 |
|---|---|---|
| L-01 | 리디렉션 호스트는 **`localhost`** — `127.0.0.1`로 바꾸면 Entra가 `redirect_uri is not valid`로 거부(MS는 `localhost`에 한해 포트 무시, `http`+`127.0.0.1`은 포털 등록 불가) | `oauth.rs:333-344` · `:365-367` · `:1186-1202` |
| L-02 | Dropbox는 등록 URI와 **정확 일치**(포트·슬래시까지) — 등록 포트 3개만 시도, 임의 포트 폴백 금지, 끝 슬래시 없음 | `oauth.rs:40-47` · `:118-121` · `:376-394` · `:1213-1229` |
| L-03 | `localhost`가 `::1`로 먼저 해석되는 환경 대비 **IPv4+IPv6 동시 대기** | `oauth.rs:350-353` · `:397-400` |
| L-04 | Dropbox RPC는 **POST 전용**이고 인자 없는 호출은 **본문도 Content-Type도 없어야** 한다(빈 본문 + `application/json` = 400). 계정 조회 실패 → 라벨 "Dropbox – account" + 내 PC 누락으로 이어졌다 | `oauth.rs:112-114` · `:455-461` · `:760-777` |
| L-05 | API 연결 판정은 **경로 유무**(account 유무 아님) | `config.rs:53-60` |
| L-06 | 재인증 시 같은 종류의 **빈 계정 행을 흡수**(유령 행 방지) | `win.rs:5770-5789` |
| L-07 | Dropbox write scope 누락 = 403(쓰기 슬라이스 이후 필수) · OneDrive `Files.ReadWrite` 상향 후 기존 연결은 재로그인 1회 → 401/403이면 재연결 안내 | `oauth.rs:59-61` · `:109-111` · `win.rs:9267-9268` |
| L-08 | Google `drive.file`은 목록 열거 불가 → `drive`(restricted). 계정 표시명은 `about?fields=user`(별도 scope 불요). refresh를 받으려면 `access_type=offline&prompt=consent` | `oauth.rs:79-88` · `:323-326` |
| L-09 | JSON `\uXXXX` 미해석 시 한글 이름이 깨지고 **그 이름으로 경로를 재구성해 조회하므로 not_found(409)** — 표시 문제가 아니라 동작 문제. 서러게이트 쌍도 결합 | `oauth.rs:646-671` · `:1157-1176` |
| L-10 | `Dropbox-API-Arg`는 **HTTP 헤더**라 ASCII만 가능 — 비ASCII는 `\uXXXX`(본문 JSON과 이스케이프 함수 분리) | `cloudfs.rs:1471-1490` · `:1724-1735` |
| L-11 | Graph `downloadUrl`·업로드 세션 URL·Google 재개 세션 URI는 **사전 인증** — Authorization 헤더를 붙이지 않는다 | `oauth.rs:779-782` · `cloudfs.rs:489-506` · `:873-883` · `:1590` · `:1613-1620` |
| L-12 | Graph 폴더 생성에 `conflictBehavior=replace` 금지(기존 폴더 통째 교체) — `fail` + 오류 무시로 멱등화 | `cloudfs.rs:1508-1514` |
| L-13 | 목록 실패도 **캐시에 담는다** — 안 담으면 재로드마다 재요청, 로딩 표시 영구 잔존, 원인 불가시 | `cloudfs.rs:136-152` |
| L-14 | 안내 행의 경로는 부모와 **달라야** 한다(제어문자 `\u{1}notice`) | `cloudfs.rs:106-111` |
| L-15 | 로딩 중 빈 목록은 식별 불가 → **플레이스홀더 1행** + 배지 | `win.rs:642-652` · `:588-605` |
| L-16 | 목록·쓰기 완료 시 **비활성 탭 포함 전부** 재열기(탭 전환만으로는 재열거되지 않음). 내 PC에서 펼친 클라우드 하위도 대상 | `win.rs:575-586` · `:9113-9118` · `panel.rs:1368-1392` |
| L-17 | 기동 시 세션 복원 탭은 콜백 등록 전에 만들어져 비어 있다 → 등록 직후 1회 재열기 | `win.rs:7676-7702` |
| L-18 | Google은 재시작 후 ID 캐시가 비어 하위 경로에서 영구 로딩 → **루트부터 경로를 되짚어 ID 해석**. 쓰기·계정 간 복사 경로도 같은 해석 사용 | `cloudfs.rs:215-219` · `:247-277` · `:1271-1274` |
| L-19 | 계정 간 복사는 **대상 폴더를 부모부터 먼저 생성**(Google은 부모 ID 필수) | `cloudfs.rs:640-664` · `:897-900` |
| L-20 | 진행률 단위는 **실제 바이트**(건수를 바이트 자리에 넣어 "0 B / 6 B"). 크기 미상(0)은 1로. 총량은 목록에서 아는 크기로 **선확정**(안 하면 세그먼트가 한 칸으로 뭉침) | `cloudfs.rs:338-340` · `:392-405` · `:665-678` · `:761-776` |
| L-21 | 계정 간 복사는 다운로드=앞 절반·업로드=뒤 절반(다운로드 끝 = 100%로 보이며 멈춘 듯한 문제) | `cloudfs.rs:466-469` · `:702-713` |
| L-22 | 큰 파일 업로드는 청크(진행 보고·취소 가능) — 단일 요청은 끝나야 상태가 바뀜(36MB 실측) | `cloudfs.rs:1287-1291` |
| L-23 | 취소 판정은 워커 통지와 **무관한 주기 폴링**으로(청크 없는 구간에서 [취소] 무반응) | `win.rs:102-106` |
| L-24 | 진행 창은 **바이트를 옮기는 작업에만**(삭제·이름변경·폴더 생성·서버 복사에 "0 B / 3 B" 무의미) | `win.rs:4342-4349` |
| L-25 | 사용자가 명시적으로 시작한 작업(인증)의 실패는 **모달로**(타이틀 문구는 놓침) | `win.rs:5742-5757` |
| L-26 | 경로 바 조각 클릭 역변환에서 링크(X-36) 항목 때문에 전체가 `None`이 되던 결함 — 건너뛰기(`continue`) | `vfs:253-258` |
| L-27 | 종결 통지 유실 = 상태 영구 고착 → 재시도 게시(진행 통지는 단발) | `win.rs:7041-7056` · `:196-203` |
| L-28 | 공유 Mutex는 poison 내성 lock(워커 panic이 UI 연쇄 panic으로 번지지 않게) | `win.rs:7031-7039` |
| L-29 | 토큰 파일 저장은 원자적(0바이트·반쪽 hex로 재로그인 강요 방지), 손상 hex는 panic 없이 None | `secret.rs:43-51` · `:73-85` |
| L-30 | client_secret이 파싱만 되고 직렬화되지 않아 다음 저장에 유실되던 결함 | `config.rs:543-548` |
| L-31 | 비Windows 빌드에서 cfg 블록이 사라지면 타입 추론 실패(E0282) — 타입 명시 | `cloud.rs:35-37` |
| L-32 | 동봉 ID는 전 사용자 쿼터 공유 → **사용자 재정의 경로 항상 유지** | `adr6:122-124` · `oauth.rs:18-21` |
| L-33 | 조직(회사·학교) MS 계정은 직접 연결 불가(게시자 확인 전) → 동기화 폴더 링크가 우회로 — 두 방식 공존 유지 | `adr6:108-120` · `:144-145` |

### 6-2. 코드에서 관찰한 한계·잠재 결함(이식 시 그대로 옮길지 결정 필요)

| # | 관찰 | 근거 | 권고 |
|---|---|---|---|
| O-01 | **목록 페이지네이션 미처리** — 500개 초과 폴더는 잘림(`@odata.nextLink`·`nextPageToken`·`has_more` 무시. 테스트 픽스처에는 두 키가 들어 있으나 검증 없음) | `cloudfs.rs:197-199` · `:220-224` · `:230-232` · `:1714` · `oauth.rs:1250` | 이식 시 페이지 루프 추가(기능 보강) |
| O-02 | **토큰 파일이 연결 인덱스에 결합** — ① API 연결 해제 시 `clear_from(i)`가 **뒤 슬롯 토큰까지 전부 삭제**(뒤 연결은 재로그인 필요) ② **링크(비API) 해제 시 토큰 미이동** → 뒤 API 연결의 인덱스가 당겨져 토큰 불일치(잘못된 슬롯 로드/부재) | `win.rs:5493-5502` · `secret.rs:39-41` · `:69-71` | 안정 식별자 키(§4-3) |
| O-03 | **account가 빈 API 연결은 재시작 시 탈락** — `is_api()`는 경로 유무로 판정하나 파서는 "경로 또는 account"를 요구 → 저장된 `kind\|라벨\|\|` 행이 무시되고 뒤 인덱스가 당겨짐 | `config.rs:58-60` vs `:819-820` · `:1465-1469` | 파서 조건을 kind 기준으로 정합 |
| O-04 | refresh 응답의 **새 refresh 토큰을 저장하지 않음**(`token_for`·`load_blocking`이 access만 사용) → 회전형 제공자에서 슬라이딩 갱신 효과 없음(만료 영향은 추정) | `cloudfs.rs:179-181` · `:450-455` · `oauth.rs:562-565` | 새 refresh를 UI로 전달해 저장 |
| O-05 | access 토큰 캐시 없음 — 폴더 진입·전송 잡마다 refresh 호출. 장시간 잡 중 만료(약 1시간) 재발급 없음 | `cloudfs.rs:177-183` · `:368` · `:616-617` · `:778` | 만료 시각 기반 캐시 + 401 시 1회 재발급 |
| O-06 | Dropbox 다운로드는 진행·취소 없음(다른 2사는 있음) | `cloudfs.rs:517-533` | 포트에 진행 콜백 통일 |
| O-07 | 다운로드·단순 업로드는 **전량 메모리 적재**(다운로드 상한 512MiB, Dropbox 단순 업로드 최대 140MiB) | `cloudfs.rs:329` · `:539-541` · `:1194-1199` · `:1292` · `:1579` | 파일 스트리밍(포트 설계 시 반영) |
| O-08 | 수신 루프가 read 오류에서 조용히 `break` → 2xx면 **잘린 본문을 성공으로 반환**할 수 있음(`Content-Length` 대조 없음) | `oauth.rs:1064-1080` · `:1093-1100` | 길이 대조·오류 전파 |
| O-09 | Google 재개 업로드의 중간 청크 오류 무시 — 실제 실패도 마지막 청크까지 진행 | `cloudfs.rs:873-886` | 308을 정상으로 인식하는 상태 판정 |
| O-10 | **이동(Move)이 복사로 동작**: 계정 간 · 클라우드→로컬 · 로컬→클라우드 모두 원본 미삭제. 같은 연결 안에서만 진짜 이동 | `win.rs:4391-4447` · `:4464-4484` · `:4498-4502` | 동작 유지 시 사용자 안내 필요 |
| O-11 | 클라우드 삭제는 확인창·undo 없음. 주석은 "서비스 휴지통으로 이동"이나 Google `DELETE files/{id}`는 휴지통을 거치지 않는 영구 삭제로 알려져 있음(추정 — 외부 API 규약 재확인 필요) | `win.rs:3721-3731` · `cloudfs.rs:1340-1348` | 확인 필요(휴지통 = `trashed:true` PATCH) |
| O-12 | Google 네이티브 문서(Docs·Sheets 등)는 `alt=media`로 받을 수 없음(추정) — export 분기 없음. 목록에는 크기 0 파일로 보임 | `cloudfs.rs:508-516` | 표시/안내 정책 결정 |
| O-13 | 여러 연결이 섞인 선택은 **첫 연결 항목만** 처리(나머지 무시·안내 없음) | `win.rs:4205-4207` · `:4405-4407` · `:3726` | 유지 또는 안내 |
| O-14 | 리디렉션 대기는 **첫 TCP 접속 1건**으로 판정 — 브라우저의 선행 빈 연결이 먼저 오면 `state mismatch`로 실패할 수 있음(추정) | `oauth.rs:494-529` | code 없는 접속은 무시하고 계속 대기 |
| O-15 | 인증 대기 취소 수단 없음 — 브라우저를 닫아도 5분간 리스너 점유. Dropbox는 후보 포트 3개라 연속 4회째는 `Listener` 오류 | `win.rs:5703-5704` · `oauth.rs:120` | 대기 중 취소 UI |
| O-16 | `Content-Length`를 32비트로 조회(4GiB 이상 오표기 — 현 상한에서는 무해) | `oauth.rs:1027-1043` | 64비트 |
| O-17 | 시간대 오프셋 무시(3사 모두 `Z` 반환이면 무해 — 추정) | `cloudfs.rs:1631-1633` | 유지 |
| O-18 | 열기용 임시 다운로드 폴더(`NexaDir\cloud`) 정리 코드 없음 | `win.rs:4182-4184` | 종료 시 정리 검토 |
| O-19 | 동기화 폴더 탐지가 UI 스레드에서 동기 실행(끊긴 네트워크 드라이브에서 지연 가능 — 추정) | `win.rs:541-550` · `cloud.rs:156-160` | 워커화 검토 |
| O-20 | `json_str`은 문서 전체 첫 일치 — 중첩 객체에 같은 키가 먼저 나오면 오인(현 요청은 `$select`/`fields`로 필드를 좁혀 회피) | `oauth.rs:629-633` · `cloudfs.rs:198` · `:223` | 필드 제한을 유지 |

## 7. 회귀 테스트 후보

자동화: **U** = 순수 단위 테스트(3-OS CI 가능) · **F** = 가짜 전송 계층/로컬 HTTP 서버로 자동화 가능 · **I** = OS 통합(해당 OS 러너 필요) · **M** = 실계정 수동(비밀값·브라우저 필요).

| # | 시나리오 | 대상 ID | 자동화 | 비고 |
|---|---|---|---|---|
| T-01 | 기존 단위 테스트 29건 이식: `cloud.rs` 2 · `oauth.rs` 13 · `cloudfs.rs` 9 · `secret.rs` 5 | 005·006·021·024·025·027·042~048·050·054·066 | U | `secret.rs`의 2건은 Windows 전용 → 보관 포트별로 재작성 |
| T-02 | vfs 센티널 4건 이식(`cloud_path_parts_and_build` · `cloud_display_leaf_and_parent` · `cloud_from_display_roundtrip` · `cloud_entries_without_lister_is_empty`) + 구분자 `/` 표기 케이스 추가 | 050~052 | U | `vfs:349-448` |
| T-03 | 설정 왕복: 링크/API 혼합 3건 · 라벨 파이프 치환 · 32개 상한 · client_id/secret 왕복 · **account 빈 API 연결 왕복**(O-03) | 018~020 | U | |
| T-04 | 리디렉션 URI 규칙: `localhost` 고정 · 서비스별 슬래시 · Dropbox 포트 집합 · 후보 전부 점유 시 `Listener` | 024 | U | 실제 포트 바인딩 — 병렬 실행 충돌 주의 |
| T-05 | 루프백 수신: 로컬 TCP로 `GET /?code=…&state=…` 전송 → code 추출 · state 불일치 · `error=` · 타임아웃(짧게) · IPv6 접속 · 응답 HTML | 029 | U | 네트워크 불요(루프백) |
| T-06 | 토큰 교환·refresh·계정 조회: 가짜 전송으로 요청 본문(파라미터·secret 조건부) 검증, 응답 변형(오류 본문·refresh 미포함) | 030~032 | F | 포트 분리 전제(§4-2 권고 1) |
| T-07 | 서비스별 요청 스냅숏: 목록·다운로드·업로드(단순/청크 경계 4MiB·140MiB·5MiB·8MiB)·삭제·이름변경·새 폴더·복사·이동의 **메서드·URL·헤더·본문** 고정 | 063~065 · 069 · 075~080 | F | 한글·공백·따옴표 경로 포함(L-09·L-10) |
| T-08 | Google ID 되짚기: ID 캐시 빈 상태에서 `/a/b/c` 목록 → 루트부터 3회 조회 | 064 | F | L-18 |
| T-09 | 목록 실패 캐시: 오류 응답 → `[!]` 1행 + 재요청 없음 → `invalidate` 후 재요청 | 055 · 059 | F | L-13 |
| T-10 | 중복 요청 방지: 같은 `(idx, inner)` 동시 2회 → 워커 1개 | 054 · 055 | U/F | |
| T-11 | 계정 간 복사: 폴더 포함 원본 → 대상 폴더 생성 순서(깊이 오름차순) · 세그먼트 반반 진행 · 임시 파일 정리 | 081 | F | L-19~21 |
| T-12 | 취소: 다운로드 청크 중·업로드 청크 경계·항목 사이에서 `CANCELLED` 전파, 부분 결과 보존 | 068 · 074 · 090 | F | |
| T-13 | 연결 추가/해제 후 인덱스: 링크·API 혼합에서 중간 항목 해제 → 남은 API 연결의 토큰이 **여전히 열리는가** | 015 · 035 · 036 | U | O-02 회귀 방지(dir3 신설) |
| T-14 | 재인증 흡수: 같은 종류·같은 계정 재연결 = 행 1개 유지 · 빈 계정 행 흡수 | 033 | U | 로직을 순수 함수로 분리 전제 |
| T-15 | 토큰 보관 왕복·덮어쓰기·삭제·손상 파일·임시 파일 미잔존 — OS별 보관 포트마다 | 035~037 | I | Windows DPAPI / 파일 봉투 / (선택) Keychain·Secret Service |
| T-16 | HTTP 전송 포트 적합성: 로컬 TLS 서버 상대로 메서드·헤더·빈 본문 POST·`Location`·상한 초과·진행·취소·비2xx 오류 메시지 — **OS별 구현마다 동일 스위트** | 039~041 | I | §4-2 계약 목록이 곧 케이스 목록 |
| T-17 | 동기화 폴더 탐지: 가짜 홈 디렉터리(`info.json`, `Library/CloudStorage/*`)로 후보 산출 · 실존 필터 · 중복 제거 | 001~004 · 007 | U(mac/Linux) · I(Windows 레지스트리) | 탐지 원천을 주입 가능하게 분리 전제 |
| T-18 | Cloud 메뉴 모델: 연결 0/1/32개 · 후보 0/n개에서 항목·명령 id·문구(API=연결 해제 / 링크=링크 해제) | 009 · 017 | U | 메뉴 구성 함수를 순수 모델로 |
| T-19 | 전송 분기표: (원본 로컬/클라우드 같은/다른 연결) × (대상 로컬/클라우드) × (복사/이동) → 선택되는 작업 종류 | 082 | U | O-10 동작 고정 또는 변경 결정 반영 |
| T-20 | 진행 창: 바이트 이동 작업에만 표시 · 임시 폴더 다운로드는 미표시 · 완료 카운트다운 | 067 · 073 · 088~091 | I(UI 하네스) | |
| T-21 | 실계정 종단: 3사 각각 연결 → 목록 → 한글 폴더 진입 → 업로드(소/대) → 다운로드 → 이름변경 → 삭제 → 연결 해제 | 전부 | M | 릴리스 전 점검표. 조직 MS 계정은 실패가 정상(CLOUD-095) |
| T-22 | 기동 복원: 클라우드 경로 탭을 가진 세션으로 시작 → 비활성 탭까지 채워짐 | 058 | I | L-17 |
