# toolbar — 도구 모음 임베드 아이콘(SVG · dir2 `assets/toolbar` 그대로)

`include_str!` 임베드 → nexa-gfx `svg` 서브셋 파서 → `svg::render_mask`(3-OS CPU · 요청 크기 즉석 래스터 · DPI 무관) →
nexa-ctl `ToolIcon::Mask`(테마 기준색 틴트 · hover/pressed = accent · 비활성 흐림). 등록 = [icons.rs](../../src/icons.rs) `EMBEDDED_SVG` 한 줄.

- 규격(dir2 사용자 확정 07-19): **32 viewBox · 콘텐츠 1..31 · stroke 2 · currentColor**. 지원 서브셋 초과 = 파싱 실패 → 글리프 폴백.
- `-dark.svg`(흰 잉크 하드코딩 변형)는 dir3에서 쓰지 않는다 — 마스크 + 테마색이 같은 결과. 원본 보존용으로만 둔다.
- 토글 켜짐 배경(accent 38 %)은 `Toolbar::set_item_checked`가 그린다(dir2 chrome.rs 블렌드와 같은 규칙).

| 파일 | 명령 |
| --- | --- |
| `panel-toggle.svg` | `view.panel_toggle` |
| `dock.svg` | `view.dock` |
| `always-on-top.svg` | `view.always_on_top` |
| `info-toggle.svg` | `view.info_toggle` |
| `colsync.svg` | `view.col_width_sync` |
| `view-tree.svg` · `view-flat.svg` · `view-tiles.svg` | `view.mode_tree` · `view.mode_flat` · `view.mode_tiles` |
| `refresh.svg` | `view.refresh` |
| `settings.svg` | `file.prefs` |
| `hidden.svg` · `dotfiles.svg` · `folders-first.svg` | `view.hidden` · `view.dot` · `view.folders_first` |
| `case-sensitive.svg` | `view.case_sensitive`(dir3 신규 10-03 — "Aa" · 대소문자 구분 정렬 토글) |
| `popout.svg` | 도크 ↗(등록만 — nexa-explorer InfoDock은 글리프) |
