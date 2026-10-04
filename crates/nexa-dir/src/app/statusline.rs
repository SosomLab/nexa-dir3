//! App — 상태줄 구성(docs/22 NEW-003 · DR-23) + 탭 상태바 메뉴(NEW-004 1차).
//!
//! 창 아래 상태줄의 오른쪽 칸 = `statusbar.layout`의 순서/표시(순서 편집 창 · 기본 = 탭 · CPU · 메모리 · 디스크 · 네트워크 · 라이선스): 탭 n/m · **시스템(PC 전체)**의
//! CPU % · 메모리 · 디스크 읽기/쓰기 속도 · 네트워크 다운로드/업로드 속도(사용자 10-04 "시스템 상태값 · 네트워크 추가") ·
//! 라이선스(클릭 = 라이선스 창). 부하는 `statusbar.load_interval_ms`마다 [`platform::sysload::sample`]로 조회한다
//! (부하 칸이 하나도 없으면 조회도 · 깨우기도 없다).

use crate::app::ctxmenu::CtxKind;
use crate::platform::sysload::{self, SysLoad};
use crate::*;
use nexa_ctl::{StatusPart, StatusSeg};

/// 칸 하나 = `(블록 id, 그 안에 보일 항목 id들)`.
pub(crate) type StatusBlock = (&'static str, Vec<&'static str>);

/// 설정값(`tab:1|cpu:1|mem:1[app:1,system:1]|…`) → 표시할 칸과 그 안의 항목(순서대로 · 숨긴 것 제외 · 빈 값 = 전부 ·
/// 항목이 있는 칸에서 항목을 전부 끄면 그 칸도 빠진다 · 순수).
pub(crate) fn status_items_of(value: &str) -> Vec<StatusBlock> {
    let defs = crate::order::STATUSBAR_BLOCKS;
    let mut blocks = crate::order::parse_order_with(defs, value);
    // 저장값에 없던 새 칸은 맨 뒤에 보충된다 — 앱 메모리 칸이 생기기 전에 저장한 값이면 그 칸을 **라이선스 앞**으로 옮긴다
    // (맨 오른쪽 = 앱 메모리 · 라이선스 순 — 사용자 10-04).
    if !value.is_empty() && !value.contains("appmem") {
        if let (Some(a), Some(l)) = (
            blocks.iter().position(|b| b.0 == "appmem"),
            blocks.iter().position(|b| b.0 == "license"),
        ) {
            if a > l {
                let app = blocks.remove(a);
                blocks.insert(l, app);
            }
        }
    }
    blocks
        .into_iter()
        .filter(|(_, vis, _)| *vis)
        .filter_map(|(b, _, items)| {
            let (block, kids) = defs.iter().copied().find(|(d, _)| *d == b)?;
            let shown: Vec<&'static str> = items
                .iter()
                .filter(|(_, vis)| *vis)
                .filter_map(|(k, _)| kids.iter().copied().find(|d| *d == k))
                .collect();
            (kids.is_empty() || !shown.is_empty()).then_some((block, shown))
        })
        .collect()
}

/// 바이트/초 → 짧은 속도 글(`0 B/s` · `1.2 MB/s`).
pub(crate) fn fmt_rate(bps: u64) -> String {
    format!("{}/s", filelist::format_size(bps))
}

/// 두 줄로 쌓는 줄의 글리프 크기(논리 px) — **줄 위아래 · 줄 사이 여백이 최소가 되는 가장 큰 크기**(사용자 10-04):
/// 상태줄 높이 22에서 위 선 1을 뺀 21을 둘로 나눈 띠(10.5)에 글의 잉크(화살표 꼭대기 −8 ~ `/` 바닥 +2 = 10)가 꼭 맞는다
/// (실측 — 9.5는 잉크 7이라 줄마다 3.5씩 남았고 · 14는 11이라 위아래 줄이 겹친다).
pub(crate) const STATUS_ROW_PX: f32 = 12.0;

