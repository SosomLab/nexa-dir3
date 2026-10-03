//! App — 상태줄 구성(docs/22 NEW-003 · DR-23) + 탭 상태바 메뉴(NEW-004 1차).
//!
//! 창 아래 상태줄의 오른쪽 칸 = `statusbar.layout`의 순서/표시(순서 편집 창 · 기본 = 탭 · CPU · 메모리 · 디스크 · 네트워크 · 라이선스): 탭 n/m · **시스템(PC 전체)**의
//! CPU % · 메모리 · 디스크 읽기/쓰기 속도 · 네트워크 다운로드/업로드 속도(사용자 10-04 "시스템 상태값 · 네트워크 추가") ·
//! 라이선스(클릭 = 라이선스 창). 부하는 `statusbar.load_interval_ms`마다 [`platform::sysload::sample`]로 조회한다
//! (부하 칸이 하나도 없으면 조회도 · 깨우기도 없다).

use crate::app::ctxmenu::CtxKind;
use crate::platform::sysload::{self, SysLoad};
use crate::*;
use nexa_ctl::StatusSeg;

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

impl App {
    /// 상태줄 오른쪽 칸(지금 값으로).
    pub(crate) fn status_segments(&self) -> Vec<StatusSeg> {
        let dash = || "–".to_string();
        let load: Option<SysLoad> = self.load;
        // 항목이 있는 칸 = 보이는 항목을 순서대로 ` · `로 잇는다.
        let join = |parts: Vec<String>| parts.join(" · ");
        status_items_of(self.settings.get("statusbar.layout").unwrap_or(""))
            .into_iter()
            .map(|(id, kids)| match id {
                "tab" => {
                    let p = &self.panels[self.active];
                    StatusSeg::label(
                        id,
                        trf(
                            "status.tab",
                            &[
                                &(p.active_index() + 1).to_string(),
                                &p.tab_count().to_string(),
                            ],
                        ),
                    )
                }
                "cpu" => StatusSeg::label(
                    id,
                    trf(
                        "status.cpu",
                        &[&load.map_or_else(dash, |l| format!("{:.1}", l.cpu_pct))],
                    ),
                ),
                // 메모리 = 이 프로그램(nexa-sql 상태줄과 같은 뜻) · 시스템 사용률.
                "mem" => StatusSeg::label(
                    id,
                    join(
                        kids.iter()
                            .map(|k| match *k {
                                "app" => trf(
                                    "status.mem.app",
                                    &[&load
                                        .and_then(|l| l.mem_app)
                                        .map_or_else(dash, filelist::format_size)],
                                ),
                                _ => trf(
                                    "status.mem.sys",
                                    &[
                                        &load.map_or_else(dash, |l| format!("{:.0}", l.mem_pct())),
                                        &load.map_or_else(dash, |l| {
                                            filelist::format_size(l.mem_used)
                                        }),
                                    ],
                                ),
                            })
                            .collect(),
                    ),
                ),
                "disk" => {
                    let d = load.and_then(|l| l.disk_bps);
                    let parts = kids
                        .iter()
                        .map(|k| match *k {
                            "write" => trf(
                                "status.disk.write",
                                &[&d.map_or_else(dash, |v| fmt_rate(v.1))],
                            ),
                            _ => trf(
                                "status.disk.read",
                                &[&d.map_or_else(dash, |v| fmt_rate(v.0))],
                            ),
                        })
                        .collect();
                    StatusSeg::label(id, trf("status.disk", &[&join(parts)]))
                }
                "net" => {
                    let n = load.and_then(|l| l.net_bps);
                    let parts = kids
                        .iter()
                        .map(|k| match *k {
                            "download" => trf(
                                "status.net.down",
                                &[&n.map_or_else(dash, |v| fmt_rate(v.0))],
                            ),
                            _ => trf("status.net.up", &[&n.map_or_else(dash, |v| fmt_rate(v.1))]),
                        })
                        .collect();
                    StatusSeg::label(id, join(parts))
                }
                _ => StatusSeg::new(id, self.license_badge()),
            })
            .collect()
    }

    /// 부하 칸이 하나라도 있는가.
    fn status_wants_load(&self) -> bool {
        status_items_of(self.settings.get("statusbar.layout").unwrap_or(""))
            .iter()
            .any(|(k, _)| matches!(*k, "cpu" | "mem" | "disk" | "net"))
    }

    /// 유휴 틱 — 주기가 됐으면 부하를 조회해 칸을 갱신하고 다음 조회 시각을 돌려준다(부하 칸이 없으면 `None` = 깨우지 않음).
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
                self.load = Some(match self.load_prev {
                    Some((at, prev)) => sysload::load_between(prev, cur, now - at),
                    None => sysload::load_between(cur, cur, Duration::ZERO),
                });
                self.load_prev = Some((now, cur));
                let mut inv = Invalidations::default();
                let segs = self.status_segments();
                if self.statusbar.set_segments(segs, &mut inv) {
                    self.redraw();
                }
            }
        }
        Some(self.load_next)
    }

    /// 상태줄 칸 클릭 — 라이선스 = 라이선스 창(우클릭은 호스트가 순서 편집 창을 연다).
    pub(crate) fn status_click(&mut self, id: &str, right: bool) {
        if id == "license" && !right {
            self.command("help.license");
        }
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
                "mem[app,system]",
                "disk[write,read]",
                "net[download,upload]",
                "license"
            ],
            "빈 값 = 기본(전부)"
        );
        assert_eq!(
            ids("license:1|net:1[upload:1,download:0]|disk:1[read:1,write:1]|cpu:0|mem:1[app:0,system:0]"),
            ["license", "net[upload]", "disk[read,write]", "tab"],
            "적힌 순서 · 숨김 제외 · 항목을 전부 끈 칸은 빠짐 · 빠진 칸은 정의 순으로 보충"
        );
        assert_eq!(fmt_rate(0), "0 B/s");
        assert_eq!(fmt_rate(1536), "1.5 KB/s");
    }
}
