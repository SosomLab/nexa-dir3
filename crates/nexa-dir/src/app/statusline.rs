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
    crate::order::parse_order_with(defs, value)
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
        // 화살표 조각: `↑ 1.2 MB/s`(견본 = 화살표 + 가장 넓은 속도 글).
        let arrow = |up: bool, v: Option<u64>| {
            let a = if up { "↑ " } else { "↓ " };
            part(format!("{a}{}", v.map_or_else(dash, fmt_rate)))
                .color(if up { up_c } else { down_c })
                .hints(size_hints(a, "/s"))
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
                    let mut parts = vec![part(tr("status.abbr.disk"))];
                    parts.extend(kids.iter().map(|k| match *k {
                        "read" => arrow(true, d.map(|v| v.0)),
                        _ => arrow(false, d.map(|v| v.1)),
                    }));
                    StatusSeg::with_parts(id, parts)
                }
                "net" => {
                    let n = load.and_then(|l| l.net_bps);
                    let mut parts = vec![part(tr("status.abbr.net"))];
                    parts.extend(kids.iter().map(|k| match *k {
                        "upload" => arrow(true, n.map(|v| v.1)),
                        _ => arrow(false, n.map(|v| v.0)),
                    }));
                    StatusSeg::with_parts(id, parts)
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

    /// 탭 상태바 칸 클릭 → 상세 메뉴(좌 · 우클릭 같은 메뉴 — 1차): 폴더 = 항목 수 상세 · Git = 브랜치 · 복사 · 새로 고침.
    pub(crate) fn open_tab_status_menu(&mut self, panel: usize, seg: &str) {
        let p = &self.panels[panel];
        let root = p.root_path();
        let info = |text: String| CtxItem::maybe("aux.info", text, false);
        let mut items: Vec<CtxItem> = Vec::new();
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
                items.push(CtxItem::Separator);
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
    }

    /// 보조 메뉴(탭 상태바 · 툴바 · 런처) 항목 실행.
    pub(crate) fn aux_menu_action(&mut self, panel: usize, id: &str) {
        match id {
            "aux.git.copy" => {
                if let Some((_, branch)) = self.panels[panel].git_info() {
                    let _ = clipboard::write_text(&branch);
                }
            }
            "aux.sb.edit" => self.open_order_editor("statusbar.layout"),
            "aux.refresh" => {
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
        assert_eq!(fmt_rate(0), "0 B/s");
        assert_eq!(fmt_rate(1536), "1.5 KB/s");
    }
}