/// 크기 글의 폭 견본 — **기본 너비 확보용**(사용자 10-04 "완전 고정이 아니라 되도록 변하지 않게"): 흔한 값(세 자리 + 소수
/// 한 자리 · KB/MB/GB)의 폭을 미리 잡아 두고, 그보다 넓은 값이 오면 그때만 칸이 늘어난다. `suffix` = `/s` 등.
fn size_hints(prefix: &str, suffix: &str) -> Vec<String> {
    ["KB", "MB", "GB"]
        .iter()
        .map(|u| format!("{prefix}999.9 {u}{suffix}"))
        .collect()
}

impl App {
    /// 상태줄 오른쪽 칸(지금 값으로) — 표시는 약어(C · M · D · N) + 값 · 값 조각은 폭 견본으로 폭이 고정된다.
    /// 디스크 = ↑ 읽기 · ↓ 쓰기 / 네트워크 = ↑ 업로드 · ↓ 다운로드 — ↑ = 빨강(`danger`) · ↓ = 파랑(`accent`).
    /// 모든 칸은 누를 수 있다(상세 팝업 · 앱 메모리 = 메모리 창 · 라이선스 = 라이선스 창).
    pub(crate) fn status_segments(&self) -> Vec<StatusSeg> {
        let dash = || "–".to_string();
        let load: Option<SysLoad> = self.load;
        let (up_c, down_c) = (self.theme.danger, self.theme.accent);
        let part = StatusPart::new;
        // 디스크 · 네트워크 = 약어 옆에 **두 줄로 쌓는다**(위 ↑ · 아래 ↓ — 사용자 10-04): 한 줄에 `↑ 24.5 MB/s` ·
        // 글꼴은 상태줄 높이에 두 줄이 들어가는 크기로(값 · 단위 같은 크기 — 따로 줄이던 −1/−2는 취소).
        let row_delta = self.status_row_font_delta();
        let row = |up: bool, v: Option<u64>| {
            let a = if up { "↑ " } else { "↓ " };
            part(format!("{a}{}", v.map_or_else(dash, fmt_rate)))
                .color(if up { up_c } else { down_c })
                .hints(size_hints(a, "/s"))
                .font_delta(row_delta)
        };
        status_items_of(self.settings.get("statusbar.layout").unwrap_or(""))
            .into_iter()
            .map(|(id, kids)| match id {
                "tab" => {
                    let p = &self.panels[self.active];
                    StatusSeg::with_parts(
                        id,
                        vec![part(trf(
                            "status.tab",
                            &[
                                &(p.active_index() + 1).to_string(),
                                &p.tab_count().to_string(),
                            ],
                        ))],
                    )
                }
                "cpu" => StatusSeg::with_parts(
                    id,
                    vec![
                        part(tr("status.abbr.cpu")),
                        part(load.map_or_else(dash, |l| format!("{:.1}%", l.cpu_pct)))
                            .hints(vec!["100.0%".into()]),
                    ],
                ),
                "mem" => StatusSeg::with_parts(
                    id,
                    vec![
                        part(tr("status.abbr.mem")),
                        part(load.map_or_else(dash, |l| filelist::format_size(l.mem_used)))
                            .hints(size_hints("", "")),
                    ],
                ),
                "disk" => {
                    let d = load.and_then(|l| l.disk_bps);
                    let rows = kids
                        .iter()
                        .map(|k| match *k {
                            "read" => row(true, d.map(|v| v.0)),
                            _ => row(false, d.map(|v| v.1)),
                        })
                        .collect();
                    StatusSeg::with_parts(id, vec![part(tr("status.abbr.disk"))]).rows(rows)
                }
                "net" => {
                    let n = load.and_then(|l| l.net_bps);
                    let rows = kids
                        .iter()
                        .map(|k| match *k {
                            "upload" => row(true, n.map(|v| v.1)),
                            _ => row(false, n.map(|v| v.0)),
                        })
                        .collect();
                    StatusSeg::with_parts(id, vec![part(tr("status.abbr.net"))]).rows(rows)
                }
                // 이 프로그램의 메모리(누르면 메모리 창).
                "appmem" => StatusSeg::with_parts(
                    id,
                    vec![part(
                        load.and_then(|l| l.mem_app)
                            .map_or_else(dash, filelist::format_size),
                    )
                    .hints(size_hints("", ""))],
                ),
                _ => StatusSeg::new(id, self.license_badge()),
            })
            .collect()
    }

