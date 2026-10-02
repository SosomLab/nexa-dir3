# MILESTONES — 기능·목표 관점 현황

> 목표순. ✅ 완료 / 🚧 진행 / 📐 설계 / ☐ 미착수. 상세는 journal, 할 일은 [TODO](TODO.md).

| M | 목표 | 완료 기준 | 상태 |
| --- | --- | --- | --- |
| M0 | **골격** — 규칙 문서(CLAUDE·10·15·16·18·01) · 워크스페이스 · CI 3-OS · `--smoke`/`--selfcheck` 뼈대 · 이식 원장 색인 · 복원 태그 | CI green · 문서 4층 존재 · `cargo run -- --smoke` = 0 | ✅ 10-03(T-05~08 잔여는 M3 이후) |
| M1 | **기반 크레이트 이식** — `ndir-core/vfs/tree/ops/term`(dir2 테스트 그대로) · `ndir-i18n`(`.lang` + 키 검사) · `ndir-settings`(엔진 + dir2 키 표 + dir2 가져오기 + 명령 표·키맵) · `ndir-license` | dir2 테스트 전수 green(3-OS) · 레지스트리 무결성 시험 · 라이선스 8건 | ✅ 10-03 (테스트 196) |
| M2 | **nexa-ui 보강** — InputEvent 확장 · MenuBar(체크·단축키 열) · Toolbar(checked) · TabBar(아이콘·툴팁) · StatusBar · PathBar · `nexa-grid` VirtualRows(dir2 rows/columns/typeahead) · InfoDock · Tooltip · Dialog/MessageBox/Progress · `RecordCtx` 공용 | nexa-ui 테스트 green · nexa-sql 빌드 유지 · 각 컨트롤 T2 시험 | ☐ |
| M3 | **앱 골격** — winit 호스트(nexa-sql 복사) · AppCore/Shell · 명령 표 · 메뉴바·툴바·런처·탭·경로바·듀얼 패널 파일 목록·스플리터·상태바 · 설정 창·단축키 창·키맵 · 세션 복원 · 테마/언어 전환 | dir2 레이아웃 수치와 동일한 `layout.dump` 골든 · T3 시나리오 · 실기 캡처 | 🚧 10-03(T-40 ✅ · T-42·43 부분) |
| M4 | **플랫폼 층** — 포트 + Windows/macOS/Linux 구현(셸 메뉴 · 휴지통 · 파일 클립보드 · DnD · 감시 · 열기 · 셸 탐지 · PTY) + Fake · `--selfcheck` 실제 항목 | 3-OS selfcheck PASS(CI = `--ci`) · MC/DC 시험 | ☐ |
| M5 | **도크·터미널·미리보기·플러그인** — 도크 정보/미리보기/터미널 · 터미널 테마 15종·복사 서식 · F3 창 · 압축 미리보기 · WASM 런타임(dir2 ABI) · 동봉 플러그인 · 설정 플러그인 페이지 · 3-OS 빌드 스크립트 | 동봉 `.wasm` 2종 무수정 로드 · dir2 wasm 테스트 11종 · PTY 왕복 | ☐ |
| M6 | **파일 작업·일괄 이름 변경·실행 취소·클라우드** — 전송(진행·취소·충돌·스테이징) · 삭제(휴지통/영구) · 새로 만들기 · 인라인 이름 바꾸기 · 일괄 이름 변경 창 · 히스토리 · (클라우드 OAuth = Q-6 결정 뒤) | ops 시험 + T3 시나리오 · 실기 QA | ☐ |
| M7 | **라이선스 창·About·배포** — 라이선스 창 · Help 메뉴 · nexa-license 발급기 보강(mail_text·id-prefix·E2E) · 3-OS 패키징 · release.yml · 채널 결정 | 3-OS 산출물 · 설치 스모크 · `0.23.0` 태그(승인) | ☐ |
| M8 | **교차 검증·회귀 완성** — 검증 매트릭스 빈칸 0 · dir2 대조 실기 QA · 성능 기준선 · 자가 점검 창 | 매트릭스 전수 · 성능 수치 journal | ☐ |
