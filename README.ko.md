# Nexa Dir 3 — 크로스플랫폼 네이티브 파일 탐색기 (올 러스트)

> 한글 안내. 영어 원본(패키지 동봉) = [README.md](README.md).

> Cross-platform (Windows · macOS · Linux), single-binary, ultra-lightweight native file explorer — the cross-platform successor of [nexa-dir2](https://github.com/SosomLab/nexa-dir2).

**Nexa Dir 3**는 Windows 전용이던 [nexa-dir2](https://github.com/SosomLab/nexa-dir2)(`0.22.0`)의 **기능·화면 배치·자원을 그대로** 세 OS에서 재현하는 프로젝트입니다. 창·입력은 `winit + softbuffer`, 화면은 자체 CPU 래스터([nexa-ui](https://github.com/SosomLab/nexa-ui)), 설정 구조는 [nexa-sql](https://github.com/SosomLab/nexa-sql) 방식, 라이선스는 [nexa-license](https://github.com/SosomLab/nexa-license)를 씁니다. OS 기본 컨트롤은 쓰지 않습니다 — 세 OS에서 같은 화면.

## 설계 원칙

1. **기능·배치 패리티** — nexa-dir2의 기능 목록([docs/port](docs/port/00-index.md) 이식 원장)이 체크리스트. 차이는 "의도된 차이"로만.
2. **컨트롤은 전부 nexa-ui** — 없으면 nexa-ui에 추가해 공유(nexa-sql 등 계열 앱과 같은 부품).
3. **OS 분기는 한 층(`platform/`)** — 셸 컨텍스트 메뉴 · 휴지통 · 클립보드 · DnD · 폴더 감시 · 터미널 셸(Windows `pwsh` / macOS·Linux `$SHELL`).
4. **회귀가 바로 보이게** — 3-OS CI · 헤드리스 앱 시나리오 · `nexa-dir --selfcheck`(자가 점검) · 검증 매트릭스.

## 현재 상태

- 단계: **M7 배포 · M8 교차 검증**(2026-10-03 M0 착수 → 10-05 v0.23.0 공개 → v0.23.x 전 채널 게시). 상세 → [docs/STATUS.md](docs/STATUS.md) · 로드맵 → [docs/MILESTONES.md](docs/MILESTONES.md).

## 빌드 · 점검

형제 저장소를 나란히 둡니다: `../nexa-ui` · `../nexa-license`(path 의존).

```bash
cargo build --workspace
cargo run -p nexa-dir -- --smoke        # 창 없이 기동 점검
cargo run -p nexa-dir -- --selfcheck    # 자가 점검(doctor) — 설치본 Windows 콘솔에서는 `ndir --selfcheck`(콘솔 보조 exe · 출력 순서/캡처 정상)
scripts/check-3os.sh                    # push 전 3-OS 교차 검사
```

절차 SSOT = [docs/18](docs/18-build-and-test.md).

## 문서 — [문서 홈](docs/README.md)

바로가기: [이식 메모리 CLAUDE.md](CLAUDE.md) · [결정 기록](docs/10-decision-record.md) · [개발 기준](docs/15-dev-methodology.md) · [문서·git 규약](docs/16-doc-git-conventions.md) · [이식 원장](docs/port/00-index.md)

## 프로젝트 정보 / 라이선스

| 항목 | 내용 |
| --- | --- |
| 조직 | **SosomLab** — <https://sosomlab.com> |
| 원본 | <https://github.com/SosomLab/nexa-dir2> (기능 원천) · <https://github.com/SosomLab/nexa-dir> (원조) |
| 개발자 | Sangyong Bae — kiros33@gmail.com |

**PolyForm Noncommercial 1.0.0** ([LICENSE.md](LICENSE.md) · 한글 [LICENSE.ko.md](LICENSE.ko.md)) — 개인·비상업 무료, 상업 사용은 유료 라이선스(문의 kiros33@sosomlab.com).