    /// 두 줄로 쌓는 칸(디스크 · 네트워크)의 줄 글꼴 크기 증분(논리 px): 상태줄 높이(22)에 두 줄이 들어가는 글리프 크기
    /// ([`STATUS_ROW_PX`])와 상태줄 글꼴의 차이 — 상태줄 글꼴을 키워도 두 줄은 같은 크기로 남는다.
    pub(crate) fn status_row_font_delta(&self) -> f32 {
        STATUS_ROW_PX
            - self
                .ui_font
                .em_to_px(self.settings.font_px("statusbar.font_size"))
    }

    /// 부하 칸이 하나라도 있는가.
    fn status_wants_load(&self) -> bool {
        status_items_of(self.settings.get("statusbar.layout").unwrap_or(""))
            .iter()
            .any(|(k, _)| matches!(*k, "cpu" | "mem" | "disk" | "net" | "appmem"))
    }

    /// 유휴 틱 — 주기가 됐으면 부하를 조회해 칸을 갱신하고 다음 조회 시각을 돌려준다(부하 칸이 없으면 `None` = 깨우지 않음).
    /// 상세 팝업이 떠 있으면 같은 주기로 그 내용도 갱신한다.
    pub(crate) fn status_load_tick(&mut self, now: Instant) -> Option<Instant> {
        if !self.status_wants_load() {
            return None;
        }
        if now >= self.load_next {
            let every = self
                .settings
                .int("statusbar.load_interval_ms")
                .clamp(500, 60_000) as u64;
            self.load_next = now + Duration::from_millis(every);
            if let Some(cur) = sysload::sample() {
                let cores = std::thread::available_parallelism().map_or(1, usize::from);
                self.load = Some(match self.load_prev {
                    Some((at, prev)) => sysload::load_between(prev, cur, now - at, cores),
                    None => sysload::load_between(cur, cur, Duration::ZERO, cores),
                });
                self.load_prev = Some((now, cur));
                let mut inv = Invalidations::default();
                let segs = self.status_segments();
                if self.statusbar.set_segments(segs, &mut inv) {
                    self.redraw();
                }
                self.refresh_status_popup();
                if self.mem_win.is_open() {
                    self.mem_win.redraw(); // 메모리 창도 같은 주기로
                }
            }
        }
        Some(self.load_next)
    }

    /// 상태줄 칸 클릭 — 앱 메모리 = 메모리 창 · 라이선스 = 라이선스 창 · 그 밖 = 상세 팝업(우클릭은 호스트가 순서 편집 창).
    pub(crate) fn status_click(&mut self, id: &str, right: bool) {
        if right {
            return;
        }
        // 이 칸의 상세 팝업이 떠 있던 채로 다시 눌렀다 = 닫기만(토글).
        if self.status_popup_was.take().as_deref() == Some(id) {
            self.status_popup = None;
            return;
        }
        match id {
            "license" => self.command("help.license"),
            // 메모리 창(모덜리스 · 열려 있으면 닫기 토글 — 라이선스 창과 같은 규칙).
            "appmem" => {
                if self.mem_win.is_open() {
                    self.mem_win.close();
                } else {
                    self.open_memory = true;
                }
            }
            _ => self.open_status_popup(id),
        }
    }

    /// 상세 팝업 항목(지금 값) — 칸마다 그냥 볼 때보다 자세히.
    pub(crate) fn status_popup_items(&self, id: &str) -> Vec<CtxItem> {
        let dash = || "–".to_string();
        let info = |text: String| CtxItem::item("aux.info", text);
        let size = filelist::format_size;
        let load = self.load;
        let totals = self.load_prev.map(|(_, s)| s);
        let mut items: Vec<CtxItem> = match id {
            "cpu" => vec![
                info(trf(
                    "sb.cpu.total",
                    &[&load.map_or_else(dash, |l| format!("{:.1}", l.cpu_pct))],
                )),
                info(trf(
                    "sb.cpu.app",
                    &[&load
                        .and_then(|l| l.app_cpu_pct)
                        .map_or_else(dash, |p| format!("{p:.1}"))],
                )),
                info(trf(
                    "sb.cpu.cores",
                    &[&std::thread::available_parallelism()
                        .map_or(1, usize::from)
                        .to_string()],
                )),
            ],
            "mem" => vec![
                info(trf(
                    "sb.mem.used",
                    &[
                        &load.map_or_else(dash, |l| size(l.mem_used)),
                        &load.map_or_else(dash, |l| format!("{:.0}", l.mem_pct())),
                    ],
                )),
                info(trf(
                    "sb.mem.free",
                    &[&load.map_or_else(dash, |l| size(l.mem_total.saturating_sub(l.mem_used)))],
                )),
                info(trf(
                    "sb.mem.total",
                    &[&load.map_or_else(dash, |l| size(l.mem_total))],
                )),
                info(trf(
                    "sb.mem.app",
                    &[&load.and_then(|l| l.mem_app).map_or_else(dash, size)],
                )),
            ],
            "disk" => {
                let d = load.and_then(|l| l.disk_bps);
                let t = totals.and_then(|s| s.disk);
                vec![
                    info(trf(
                        "sb.disk.read",
                        &[&d.map_or_else(dash, |v| fmt_rate(v.0))],
                    )),
                    info(trf(
                        "sb.disk.write",
                        &[&d.map_or_else(dash, |v| fmt_rate(v.1))],
                    )),
                    info(trf(
                        "sb.disk.readTotal",
                        &[&t.map_or_else(dash, |v| size(v.0))],
                    )),
                    info(trf(
                        "sb.disk.writeTotal",
                        &[&t.map_or_else(dash, |v| size(v.1))],
                    )),
                ]
            }
            "net" => {
                let n = load.and_then(|l| l.net_bps);
                let t = totals.and_then(|s| s.net);
                vec![
                    info(trf("sb.net.up", &[&n.map_or_else(dash, |v| fmt_rate(v.1))])),
                    info(trf(
                        "sb.net.down",
                        &[&n.map_or_else(dash, |v| fmt_rate(v.0))],
                    )),
                    info(trf(
                        "sb.net.upTotal",
                        &[&t.map_or_else(dash, |v| size(v.1))],
                    )),
                    info(trf(
                        "sb.net.downTotal",
                        &[&t.map_or_else(dash, |v| size(v.0))],
                    )),
                ]
            }
            _ => (0..if self.dual { 2 } else { 1 })
                .map(|i| {
                    let p = &self.panels[if self.dual { i } else { self.active }];
                    info(trf(
                        if i == 0 && self.dual || !self.dual && self.active == 0 {
                            "sb.tab.left"
                        } else {
                            "sb.tab.right"
                        },
                        &[
                            &(p.active_index() + 1).to_string(),
                            &p.tab_count().to_string(),
                            &p.root_path().display().to_string(),
                        ],
                    ))
                })
                .collect(),
        };
        items.push(CtxItem::Separator);
        items.push(CtxItem::item("aux.sb.edit", tr("sb.edit")));
        items
    }

    /// 메모리 창의 보기(지금 값): 이 프로그램의 메모리(운영체제 값) · 영역별 추정(목록 · 창 표면) + 기타(차이) · 시스템.
    pub(crate) fn mem_view(&self) -> crate::mem_win::MemView {
        let cur = sysload::sample();
        let app = cur.and_then(|s| s.mem_app);
        let lists: u64 = self.panels.iter().map(Panel::mem_estimate).sum();
        // 창 표면 = 메인 창의 프레임 버퍼(가로 × 세로 × 4바이트).
        let surfaces =
            u64::from(self.viewport.0.max(0) as u32) * u64::from(self.viewport.1.max(0) as u32) * 4;
        crate::mem_win::MemView {
            app,
            rows: crate::mem_win::rows_with_other(
                vec![
                    (tr("mem.cat.lists"), lists),
                    (
                        tr("mem.cat.icons"),
                        app::row_icons::cache_bytes() + app::launcher_icons::cache_bytes(),
                    ),
                    (
                        tr("mem.cat.terminal"),
                        self.terms.iter().map(TermView::mem_estimate).sum(),
                    ),
                    (tr("mem.cat.surfaces"), surfaces),
                ],
                app,
                tr("mem.cat.other"),
            ),
            system: cur.map(|s| (s.mem_used, s.mem_total)),
        }
    }

    /// 칸의 상세 팝업을 그 칸 위에 연다(값은 조회 주기마다 갱신 — [`Self::refresh_status_popup`]).
    pub(crate) fn open_status_popup(&mut self, id: &str) {
        let Some(r) = self.statusbar.seg_rect(id) else {
            return;
        };
        let items = self.status_popup_items(id);
        self.ctx_anchor_next = Some((r.x, r.y));
        self.open_ctx(CtxKind::Aux(self.active), items);
        self.status_popup = Some(id.to_string());
    }

    /// 떠 있는 상세 팝업의 내용을 지금 값으로(닫혔으면 기억을 지운다).
    fn refresh_status_popup(&mut self) {
        let Some(id) = self.status_popup.clone() else {
            return;
        };
        if !self.tab_menu.is_open() || !matches!(self.ctx_kind, Some(CtxKind::Aux(_))) {
            self.status_popup = None;
            return;
        }
        let items = self.status_popup_items(&id);
        self.reopen_ctx(items);
    }

    /// Git 상태를 패널에 맞춘다(탭 상태바 길목): 패널이 보는 저장소의 요약이 있으면 칸에 덧붙이고 · 없고 조회 중도 아니면
    /// 워커를 돌린다(`git status --porcelain=v2 --branch` — UI는 기다리지 않는다 · git이 없으면 브랜치 이름만 남는다).
    pub(crate) fn git_sync(&mut self, inv: &mut Invalidations) {
        for i in 0..self.panels.len() {
            let Some((repo, _)) = self.panels[i].git_info() else {
                self.panels[i].set_git_extra(String::new());
                continue;
            };
            let extra = self
                .git_detail
                .get(&repo)
                .map(dirinfo::GitDetail::short)
                .unwrap_or_default();
            self.panels[i].set_git_extra(extra);
            self.panels[i].sync_status(inv);
            if !self.git_detail.contains_key(&repo) && self.git_enabled {
                self.git_request(repo);
            }
        }
    }

    /// 저장소 상태 조회를 워커에 맡긴다(같은 저장소가 조회 중이면 건너뜀).
    pub(crate) fn git_request(&mut self, repo: PathBuf) {
        if !self.git_busy.insert(repo.clone()) {
            return;
        }
        let tx = self.git_tx.clone();
        std::thread::spawn(move || {
            let out = platform::quiet_command("git")
                .arg("-C")
                .arg(&repo)
                .args(["status", "--porcelain=v2", "--branch"])
                .stdin(std::process::Stdio::null())
                .output();
            let detail = out
                .ok()
                .filter(|o| o.status.success())
                .map(|o| dirinfo::parse_porcelain_v2(&String::from_utf8_lossy(&o.stdout)));
            let _ = tx.send((repo, detail));
        });
    }

    /// 유휴 틱 — 조회 결과를 받아 칸 · 떠 있는 팝업을 갱신한다. 조회 중이면 곧 다시 깨운다.
    pub(crate) fn git_tick(&mut self, now: Instant) -> Option<Instant> {
        let mut got = false;
        while let Ok((repo, detail)) = self.git_rx.try_recv() {
            self.git_busy.remove(&repo);
            // 실패(git 없음 · 저장소 아님)도 기억한다 — 같은 저장소를 되풀이해 조회하지 않게(빈 요약).
            self.git_detail.insert(repo, detail.unwrap_or_default());
            got = true;
        }
        if got {
            let mut inv = Invalidations::default();
            self.git_sync(&mut inv);
            self.redraw();
        }
        (!self.git_busy.is_empty()).then(|| now + Duration::from_millis(150))
    }

    /// 탭 상태바 칸 클릭 → 상세 메뉴(좌 · 우클릭 같은 메뉴 — 1차): 폴더 = 항목 수 상세 · Git = 브랜치 · 복사 · 새로 고침.
    pub(crate) fn open_tab_status_menu(&mut self, panel: usize, seg: &str) {
        let p = &self.panels[panel];
        let root = p.root_path();
        let info = |text: String| CtxItem::maybe("aux.info", text, false);
        let mut items: Vec<CtxItem> = Vec::new();
        let mut git_repo: Option<PathBuf> = None;
        match seg {
            panel::SEG_GIT => {
                let Some((repo, branch)) = p.git_info() else {
                    return;
                };
                items.push(info(trf("tabstatus.git.branch", &[&branch])));
                items.push(info(trf(
                    "tabstatus.git.repo",
                    &[&repo.display().to_string()],
                )));
                if let Some(d) = self.git_detail.get(&repo) {
                    if let Some(up) = &d.upstream {
                        items.push(info(trf(
                            "tabstatus.git.upstream",
                            &[up, &d.ahead.to_string(), &d.behind.to_string()],
                        )));
                    }
                    items.push(info(if d.is_clean() {
                        tr("tabstatus.git.clean")
                    } else {
                        trf(
                            "tabstatus.git.changes",
                            &[
                                &d.staged.to_string(),
                                &d.changed.to_string(),
                                &d.untracked.to_string(),
                                &d.conflicts.to_string(),
                            ],
                        )
                    }));
                }
                items.push(CtxItem::Separator);
                // 메뉴를 열 때마다 최신 상태를 다시 조회한다(도착하면 칸이 갱신된다 · 메뉴는 지금 아는 값으로).
                git_repo = Some(repo);
                items.push(CtxItem::item("aux.git.copy", tr("tabstatus.git.copy")));
                items.push(CtxItem::item("aux.refresh", tr("menu.view.refresh")));
            }
            _ => {
                let shown = p.rows().source().len();
                items.push(info(trf("tabstatus.shown", &[&shown.to_string()])));
                if !ndir_vfs::is_virtual_root(&root) {
                    if let Some((dirs, files)) = dirinfo::count_entries(&root) {
                        items.push(info(trf("tabstatus.total", &[&(dirs + files).to_string()])));
                        items.push(info(trf("tabstatus.folders", &[&dirs.to_string()])));
                        items.push(info(trf("tabstatus.files", &[&files.to_string()])));
                    }
                }
                items.push(CtxItem::Separator);
                items.push(CtxItem::item("aux.refresh", tr("menu.view.refresh")));
            }
        }
        self.open_ctx(CtxKind::Aux(panel), items);
        if let Some(repo) = git_repo.filter(|_| self.git_enabled) {
            self.git_request(repo);
        }
    }

    /// 툴바 우클릭 메뉴(dir2 `show_bar_popup`): 도구 모음 순서… · 설정….
    pub(crate) fn open_toolbar_menu(&mut self) {
        let items = vec![
            CtxItem::item("aux.tb.order", format!("{}…", tr("pref.toolbarOrder"))),
            CtxItem::Separator,
            CtxItem::item("aux.prefs", tr("menu.file.prefs")),
        ];
        self.open_ctx(CtxKind::Aux(self.active), items);
    }

    /// 상태줄 우클릭 메뉴(툴바 우클릭과 같은 모양): 상태바 편집… · 설정….
    pub(crate) fn open_statusbar_menu(&mut self) {
        let items = vec![
            CtxItem::item("aux.sb.edit", tr("sb.edit")),
            CtxItem::Separator,
            CtxItem::item("aux.prefs", tr("menu.file.prefs")),
        ];
        self.open_ctx(CtxKind::Aux(self.active), items);
    }

    /// 커서 아래 빠른 실행 항목의 자리(`launcher_items` 인덱스 · 빈 곳 · 구분선 = `None`).
    fn launcher_item_at(&self, x: i32, y: i32) -> Option<usize> {
        (0..self.launcher_items.len()).find(|i| {
            self.launcherbar
                .item_rect(&format!("launch:{i}"))
                .is_some_and(|r| r.contains(Point { x, y }))
        })
    }

    /// 빠른 실행 우클릭 메뉴: (항목 위) 편집… · 제거 / 항목 추가… · 구분선 추가 / 빠른 실행 숨기기 · 설정….
    pub(crate) fn open_launcher_menu(&mut self) {
        let (x, y) = self.cursor;
        let mut items: Vec<CtxItem> = Vec::new();
        if let Some(i) = self.launcher_item_at(x, y) {
            items.push(CtxItem::item(
                format!("aux.launch.edit:{i}"),
                trf("launcher.menu.edit", &[&self.launcher_items[i].label]),
            ));
            items.push(CtxItem::item(
                format!("aux.launch.remove:{i}"),
                tr("launcher.menu.remove"),
            ));
            items.push(CtxItem::Separator);
        }
        items.push(CtxItem::item("aux.launch.add", tr("launcher.menu.add")));
        items.push(CtxItem::item(
            "aux.launch.addsep",
            tr("launcher.menu.addSep"),
        ));
        items.push(CtxItem::Separator);
        items.push(CtxItem::item("aux.launch.hide", tr("launcher.menu.hide")));
        items.push(CtxItem::item("aux.prefs", tr("menu.file.prefs")));
        self.open_ctx(CtxKind::Aux(self.active), items);
    }

    /// 빠른 실행 항목 목록을 설정에 쓰고 바를 다시 만든다.
    fn save_launcher_items(&mut self, items: &[launcher::LauncherItem]) {
        let _ = self
            .settings
            .set("launcher.items", &launcher::encode_items(items));
        let _ = self.settings.save();
        self.after_setting_changed("launcher.items");
    }

    /// 항목 입력 대화상자(추가 = `None` · 편집 = `Some(자리)`) — 한 줄 `라벨|실행 파일|인자`.
    fn ask_launcher_item(&mut self, at: Option<usize>) {
        let initial = at
            .and_then(|i| self.launcher_items.get(i))
            .map(launcher::LauncherItem::encode)
            .unwrap_or_default();
        let mut spec = crate::dlg_win::DlgSpec::confirm(
            tr("launcher.dlg.title"),
            tr("launcher.dlg.text"),
            tr("dlg.ok"),
        );
        spec.input = Some(crate::dlg_win::DlgInput {
            label: tr("launcher.dlg.input"),
            masked: false,
            initial,
        });
        let _ = self.ask(spec, crate::app::dialogs::DlgReply::LauncherItem(at));
    }

    /// 대화상자 확인 — 입력을 항목으로 풀어 추가/교체한다(형식이 틀리면 안내만).
    pub(crate) fn launcher_item_entered(&mut self, at: Option<usize>, text: &str) {
        let Some(item) = launcher::LauncherItem::parse(text) else {
            self.toasts.push(
                toast::ToastKind::Warn,
                tr("launcher.dlg.title"),
                tr("launcher.dlg.invalid"),
            );
            return;
        };
        let mut items = self.launcher_items.clone();
        match at.filter(|i| *i < items.len()) {
            Some(i) => items[i] = item,
            None => items.push(item),
        }
        self.save_launcher_items(&items);
    }

    /// 보조 메뉴(탭 상태바 · 툴바 · 런처) 항목 실행.
    pub(crate) fn aux_menu_action(&mut self, panel: usize, id: &str) {
        if let Some(i) = id
            .strip_prefix("aux.launch.edit:")
            .and_then(|n| n.parse::<usize>().ok())
        {
            return self.ask_launcher_item(Some(i));
        }
        if let Some(i) = id
            .strip_prefix("aux.launch.remove:")
            .and_then(|n| n.parse::<usize>().ok())
        {
            let mut items = self.launcher_items.clone();
            if i < items.len() {
                items.remove(i);
                self.save_launcher_items(&items);
            }
            return;
        }
        match id {
            "aux.tb.order" => self.open_order_editor("toolbar.layout"),
            "aux.prefs" => self.command("file.prefs"),
            "aux.launch.add" => self.ask_launcher_item(None),
            "aux.launch.addsep" => {
                let mut items = self.launcher_items.clone();
                items.push(launcher::LauncherItem::separator());
                self.save_launcher_items(&items);
            }
            "aux.launch.hide" => self.command("view.launcher"),
            "aux.git.copy" => {
                if let Some((_, branch)) = self.panels[panel].git_info() {
                    let _ = clipboard::write_text(&branch);
                }
            }
            "aux.sb.edit" => self.open_order_editor("statusbar.layout"),
            "aux.refresh" => {
                // 저장소 상태도 다시 조회한다.
                if let Some((repo, _)) = self.panels[panel].git_info() {
                    self.git_detail.remove(&repo);
                }
                self.panels[panel].invalidate_dir_info();
                if panel != self.active {
                    self.set_active(panel);
                }
                self.command("view.refresh");
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_items_parse_order_and_drop_unknown() {
        let ids = |v: &str| -> Vec<String> {
            status_items_of(v)
                .into_iter()
                .map(|(b, k)| {
                    if k.is_empty() {
                        b.to_string()
                    } else {
                        format!("{b}[{}]", k.join(","))
                    }
                })
                .collect()
        };
        assert_eq!(
            ids(""),
            [
                "tab",
                "cpu",
                "mem",
                "disk[read,write]",
                "net[upload,download]",
                "appmem",
                "license"
            ],
            "빈 값 = 기본(전부)"
        );
        assert_eq!(
            ids("license:1|net:1[download:1,upload:0]|disk:0[read:1]|cpu:0|appmem:1"),
            ["license", "net[download]", "appmem", "tab", "mem"],
            "적힌 순서 · 숨김 제외 · 항목을 전부 끈 칸은 빠짐 · 빠진 칸은 정의 순으로 보충"
        );
        // 옛 형식(9a8ed19 — mem[app,system] · appmem 없음)으로 저장된 값: 없어진 항목은 버리고 · 사용자 순서는 지키고 ·
        // 새 칸(appmem)은 정의상 앞 형제(net) 뒤 = 라이선스 앞에 보충된다.
        assert_eq!(
            ids("tab:1|cpu:1|mem:1[app:1,system:1]|disk:1[write:1,read:1]|net:1[download:1,upload:1]|license:1"),
            [
                "tab",
                "cpu",
                "mem",
                "disk[write,read]",
                "net[download,upload]",
                "appmem",
                "license"
            ]
        );
        assert_eq!(fmt_rate(0), "0 B/s");
        assert_eq!(fmt_rate(1536), "1.5 KB/s");
    }
}
