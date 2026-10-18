//! 파일 목록 행 공급자 — `ndir-tree::Tree`(dir2 인라인 트리 평면 스트림)를 nexa-grid [`RowSource`]로 잇는다.
//!
//! dir2에서는 `nexa-app/src/panel.rs`가 같은 일을 했다(docs/port/13 · 21 G-1): 위젯은 가시 행만 묻고, 펼침/선택/정렬/타입어헤드는
//! 코어(`ndir-tree`)가 결정한다. 이 파일은 **번역만**(셀 텍스트 서식 포함) — 창·OS API를 모른다(docs/15 §2-1).
//!
//! 컬럼 키(dir2 docs/port/13 §2-5 순서): 0 이름(트리) · 1 확장자 · 2 크기 · 3 수정 시각 · 4 종류(`col.*` i18n). 정렬 키 대응 = [`sort_key_of`].

use ndir_core::FileKind;
use ndir_tree::{FindScope, SelectMode, SortKey, SortSpec, Tree};
use nexa_grid::{Marker, RowItem, RowSource, SelectOp};
use std::path::{Path, PathBuf};

/// 컬럼 키(그리드 `Column.key`).
pub(crate) const COL_NAME: u32 = 0;
pub(crate) const COL_EXT: u32 = 1;
pub(crate) const COL_SIZE: u32 = 2;
pub(crate) const COL_MODIFIED: u32 = 3;
pub(crate) const COL_KIND: u32 = 4;
/// 내 PC 전용 열(dir2 X-17 · PANEL-044): 전체 크기 · 여유 공간 — 값은 `Disk` 포트가 채운다(`set_drive_space`).
pub(crate) const COL_TOTAL: u32 = 5;
pub(crate) const COL_FREE: u32 = 6;
/// 상태 열(dir3 신규 10-03 — 클라우드/네트워크 파일 상태 · 아이콘 셀 · dir2 CLOUD-096 β 제안).
pub(crate) const COL_STATUS: u32 = 7;

/// 상태 → 아이콘 키(`status:<이름>` — 리졸버 `app/row_icons.rs`가 색 입힌 그림을 준다) · 표시할 것이 없으면 `None`.
pub(crate) fn status_icon_key(st: ndir_vfs::FileStatus) -> Option<&'static str> {
    use ndir_vfs::FileStatus as S;
    Some(match st {
        S::None => return None,
        S::CloudOnly => "status:cloud",
        S::AvailableLocally => "status:local",
        S::AlwaysKeep => "status:pinned",
        S::Network => "status:network",
        S::Nfd => return None, // 아이콘 없음 = 글 "NFD"(상태 열 · 10-18)
    })
}

/// 상태 → 글(아이콘을 못 그릴 때의 폴백 · 정보 표시).
pub(crate) fn status_label(st: ndir_vfs::FileStatus) -> String {
    use ndir_vfs::FileStatus as S;
    match st {
        S::None => String::new(),
        S::CloudOnly => ndir_i18n::tr("status.cloudOnly"),
        S::AvailableLocally => ndir_i18n::tr("status.availableLocally"),
        S::AlwaysKeep => ndir_i18n::tr("status.alwaysKeep"),
        S::Network => ndir_i18n::tr("status.network"),
        S::Nfd => ndir_i18n::tr("status.nfd"),
    }
}

/// 탭 제목(dir2 PANEL-013): 가상 최상위 = `nav.mypc` · 일반 = 마지막 경로 요소 · 드라이브 루트 = `D:`(후행 구분자 제거) ·
/// Unix 루트 = `/`(구분자를 떼면 빈 글자가 되던 것 — 사용자 10-03 "/ 위치의 탭 이름이 공백").
pub(crate) fn title_of(p: &Path) -> String {
    if ndir_vfs::is_virtual_root(p) {
        return ndir_i18n::tr("nav.mypc");
    }
    match p.file_name() {
        Some(n) if !n.is_empty() => n.to_string_lossy().into_owned(),
        _ => {
            let full = p.to_string_lossy();
            let cut = full.trim_end_matches(['\\', '/']);
            if cut.is_empty() && !full.is_empty() {
                "/".to_string()
            } else {
                cut.to_string()
            }
        }
    }
}

/// 경로 바 문자열: 가상 최상위 = 사람이 읽는 라벨 · 그 밖 = 경로 그대로.
pub(crate) fn display_path(p: &Path) -> String {
    if ndir_vfs::is_virtual_root(p) {
        ndir_i18n::tr("nav.mypc")
    } else {
        p.to_string_lossy().into_owned()
    }
}

/// 바로 가기 확장자(탐색기가 이름에서 늘 숨기는 것 · 소문자).
const SHORTCUT_EXTS: [&str; 3] = [".lnk", ".url", ".appref-ms"];

/// `name`이 바로 가기 파일이면 `(확장자 뺀 이름, 확장자(점 포함 · 원래 대소문자))`(순수 · 대소문자 무시 · 이름이 확장자뿐이면 `None`).
pub(crate) fn split_shortcut_ext(name: &str) -> Option<(&str, &str)> {
    SHORTCUT_EXTS.iter().find_map(|ext| {
        let cut = name.len().checked_sub(ext.len())?;
        (cut > 0 && name.is_char_boundary(cut) && name[cut..].eq_ignore_ascii_case(ext))
            .then(|| name.split_at(cut))
    })
}

/// Unix "내 PC"의 홈 폴더 행인가(`$HOME`과 같은 경로) — 용량은 보여도 종류는 드라이브가 아니라 폴더.
fn is_home_dir(name: &str) -> bool {
    cfg!(unix)
        && std::env::var_os("HOME")
            .is_some_and(|h| Path::new(&h) == Path::new(name.trim_end_matches('/')) && name != "/")
}

/// Windows 바로 가기 파일(`.lnk` · 대소문자 무시)인가 — 대상 해석(GAP-007) 대상.
pub(crate) fn is_lnk(path: &Path) -> bool {
    path.extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("lnk"))
}

/// 목록·이름 바꾸기에 보이는 이름(GAP-006 · dir2 `source.rs:456-465`): `hide`(= 이 OS가 바로 가기 확장자를 숨기는가)이고
/// 폴더가 아닌 바로 가기 파일이면 확장자를 뺀다(확장자 열은 `lnk` 그대로).
pub(crate) fn display_name(name: &str, is_dir: bool, hide: bool) -> &str {
    if hide && !is_dir {
        if let Some((stem, _)) = split_shortcut_ext(name) {
            return stem;
        }
    }
    name
}

/// 이름 바꾸기 확정 이름(dir2 `win.rs:4036-4045`): 원래 이름이 숨겨진 바로 가기 확장자를 가졌고 새 이름에 그 확장자가 없으면
/// 다시 붙인다(사용자가 본 것은 확장자 없는 이름 — 붙이지 않으면 바로 가기가 깨진다).
pub(crate) fn restore_shortcut_ext(old: &str, new: &str, is_dir: bool, hide: bool) -> String {
    if hide && !is_dir {
        if let Some((_, ext)) = split_shortcut_ext(old) {
            let has = new.len() > ext.len()
                && new.is_char_boundary(new.len() - ext.len())
                && new[new.len() - ext.len()..].eq_ignore_ascii_case(ext);
            if !has && !new.is_empty() {
                return format!("{new}{ext}");
            }
        }
    }
    new.to_string()
}

/// 열람 옵션(설정 `list.*`에서) — 다시 열 때 그대로 쓴다.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ListOpts {
    pub show_hidden: bool,
    pub show_dotfiles: bool,
    /// 보호된 운영 체제 항목 표시(`list.show_protected`).
    pub show_protected: bool,
    pub folders_first: bool,
    pub case_sensitive: bool,
    /// 자연 정렬(숫자를 값으로 · 설정 `list.sort_natural` = 새 탭 기본값 · 탭 보기 옵션 5번째 — 10-08 `list.view_scope` 적용).
    pub natural: bool,
    /// 빈 폴더(현재 보기로 보일 자식이 없는 폴더)의 펼침 글리프 숨김(설정 `list.hide_empty_glyph` · dir2 X-43 · WINC-016 · 10-18 — 종전에는 키만 있고
    /// 읽는 곳이 없어 모든 폴더에 ▸가 그려졌다). 전역(탭 보기 옵션이 아니다).
    pub hide_empty_glyph: bool,
    /// 이름을 NFC로 합성해 표시(설정 `list.nfc_display` · 10-18 · macOS NFD 이름의 자모가 흩어져 보이는 것) — 정렬·타입어헤드·이름 바꾸기는 원래 이름.
    pub nfc_display: bool,
}

/// 탭 보기 옵션 5종 `(숨김, Dot, 폴더 우선, 대소문자 구분 정렬, 자연 정렬)` — 탭이 소유하고 보기 범위(`list.view_scope`)만큼 함께 바뀐다.
pub(crate) type ViewOpts = (bool, bool, bool, bool, bool);

impl ListOpts {
    /// 탭 보기 옵션 5종만 바꾼 사본 — 나머지(보호 항목 표시 = 전역)는 그대로.
    pub(crate) fn with_view(
        self,
        (hidden, dot, folders_first, case, natural): ViewOpts,
    ) -> ListOpts {
        ListOpts {
            show_hidden: hidden,
            show_dotfiles: dot,
            folders_first,
            case_sensitive: case,
            natural,
            ..self
        }
    }

    /// 탭 보기 옵션 5종.
    pub(crate) fn view(self) -> ViewOpts {
        (
            self.show_hidden,
            self.show_dotfiles,
            self.folders_first,
            self.case_sensitive,
            self.natural,
        )
    }

    /// 세션 플래그(dir2 `panel{i}.views` — bit0 숨김 · bit1 Dot · bit2 폴더 우선 · **bit3 대소문자 구분**(dir3 추가 —
    /// dir2는 하위 3비트만 읽는다) · **bit4 자연 정렬 끔**(10-08 · 반전 저장 — 기본값이 켬이라 종전 세션(bit4 = 0)이 그대로 켬으로 읽힌다)).
    pub(crate) fn view_flags(self) -> u8 {
        u8::from(self.show_hidden)
            | u8::from(self.show_dotfiles) << 1
            | u8::from(self.folders_first) << 2
            | u8::from(self.case_sensitive) << 3
            | u8::from(!self.natural) << 4
    }

    /// 세션 플래그 → 보기 옵션 5종(bit4 = 자연 정렬 **끔**).
    pub(crate) fn view_of_flags(f: u8) -> ViewOpts {
        (f & 1 != 0, f & 2 != 0, f & 4 != 0, f & 8 != 0, f & 16 == 0)
    }
}

/// 한 패널의 행 공급자.
#[derive(Debug)]
pub(crate) struct TreeSource {
    tree: Option<Tree>,
    path: PathBuf,
    opts: ListOpts,
    /// 이 폴더가 네트워크 위치(NFS · SMB 마운트 · UNC) 아래인가 — 열 때 한 번 판정(상태 열).
    on_network: bool,
    /// 열기 실패 사유(상태줄에 · 빈 목록).
    error: Option<String>,
    /// 드라이브 이름(`C:\`) → (전체, 여유) — 가상 최상위에서만 · 호스트가 Disk 포트로 채운다.
    drive_space: std::collections::HashMap<String, (u64, u64)>,
    /// 용량을 이미 물었다(워커 진행 중이거나 결과가 비었음 — 매 사건 재시도하지 않는다 · 10-18). 다시 읽으면 내린다.
    drive_space_asked: bool,
    /// 잘라내기 대기 경로(dir2 SHELL-044/X-32 — 그 행은 흐리게 · 호스트가 클립보드와 동기).
    cut_marks: std::collections::HashSet<PathBuf>,
    /// 이름 앞 아이콘 끔(설정 `list.row_icons` · 성능 향상 모드) — 끄면 [`RowSource::icon`]이 없음을 돌려준다(조회 · 그리기 · 자리 모두 없다).
    no_icons: bool,
    /// 빈 폴더 글리프 판정 캐시(X-43 · 노드 → 가시 자식 있음): 미열거 폴더는 `probe_has_visible_child` 1회 뒤 캐시 · 재로드/옵션 변경 = 비움.
    probe_cache: std::cell::RefCell<std::collections::HashMap<ndir_tree::NodeId, bool>>,
    /// 프로브 워커 결과함(10-18 · 창이 있으면 그리기 중 `read_dir`을 하지 않고 워커에 맡긴다) · 세대(재로드 = 새 노드 id) · 워커에 가 있는 노드.
    probe_inbox: probe::Inbox,
    probe_gen: u64,
    probe_pending: std::cell::RefCell<std::collections::HashSet<ndir_tree::NodeId>>,
}

impl TreeSource {
    /// 폴더를 연다 — 실패해도 공급자는 만들어진다(빈 목록 + `error`).
    pub(crate) fn open(path: &Path, opts: ListOpts) -> TreeSource {
        let mut s = TreeSource {
            tree: None,
            path: path.to_path_buf(),
            opts,
            on_network: !ndir_vfs::is_virtual_root(path) && ndir_vfs::is_network_path(path),
            error: None,
            drive_space: std::collections::HashMap::new(),
            drive_space_asked: false,
            cut_marks: std::collections::HashSet::new(),
            no_icons: false,
            probe_cache: Default::default(),
            probe_inbox: Default::default(),
            probe_gen: 0,
            probe_pending: Default::default(),
        };
        s.reload();
        s
    }

    /// 프로브 워커 결과를 캐시에 옮긴다(유휴 틱) — 바뀐 것이 있으면 `true`(마커 다시 그리기).
    pub(crate) fn drain_probes(&mut self) -> bool {
        let got: Vec<(u64, ndir_tree::NodeId, bool)> = self
            .probe_inbox
            .lock()
            .map(|mut v| v.drain(..).collect())
            .unwrap_or_default();
        let mut changed = false;
        for (gen, id, v) in got {
            if gen != self.probe_gen {
                continue; // 재로드 전 노드 id(낡은 결과)
            }
            self.probe_pending.borrow_mut().remove(&id);
            self.probe_cache.borrow_mut().insert(id, v);
            changed = true;
        }
        changed
    }

    /// 보이는 폴더 행의 프로브를 **지금** 제출한다(덤프·단언 전 · 10-18): 그리기가 한 번도 안 돈 채 덤프가 오면 제출된 프로브가 없어
    /// 기다릴 것도 없고, 덤프가 행을 읽는 순간에야 제출돼 ▸가 남았다(T4 `nav-up-select` 게이트 빨강 · 기계 속도에 따라 결정적).
    pub(crate) fn probe_prefetch(&self) {
        if !probe::is_async() || !self.opts.hide_empty_glyph {
            return;
        }
        let Some(t) = &self.tree else { return };
        for i in 0..t.visible_len().min(PROBE_PREFETCH_CAP) {
            if let Some(r) = t.row_ref(i) {
                if r.has_children {
                    let _ = self.dir_has_visible_children(r.id);
                }
            }
        }
    }

    /// 워커 결과를 기다리는 폴더가 있는가(이벤트 루프 깨움 · 덤프 대기).
    pub(crate) fn probes_pending(&self) -> bool {
        !self.probe_pending.borrow().is_empty()
    }

    /// 이름 앞 아이콘 켬/끔 — 바뀌었으면 true.
    pub(crate) fn set_icons(&mut self, on: bool) -> bool {
        let changed = self.no_icons == on;
        self.no_icons = !on;
        changed
    }

    /// 잘라내기 표식 집합 교체 — 바뀌었으면 true(호스트가 목록 무효화).
    pub(crate) fn set_cut_marks(&mut self, marks: std::collections::HashSet<PathBuf>) -> bool {
        if self.cut_marks == marks {
            return false;
        }
        self.cut_marks = marks;
        true
    }

    /// 같은 경로를 다시 읽는다(F5 · 옵션 변경 · 폴더 감시) — **무간섭 재열람**(dir2 PANEL-036 `reopen_filtered`):
    /// 정렬 키 · 펼친 폴더 · 선택을 스냅샷해 새 트리에 경로로 되돌린다(사라진 항목은 조용히 빠진다). 종전에는 새 트리를 기본 정렬로
    /// 열기만 해 폴더가 밖에서 바뀔 때마다 선택·펼침·정렬이 풀렸다(GAP-005 · 10-03 T4 `ctx.wait` 시나리오가 적발).
    pub(crate) fn reload(&mut self) {
        self.probe_cache.borrow_mut().clear(); // 새 트리 = 새 노드 id · FS 변화 재실측(X-43)
        self.drive_space_asked = false;
        self.probe_gen += 1;
        self.probe_pending.borrow_mut().clear();
        let (keys, expanded, selected) = match &self.tree {
            Some(t) => {
                let expanded: Vec<String> = (0..t.visible_len())
                    .filter_map(|i| t.visible_id(i))
                    .filter(|&id| t.is_expanded(id) == Some(true))
                    .filter_map(|id| t.node_path(id))
                    .map(|p| p.to_string_lossy().into_owned())
                    .collect();
                let selected: Vec<String> = t
                    .selected_paths()
                    .into_iter()
                    .map(|p| p.to_string_lossy().into_owned())
                    .collect();
                (t.sort_spec().keys.clone(), expanded, selected)
            }
            None => (vec![(SortKey::Name, false)], Vec::new(), Vec::new()),
        };
        match Tree::open_visible(
            &self.path,
            self.opts.show_hidden,
            self.opts.show_dotfiles,
            self.opts.show_protected,
        ) {
            Ok(mut t) => {
                t.set_sort(SortSpec {
                    keys,
                    folders_first: self.opts.folders_first,
                    case_sensitive: self.opts.case_sensitive,
                    natural: self.opts.natural,
                });
                // 펼침은 가시 순서(부모 먼저)로 모았으므로 그대로 다시 펼치면 된다 · 없어진 폴더는 건너뛴다.
                for dir in &expanded {
                    let _ = t.expand_path(dir);
                }
                // 선택 복원 = 색인 한 번(O(n + k) · 종전 경로마다 선형 검색 · 10-18).
                let _ = t.select_paths(&selected);
                self.tree = Some(t);
                self.error = None;
            }
            Err(e) => {
                self.tree = None;
                self.error = Some(e.to_string());
            }
        }
    }

    /// 행의 저장 상태(상태 열): 클라우드 속성이 있으면 그것 · 없고 네트워크 위치면 네트워크 · 그 밖 = 없음.
    pub(crate) fn status_of(&self, index: usize) -> ndir_vfs::FileStatus {
        let Some(r) = self.tree.as_ref().and_then(|t| t.row_ref(index)) else {
            return ndir_vfs::FileStatus::None;
        };
        match ndir_vfs::cloud_status(r.attrs) {
            // NFD 이름 식별(10-18) — 클라우드 상태가 없을 때만(상태 열은 한 가지만 보인다).
            ndir_vfs::FileStatus::None if ndir_ops::nfc::is_decomposed(r.name) => {
                ndir_vfs::FileStatus::Nfd
            }
            ndir_vfs::FileStatus::None if self.on_network => ndir_vfs::FileStatus::Network,
            st => st,
        }
    }

    /// 이 소스의 열람 옵션(탭별 보기 옵션의 원천 — 숨김 · Dot · 폴더 우선은 탭이 소유한다).
    pub(crate) fn opts(&self) -> ListOpts {
        self.opts
    }

    pub(crate) fn set_opts(&mut self, opts: ListOpts) {
        if self.opts != opts {
            self.opts = opts;
            self.reload();
        }
    }

    /// 폴더에 현재 보기(필터)로 보여줄 자식이 있는가(X-43 · dir2 `dir_has_visible_children` 이식) — 마커 판정 전용:
    /// 열거 완료 폴더 = 트리 실측(필터 반영 children) · 미열거 = 프로브 1회 후 캐시 · 네트워크 위치 = 프로브 생략(글리프 유지 · X-43 β).
    fn dir_has_visible_children(&self, id: ndir_tree::NodeId) -> bool {
        let Some(t) = self.tree.as_ref() else {
            return true;
        };
        if let Some(n) = t.loaded_child_count(id) {
            return n > 0;
        }
        if let Some(&v) = self.probe_cache.borrow().get(&id) {
            return v;
        }
        let v = match t.node_path(id) {
            Some(_) if self.on_network => true,
            // 창이 있으면 워커에 맡기고 결과가 올 때까지 ▸ 유지(그리기 중 `read_dir` 없음 · 매핑 드라이브·느린 디스크에서도 프레임 유지 · 10-18).
            Some(p) if probe::is_async() => {
                if self.probe_pending.borrow_mut().insert(id) {
                    probe::submit(probe::Job {
                        gen: self.probe_gen,
                        id,
                        path: p.to_path_buf(),
                        flags: (
                            self.opts.show_hidden,
                            self.opts.show_dotfiles,
                            self.opts.show_protected,
                        ),
                        inbox: std::sync::Arc::clone(&self.probe_inbox),
                    });
                }
                return true;
            }
            Some(p) => t.probe_has_visible_child(p),
            None => true,
        };
        self.probe_cache.borrow_mut().insert(id, v);
        v
    }

    /// 가상 최상위(내 PC)를 보고 있는가.
    pub(crate) fn is_virtual_root(&self) -> bool {
        ndir_vfs::is_virtual_root(&self.path)
    }

    /// 드라이브 행 이름들(가상 최상위) — 호스트가 용량을 조회해 [`Self::set_drive_space`]로 넣는다.
    pub(crate) fn drive_names(&self) -> Vec<String> {
        if !self.is_virtual_root() {
            return Vec::new();
        }
        (0..self.len()).map(|i| self.row(i).text).collect()
    }

    pub(crate) fn drive_space_known(&self) -> bool {
        !self.drive_space.is_empty()
    }

    pub(crate) fn drive_space_asked(&self) -> bool {
        self.drive_space_asked
    }

    pub(crate) fn mark_drive_space_asked(&mut self) {
        self.drive_space_asked = true;
    }

    pub(crate) fn set_drive_space(&mut self, space: Vec<(String, (u64, u64))>) {
        self.drive_space = space.into_iter().collect();
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// 접혀 있는 폴더 행을 펼친다(이미 펼쳐졌거나 폴더가 아니면 그대로) — 펼쳤으면 `true`(끌어오다 머물면 열기 · SHELL-067).
    pub(crate) fn expand_row(&mut self, index: usize) -> bool {
        let Some(t) = self.tree.as_mut() else {
            return false;
        };
        match t.visible_id(index) {
            Some(id) if t.is_expanded(id) == Some(false) => t.expand(id).is_ok(),
            _ => false,
        }
    }

    /// 화면에 **펼쳐져 있는** 폴더들의 경로(위에서부터 최대 `cap`개) — 폴더 감시 대상(dir2 WINB-014 "가시 펼침 폴더").
    pub(crate) fn expanded_dirs(&self, cap: usize) -> Vec<PathBuf> {
        let Some(t) = self.tree.as_ref() else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for i in 0..t.visible_len() {
            if out.len() >= cap {
                break;
            }
            let Some(id) = t.visible_id(i) else { continue };
            if t.is_expanded(id) == Some(true) {
                if let Some(p) = t.node_path(id) {
                    out.push(p.to_path_buf());
                }
            }
        }
        out
    }

    /// 가시 행의 전체 경로.
    pub(crate) fn row_path(&self, index: usize) -> Option<PathBuf> {
        let t = self.tree.as_ref()?;
        let id = t.visible_id(index)?;
        t.node_path(id).map(Path::to_path_buf)
    }

    pub(crate) fn row_is_dir(&self, index: usize) -> bool {
        self.tree
            .as_ref()
            .and_then(|t| t.row_ref(index))
            .is_some_and(|r| r.kind == FileKind::Dir)
    }

    /// 선택된 항목의 전체 경로(선택 순).
    /// 선택 반전 — 목록이 있으면 `true`.
    pub(crate) fn invert_selection(&mut self) -> bool {
        match self.tree.as_mut() {
            Some(t) => {
                t.invert_selection_visible();
                true
            }
            None => false,
        }
    }

    pub(crate) fn selected_paths(&self) -> Vec<PathBuf> {
        self.tree
            .as_ref()
            .map(|t| {
                t.selected_paths()
                    .into_iter()
                    .map(Path::to_path_buf)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// 선택된 항목의 전체 경로를 **화면에 보이는 순서**(위 → 아래)로 — 경로/이름 복사용(dir2 `display_order_targets` · win.rs:2759:
    /// 선택 순은 화면 순서와 다르다). 보이는 행에 선택이 없으면 선택 순 그대로.
    pub(crate) fn selected_paths_in_view_order(&self) -> Vec<PathBuf> {
        let Some(t) = self.tree.as_ref() else {
            return Vec::new();
        };
        let out: Vec<PathBuf> = (0..t.visible_len())
            .filter_map(|i| {
                let id = t.visible_id(i)?;
                if !t.is_selected(id) {
                    return None;
                }
                t.node_path(id).map(Path::to_path_buf)
            })
            .collect();
        if out.is_empty() {
            return self.selected_paths();
        }
        out
    }

    pub(crate) fn selection_count(&self) -> usize {
        self.tree.as_ref().map_or(0, Tree::selection_count)
    }

    /// 상태줄 요약 — `status.itemCount` / `status.selectedCount`.
    pub(crate) fn status_text(&self) -> String {
        if let Some(e) = &self.error {
            return e.clone();
        }
        let n = self.len();
        let sel = self.selection_count();
        if sel > 0 {
            ndir_i18n::trf("status.selectedCount", &[&sel.to_string()])
        } else {
            ndir_i18n::trf("status.itemCount", &[&n.to_string()])
        }
    }
}

/// 그리드 컬럼 키 → 코어 정렬 키.
pub(crate) fn sort_key_of(col: u32) -> Option<SortKey> {
    Some(match col {
        COL_NAME => SortKey::Name,
        // 확장자 열 = 확장자순(dir2 source.rs:434 PANEL-065 — 종전 dir3는 종류 키로 묶어 확장자 + 종류 다중 정렬에 같은 키가
        // 두 번 들어갔다 · 10-03 적발).
        COL_EXT => SortKey::Ext,
        COL_KIND => SortKey::Kind,
        COL_SIZE => SortKey::Size,
        COL_MODIFIED => SortKey::Modified,
        COL_STATUS => SortKey::Status,
        _ => return None,
    })
}

/// 크기 서식(dir2 규약 · 탐색기식): 1024 단위 · 소수 1자리(KB 이상) · 폴더는 빈 값(호출자가 결정).
pub(crate) fn format_size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    let mut v = bytes as f64;
    let mut u = 0;
    while v >= 1024.0 && u + 1 < UNITS.len() {
        v /= 1024.0;
        u += 1;
    }
    format!("{v:.1} {}", UNITS[u])
}

/// Unix 초(UTC) → `YYYY-MM-DDThh:mm:ss`(freedesktop `.trashinfo` DeletionDate).
#[cfg_attr(not(all(unix, not(target_os = "macos"))), allow(dead_code))] // freedesktop Trash(Linux)만 쓴다.
pub(crate) fn format_iso_utc(unix_secs: i64) -> String {
    let days = unix_secs.div_euclid(86_400);
    let rem = unix_secs.rem_euclid(86_400);
    let (y, m, d) = civil_from_days(days);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}",
        rem / 3600,
        (rem % 3600) / 60,
        rem % 60
    )
}

/// Unix ms → **현지 시각** `YYYY-MM-DD HH:MM`(파일 목록 · 정보 도크 · 크래시 기록 — 사용자 10-06 "설정 파일을 수정했는데
/// 날짜가 안 바뀐다" = UTC로 찍혀 9시간 어긋나 보였다). 시간대 · DST는 OS가 그 순간 기준으로 계산한다(`nexa_fs::local_time`).
/// 0 이하 = 빈 값.
pub(crate) fn format_time_local(unix_ms: i64) -> String {
    if unix_ms <= 0 {
        return String::new();
    }
    let t = std::time::UNIX_EPOCH + std::time::Duration::from_millis(unix_ms as u64);
    let l = nexa_fs::local_time(t);
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}",
        l.year, l.month, l.day, l.hour, l.min
    )
}

/// Unix ms → `YYYY-MM-DD HH:MM` **시간대 변환 없이**(값이 이미 현지 벽시계인 경우 — DOS 시각(zip · cab)처럼 시간대가 없는
/// 압축 항목 시각 · `preview::archive::fmt_entry_time`이 오프셋을 더해 넘긴다). 화면의 파일 시각은 [`format_time_local`]로.
pub(crate) fn format_time(unix_ms: i64) -> String {
    if unix_ms <= 0 {
        return String::new();
    }
    let secs = unix_ms.div_euclid(1000);
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let (y, m, d) = civil_from_days(days);
    format!(
        "{y:04}-{m:02}-{d:02} {:02}:{:02}",
        rem / 3600,
        (rem % 3600) / 60
    )
}

/// 1970-01-01 기준 일수 → (년, 월, 일) — Howard Hinnant의 `civil_from_days`(윤년 포함 · 외부 crate 0).
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// 종류 라벨(i18n `kind.*` — 없으면 영문).
fn kind_label(kind: FileKind, name: &str) -> String {
    match kind {
        FileKind::Dir => ndir_i18n::tr("kind.folder"),
        FileKind::Symlink => ndir_i18n::tr("kind.link"),
        // 바로 가기 파일(.lnk · .url · .appref-ms) = "바로 가기"(탐색기 종류 · 확장자를 이름에서 숨기는 OS에서만 — GAP-008).
        _ if crate::platform::hides_shortcut_ext() && split_shortcut_ext(name).is_some() => {
            ndir_i18n::tr("kind.link")
        }
        _ => match name.rsplit_once('.') {
            Some((stem, ext)) if !stem.is_empty() && !ext.is_empty() => ext.to_ascii_uppercase(),
            _ => ndir_i18n::tr("kind.file"),
        },
    }
}

/// 파일마다 아이콘이 다른 확장자(dir2 `icons.rs:8-36` · PANEL-064) — 키 = 소문자 전체 경로.
const PER_FILE_ICON_EXTS: [&str; 7] = ["exe", "lnk", "ico", "cur", "msi", "scr", "appref-ms"];

/// 행 아이콘 키(dir2 `icons::icon_key` · PANEL-064): 폴더 = `dir` · 확장자 없음 = `file` · 파일별 확장자 = 소문자 전체 경로 · 그 외 = 확장자.
/// 드라이브 루트 같은 **가상 최상위의 항목**은 경로마다 고유 아이콘(셸이 경로를 본다) → 경로 키.
pub(crate) fn icon_key(is_dir: bool, path: &Path, per_path: bool) -> String {
    let lower = || path.to_string_lossy().to_lowercase();
    if per_path {
        return lower();
    }
    if is_dir {
        return "dir".into();
    }
    match path.extension().and_then(|e| e.to_str()) {
        None => "file".into(),
        Some(ext) => {
            let ext = ext.to_ascii_lowercase();
            if PER_FILE_ICON_EXTS.contains(&ext.as_str()) {
                lower()
            } else {
                ext
            }
        }
    }
}

impl RowSource for TreeSource {
    fn len(&self) -> usize {
        self.tree.as_ref().map_or(0, Tree::visible_len)
    }

    /// 상태 열 = 아이콘 셀(nexa-ui 132차 `cell_icon`) — 표시할 상태가 없으면 빈 칸.
    fn cell_icon(&self, index: usize, key: u32) -> Option<(String, String)> {
        if key != COL_STATUS {
            return None;
        }
        status_icon_key(self.status_of(index)).map(|k| (k.to_string(), String::new()))
    }

    /// 행 아이콘 `(키, 경로)`(dir2 `source.rs:426-432` · M1-7 셸 아이콘) — 그리는 쪽(nexa-grid `Adapt::draw_icon`)이 호스트 리졸버에 묻는다.
    fn icon(&self, index: usize) -> Option<(String, String)> {
        if self.no_icons {
            return None;
        }
        let row = self.tree.as_ref()?.row_ref(index)?;
        let is_dir = row.kind == FileKind::Dir;
        let path = self.row_path(index)?;
        // 링크(심볼릭 링크 · 정션)는 경로로 조회해야 셸이 화살표 오버레이를 얹는다(GAP-008 — 종류별 아이콘에는 링크 정보가 없다).
        let per_path = ndir_vfs::is_virtual_root(&self.path) || ndir_vfs::is_link_attrs(row.attrs);
        Some((
            icon_key(is_dir, &path, per_path),
            path.to_string_lossy().into_owned(),
        ))
    }

    /// 잘라내기 대기 행 = 흐림(nexa-grid X-32 · 집합이 비어 있으면 경로 계산도 하지 않는다 — dir2 `has_cut_marks` 선판정).
    fn is_ghosted(&self, index: usize) -> bool {
        !self.cut_marks.is_empty()
            && self
                .row_path(index)
                .is_some_and(|p| self.cut_marks.contains(&p))
    }

    fn row(&self, index: usize) -> RowItem {
        // 무할당 행 참조(10-18 체감 점검 — 종전 `row()`는 행마다 이름 String을 복제했다 · 셀마다 반복).
        let Some(r) = self.tree.as_ref().and_then(|t| t.row_ref(index)) else {
            return RowItem {
                text: String::new(),
                is_dir: false,
                depth: 0,
                marker: Marker::None,
            };
        };
        RowItem {
            is_dir: r.kind == FileKind::Dir,
            depth: r.depth,
            // 빈 폴더 글리프 숨김(X-43 · dir2 `source.rs` `row`): 현재 보기로 보일 자식이 없으면 마커 없음(탐색기 트리 관례) · 폴더 서식은 유지.
            marker: if !r.has_children
                || (self.opts.hide_empty_glyph && !self.dir_has_visible_children(r.id))
            {
                Marker::None
            } else if r.expanded {
                Marker::Expanded
            } else {
                Marker::Collapsed
            },
            text: {
                let shown = display_name(
                    r.name,
                    r.kind == FileKind::Dir,
                    crate::platform::hides_shortcut_ext(),
                );
                // NFC 표시(설정 · 10-18): 분해형 이름은 합성해 보인다(원래 이름은 트리에 그대로 · 상태 열 "NFD").
                if self.opts.nfc_display {
                    ndir_ops::nfc::compose(shown).into_owned()
                } else {
                    shown.to_string()
                }
            },
        }
    }

    fn cell(&self, index: usize, key: u32) -> String {
        let Some(r) = self.tree.as_ref().and_then(|t| t.row_ref(index)) else {
            return String::new();
        };
        match key {
            COL_EXT if r.kind == FileKind::Dir => String::new(),
            COL_EXT => match r.name.rsplit_once('.') {
                Some((stem, ext)) if !stem.is_empty() && !ext.is_empty() => ext.to_string(),
                _ => String::new(),
            },
            COL_STATUS => status_label(self.status_of(index)),
            COL_SIZE if r.kind == FileKind::Dir => String::new(),
            COL_SIZE => format_size(r.size),
            COL_MODIFIED => format_time_local(r.modified_unix_ms),
            COL_KIND
                if (self.drive_space.contains_key(r.name) || r.name.ends_with(":\\"))
                    && !is_home_dir(r.name) =>
            {
                ndir_i18n::tr("kind.drive")
            }
            COL_KIND => kind_label(r.kind, r.name),
            COL_TOTAL => self
                .drive_space
                .get(r.name)
                .map(|(t, _)| format_size(*t))
                .unwrap_or_default(),
            COL_FREE => self
                .drive_space
                .get(r.name)
                .map(|(_, f)| format_size(*f))
                .unwrap_or_default(),
            _ => String::new(),
        }
    }

    /// 타일 보조 줄(dir2 07-16): 드라이브 = "X 중 Y 사용 가능" + 사용량 바 · 그 밖 = 종류.
    fn tile_info(&self, index: usize) -> (String, Option<f32>) {
        let Some(r) = self.tree.as_ref().and_then(|t| t.row_ref(index)) else {
            return (String::new(), None);
        };
        if let Some(&(total, free)) = self.drive_space.get(r.name) {
            let used = total.saturating_sub(free);
            let frac = if total > 0 {
                used as f32 / total as f32
            } else {
                0.0
            };
            return (
                ndir_i18n::trf("drive.freeOf", &[&format_size(total), &format_size(free)]),
                Some(frac),
            );
        }
        (self.cell(index, COL_KIND), None)
    }

    fn toggle(&mut self, index: usize) -> bool {
        let Some(t) = self.tree.as_mut() else {
            return false;
        };
        let Some(id) = t.visible_id(index) else {
            return false;
        };
        match t.is_expanded(id) {
            Some(true) => {
                t.collapse(id);
                true
            }
            Some(false) => t.expand(id).is_ok(),
            None => false,
        }
    }

    fn set_sort(&mut self, keys: &[(u32, bool)]) -> bool {
        let Some(t) = self.tree.as_mut() else {
            return false;
        };
        let mapped: Vec<(SortKey, bool)> = keys
            .iter()
            .filter_map(|(k, desc)| sort_key_of(*k).map(|s| (s, *desc)))
            .collect();
        t.set_sort(SortSpec {
            keys: if mapped.is_empty() {
                vec![(SortKey::Name, false)]
            } else {
                mapped
            },
            folders_first: self.opts.folders_first,
            case_sensitive: self.opts.case_sensitive,
            natural: self.opts.natural,
        });
        true
    }

    fn is_selected(&self, index: usize) -> bool {
        self.tree
            .as_ref()
            .and_then(|t| t.visible_id(index).map(|id| t.is_selected(id)))
            .unwrap_or(false)
    }

    fn select(&mut self, index: usize, op: SelectOp) -> bool {
        let Some(t) = self.tree.as_mut() else {
            return false;
        };
        let Some(id) = t.visible_id(index) else {
            return false;
        };
        match op {
            SelectOp::Single => t.select(id, SelectMode::Single),
            SelectOp::Toggle => t.select(id, SelectMode::Toggle),
            SelectOp::RangeTo => t.select_range(id),
        }
        true
    }

    fn select_span(&mut self, lo: usize, hi: usize) -> bool {
        let Some(t) = self.tree.as_mut() else {
            return false;
        };
        let (Some(a), Some(b)) = (t.visible_id(lo), t.visible_id(hi)) else {
            return false;
        };
        t.select(a, SelectMode::Single);
        t.select_range(b);
        true
    }

    fn select_all(&mut self) -> bool {
        if let Some(t) = self.tree.as_mut() {
            t.select_all_visible();
            return true;
        }
        false
    }

    fn clear_selection(&mut self) -> bool {
        match self.tree.as_mut() {
            Some(t) if t.selection_count() > 0 => {
                t.clear_selection();
                true
            }
            _ => false,
        }
    }

    fn find_prefix(&self, caret: Option<usize>, prefix: &str) -> Option<usize> {
        self.tree
            .as_ref()?
            .find_prefix(caret, prefix, FindScope::VisibleStream)
    }
}

/// 덤프 전 프로브 선제출 상한(행 수 · 큰 폴더에서 수만 개를 한꺼번에 넣지 않게).
const PROBE_PREFETCH_CAP: usize = 4096;

/// 빈 폴더 프로브 워커(10-18 체감 점검): 스레드 1(첫 요청 때) · 큐 · 결과는 요청에 실린 결과함으로. 창이 없는(시험) 동안은 끔 = 종전 동기 프로브.
pub(crate) mod probe {
    use std::collections::VecDeque;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Condvar, Mutex, OnceLock};

    /// 결과함: (세대, 노드, 가시 자식 있음).
    pub(crate) type Inbox = Arc<Mutex<Vec<(u64, ndir_tree::NodeId, bool)>>>;

    pub(crate) struct Job {
        pub gen: u64,
        pub id: ndir_tree::NodeId,
        pub path: PathBuf,
        /// (show_hidden, show_dotfiles, show_protected).
        pub flags: (bool, bool, bool),
        pub inbox: Inbox,
    }

    struct Prober {
        queue: Mutex<VecDeque<Job>>,
        cv: Condvar,
    }

    static PROBER: OnceLock<Prober> = OnceLock::new();
    static ASYNC: AtomicBool = AtomicBool::new(false);

    /// 워커 모드(창이 생기면 켠다 · 시험은 끔 = 동기).
    pub(crate) fn set_async(on: bool) {
        ASYNC.store(on, Ordering::Relaxed);
    }

    pub(crate) fn is_async() -> bool {
        ASYNC.load(Ordering::Relaxed)
    }

    fn prober() -> &'static Prober {
        PROBER.get_or_init(|| {
            let p = Prober {
                queue: Mutex::new(VecDeque::new()),
                cv: Condvar::new(),
            };
            std::thread::Builder::new()
                .name("ndir-probe".into())
                .spawn(|| {
                    let p = prober();
                    loop {
                        let job = {
                            let mut q = p.queue.lock().unwrap_or_else(|e| e.into_inner());
                            while q.is_empty() {
                                q = p.cv.wait(q).unwrap_or_else(|e| e.into_inner());
                            }
                            q.pop_front()
                        };
                        let Some(job) = job else { continue };
                        let (h, d, pr) = job.flags;
                        let v = ndir_tree::probe_visible_child(&job.path, h, d, pr);
                        let (gen, id, inbox) = (job.gen, job.id, job.inbox);
                        let _ = inbox.lock().map(|mut g| g.push((gen, id, v)));
                    }
                })
                .expect("spawn probe worker");
            p
        })
    }

    /// 큐에 넣는다(워커는 첫 호출 때 생긴다).
    pub(crate) fn submit(job: Job) {
        let p = prober();
        if let Ok(mut q) = p.queue.lock() {
            q.push_back(job);
        }
        p.cv.notify_one();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts() -> ListOpts {
        ListOpts {
            show_hidden: true,
            show_dotfiles: true,
            show_protected: true,
            folders_first: true,
            case_sensitive: false,
            natural: false,
            hide_empty_glyph: false,
            nfc_display: true,
        }
    }

    /// NFC 표시(10-18): 분해형 이름은 켬이면 합성해 보이고 끄면 원래대로 · 상태 열 = NFD · 원래 이름은 트리에 그대로(이름 바꾸기 기준).
    #[test]
    fn nfd_names_display_composed_and_flagged() {
        let nfd = "\u{1112}\u{1161}\u{11AB}\u{1100}\u{1173}\u{11AF}.txt";
        let base = std::env::temp_dir().join(format!("ndir-nfc-list-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        std::fs::write(base.join(nfd), b"x").unwrap();
        let src = TreeSource::open(&base, opts());
        assert_eq!(src.row(0).text, "한글.txt", "켬 = 합성");
        assert_eq!(src.status_of(0), ndir_vfs::FileStatus::Nfd, "상태 열 = NFD");
        assert_eq!(
            src.row_path(0)
                .unwrap()
                .file_name()
                .unwrap()
                .to_str()
                .unwrap(),
            nfd,
            "원래 이름은 그대로"
        );
        let mut off = src;
        off.set_opts(ListOpts {
            nfc_display: false,
            ..opts()
        });
        assert_eq!(off.row(0).text, nfd, "끔 = 원래대로");
        let _ = std::fs::remove_dir_all(&base);
    }

    /// 빈 폴더 글리프 억제(X-43 · dir2 `source.rs` 시험 이식): 현재 필터로 보여줄 자식이 없는 폴더 = 마커 없음, is_dir(폴더 서식)은 유지 ·
    /// 필터가 내용을 드러내면 마커 복귀 · 토글 즉시 반영(옵션 변경 = 재로드 = 캐시 소거).
    #[test]
    fn hide_empty_glyph_suppresses_by_current_filter() {
        let base = std::env::temp_dir().join(format!("ndir-hide-glyph-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join("a_empty")).unwrap();
        std::fs::create_dir_all(base.join("b_dotonly")).unwrap();
        std::fs::write(base.join("b_dotonly/.dot"), b"x").unwrap();
        std::fs::create_dir_all(base.join("c_full")).unwrap();
        std::fs::write(base.join("c_full/a.txt"), b"x").unwrap();
        // 점 파일 숨김(Unix = show_hidden · Windows = show_dotfiles — 둘 다 끔) · 억제 꺼짐 = 현행(전부 Collapsed).
        let hidden = ListOpts {
            show_hidden: false,
            show_dotfiles: false,
            ..opts()
        };
        let mut src = TreeSource::open(&base, hidden);
        assert_eq!(
            src.row(0).marker,
            Marker::Collapsed,
            "억제 꺼짐 = 현행 유지"
        );
        src.set_opts(ListOpts {
            hide_empty_glyph: true,
            ..hidden
        });
        assert_eq!(src.row(0).marker, Marker::None, "빈 폴더 = 마커 억제");
        assert!(src.row(0).is_dir, "마커가 억제돼도 폴더 서식은 유지(X-43)");
        assert_eq!(
            src.row(1).marker,
            Marker::None,
            "점 파일뿐 = 현재 보기 기준 빔"
        );
        assert_eq!(
            src.row(2).marker,
            Marker::Collapsed,
            "가시 자식 있음 = 마커 유지"
        );
        // 필터가 내용을 드러내면(점 파일 표시) 마커 복귀.
        src.set_opts(ListOpts {
            hide_empty_glyph: true,
            ..opts()
        });
        assert_eq!(
            src.row(1).marker,
            Marker::Collapsed,
            "보기 변경 = 글리프 복귀"
        );
        // 설정을 다시 끄면 즉시 현행 복귀.
        src.set_opts(opts());
        assert_eq!(src.row(0).marker, Marker::Collapsed);
        // 열거 완료(펼침) 폴더는 프로브 없이 트리 실측 — 펼친 빈 폴더는 억제 켜짐에서 마커 없음.
        src.set_opts(ListOpts {
            hide_empty_glyph: true,
            ..opts()
        });
        assert!(src.expand_row(0), "빈 폴더 펼침");
        assert_eq!(
            src.row(0).marker,
            Marker::None,
            "열거 완료·가시 자식 0 = 억제(실측)"
        );
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn size_and_time_format() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(1023), "1023 B");
        assert_eq!(format_size(1024), "1.0 KB");
        assert_eq!(format_size(1_536_000), "1.5 MB");
        assert_eq!(format_time(0), "");
        // 2026-10-03 12:34:56 UTC
        assert_eq!(format_time(1_791_030_896_000), "2026-10-03 12:34");
        assert_eq!(format_iso_utc(1_791_030_896), "2026-10-03T12:34:56");
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(-1), (1969, 12, 31));
        assert_eq!(civil_from_days(10_957), (2000, 1, 1));
        assert_eq!(civil_from_days(11_016), (2000, 2, 29), "윤년");
    }

    #[test]
    fn opens_folder_dirs_first_and_toggles() {
        let dir = std::env::temp_dir().join(format!("ndir-filelist-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        std::fs::write(dir.join("a.txt"), b"hello").unwrap();
        std::fs::write(dir.join("sub/inner.rs"), b"x").unwrap();
        let mut src = TreeSource::open(&dir, opts());
        assert!(src.error().is_none());
        assert_eq!(src.len(), 2);
        let r0 = src.row(0);
        assert!(r0.is_dir && r0.text == "sub" && r0.marker == Marker::Collapsed);
        assert_eq!(src.row(1).text, "a.txt");
        assert_eq!(src.cell(1, COL_SIZE), "5 B");
        assert_eq!(src.cell(0, COL_SIZE), "");
        assert_eq!(src.cell(1, COL_KIND), "TXT");
        assert_eq!(src.cell(1, COL_EXT), "txt");
        assert_eq!(src.cell(0, COL_EXT), "");
        assert_eq!(title_of(&dir.join("sub")), "sub");
        // Unix 루트 = `/`(공백 아님) · 드라이브 루트 = `D:`.
        assert_eq!(title_of(Path::new("/")), "/");
        // 열 → 정렬 키: 확장자와 종류는 서로 다른 키 · 내 PC 열(전체 크기 · 여유 공간)은 정렬 대상이 아니다.
        assert_eq!(sort_key_of(COL_EXT), Some(SortKey::Ext));
        assert_eq!(sort_key_of(COL_KIND), Some(SortKey::Kind));
        assert_eq!(sort_key_of(COL_STATUS), Some(SortKey::Status));
        assert_eq!(sort_key_of(COL_TOTAL), None);
        #[cfg(windows)]
        assert_eq!(title_of(Path::new("D:\\")), "D:");
        assert_eq!(
            title_of(Path::new(ndir_vfs::MY_PC)),
            ndir_i18n::tr("nav.mypc")
        );
        assert_eq!(display_path(&dir), dir.to_string_lossy());
        assert!(src.toggle(0), "펼침");
        assert_eq!(src.len(), 3);
        assert_eq!(src.row(1).text, "inner.rs");
        assert_eq!(src.row(1).depth, 1);
        assert!(src.row_path(1).unwrap().ends_with("inner.rs"));
        assert!(src.toggle(0), "접힘");
        assert_eq!(src.len(), 2);
        // 선택
        assert!(src.select(1, SelectOp::Single));
        assert!(src.is_selected(1) && !src.is_selected(0));
        assert_eq!(src.selection_count(), 1);
        assert!(src.select_all());
        assert_eq!(src.selection_count(), 2);
        assert!(src.clear_selection());
        assert!(!src.clear_selection(), "이미 비었으면 false");
        // 타입어헤드
        assert_eq!(src.find_prefix(None, "a"), Some(1));
        // 정렬 키 대응
        assert_eq!(sort_key_of(COL_MODIFIED), Some(SortKey::Modified));
        assert_eq!(sort_key_of(99), None);
        assert!(src.set_sort(&[(COL_SIZE, true)]));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 내 PC 열: 용량을 넣으면 전체/여유 셀 + 타일 보조 줄 + 종류 = 드라이브 · 안 넣으면 빈 셀.
    #[test]
    fn drive_columns_use_injected_space() {
        ndir_i18n::activate(ndir_i18n::load("en", Path::new("nowhere")));
        let mut src = TreeSource::open(Path::new(ndir_vfs::MY_PC), opts());
        assert!(src.is_virtual_root());
        let names = src.drive_names();
        if names.is_empty() {
            return; // 비Windows: 드라이브 열거 없음(X-17) — 열 전환은 panel 시험이 본다.
        }
        assert_eq!(src.cell(0, COL_TOTAL), "");
        src.set_drive_space(vec![(names[0].clone(), (2048, 1024))]);
        assert!(src.drive_space_known());
        assert_eq!(src.cell(0, COL_TOTAL), "2.0 KB");
        assert_eq!(src.cell(0, COL_FREE), "1.0 KB");
        assert_eq!(src.cell(0, COL_KIND), "Drive");
        let (line, frac) = src.tile_info(0);
        assert_eq!(line, "1.0 KB free of 2.0 KB");
        assert_eq!(frac, Some(0.5));
    }

    #[test]
    fn missing_folder_is_empty_with_error() {
        let src = TreeSource::open(Path::new("Z:/definitely/not/here-ndir"), opts());
        assert_eq!(src.len(), 0);
        assert!(src.error().is_some());
        assert!(!src.status_text().is_empty());
        assert_eq!(src.row(0).text, "");
    }

    /// GAP-006: 바로 가기 확장자 숨김/복원(순수 · MC/DC — 숨기는 OS × 폴더 여부 × 확장자 유무).
    #[test]
    fn shortcut_ext_is_hidden_and_restored_on_rename() {
        assert_eq!(split_shortcut_ext("Chrome.lnk"), Some(("Chrome", ".lnk")));
        assert_eq!(split_shortcut_ext("Site.URL"), Some(("Site", ".URL")));
        assert_eq!(
            split_shortcut_ext("App.appref-ms"),
            Some(("App", ".appref-ms"))
        );
        assert_eq!(split_shortcut_ext(".lnk"), None, "이름이 확장자뿐");
        assert_eq!(split_shortcut_ext("메모.txt"), None);
        assert_eq!(display_name("Chrome.lnk", false, true), "Chrome");
        assert_eq!(
            display_name("Chrome.lnk", false, false),
            "Chrome.lnk",
            "숨기지 않는 OS"
        );
        assert_eq!(
            display_name("folder.lnk", true, true),
            "folder.lnk",
            "폴더는 그대로"
        );
        assert_eq!(
            display_name("한글 바로 가기.lnk", false, true),
            "한글 바로 가기"
        );
        assert_eq!(
            restore_shortcut_ext("Chrome.lnk", "Browser", false, true),
            "Browser.lnk"
        );
        assert_eq!(
            restore_shortcut_ext("Chrome.lnk", "Browser.LNK", false, true),
            "Browser.LNK"
        );
        assert_eq!(
            restore_shortcut_ext("Chrome.lnk", "Browser", false, false),
            "Browser"
        );
        assert_eq!(restore_shortcut_ext("a.txt", "b", false, true), "b");
        assert_eq!(restore_shortcut_ext("d.lnk", "e", true, true), "e");
        assert_eq!(restore_shortcut_ext("Chrome.lnk", "", false, true), "");
        // 종류 열(GAP-008): 바로 가기 파일 = "바로 가기"(확장자를 숨기는 OS) · 그 밖 = 확장자 대문자.
        let link = ndir_i18n::tr("kind.link");
        if crate::platform::hides_shortcut_ext() {
            assert_eq!(kind_label(FileKind::File, "Chrome.lnk"), link);
            assert_eq!(kind_label(FileKind::File, "Site.url"), link);
        } else {
            assert_eq!(kind_label(FileKind::File, "Chrome.lnk"), "LNK");
        }
        assert_eq!(kind_label(FileKind::File, "a.txt"), "TXT");
        assert_eq!(kind_label(FileKind::Symlink, "x"), link);
    }

    /// 현지 시각 표시: UTC 서식과의 차이가 그 순간의 OS 시간대 오프셋(분 단위 · DST 포함)과 같다 · 0 = 빈 값.
    #[test]
    fn local_time_display_follows_os_zone() {
        assert_eq!(format_time_local(0), "");
        let ms = 1_791_030_896_000i64; // 2026-10-03 12:34:56 UTC
        let shown = format_time_local(ms);
        let t = std::time::UNIX_EPOCH + std::time::Duration::from_millis(ms as u64);
        let l = nexa_fs::local_time(t);
        assert_eq!(
            shown,
            format!(
                "{:04}-{:02}-{:02} {:02}:{:02}",
                l.year, l.month, l.day, l.hour, l.min
            )
        );
        // 오프셋이 0이 아닌 시간대(한국 +9)에서는 UTC 서식과 달라야 한다 · 0인 곳에서는 같다.
        let off = crate::app::bulk::tz_min_from(
            ms / 1000,
            (l.year, l.month, l.day),
            (l.hour, l.min, l.sec),
        );
        assert_eq!(shown == format_time(ms), off == 0, "offset {off}");
    }
}

#[cfg(test)]
mod probe_tests {
    use super::*;

    #[test]
    fn probe_worker_fills_cache_and_ignores_stale_generation() {
        let dir = std::env::temp_dir().join(format!("ndir-probe-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("empty")).unwrap();
        std::fs::create_dir_all(dir.join("full")).unwrap();
        std::fs::write(dir.join("full/a.txt"), b"x").unwrap();
        let opts = ListOpts {
            show_hidden: false,
            show_dotfiles: false,
            show_protected: false,
            folders_first: true,
            case_sensitive: false,
            natural: true,
            hide_empty_glyph: true,
            nfc_display: false,
        };
        let mut src = TreeSource::open(&dir, opts);
        let ids: Vec<(String, ndir_tree::NodeId)> = (0..src.len())
            .filter_map(|i| {
                let r = src.tree.as_ref()?.row(i)?;
                Some((r.name.clone(), r.id))
            })
            .collect();
        let id_of = |n: &str| {
            ids.iter()
                .find(|(name, _)| name == n)
                .map(|(_, id)| *id)
                .unwrap()
        };
        let flags = (false, false, false);
        for n in ["empty", "full"] {
            let id = id_of(n);
            src.probe_pending.borrow_mut().insert(id);
            probe::submit(probe::Job {
                gen: src.probe_gen,
                id,
                path: dir.join(n),
                flags,
                inbox: std::sync::Arc::clone(&src.probe_inbox),
            });
        }
        // 낡은 세대 결과는 버려진다.
        probe::submit(probe::Job {
            gen: src.probe_gen + 99,
            id: id_of("empty"),
            path: dir.join("full"),
            flags,
            inbox: std::sync::Arc::clone(&src.probe_inbox),
        });
        assert!(src.probes_pending());
        let at = std::time::Instant::now();
        while src.probes_pending() && at.elapsed() < std::time::Duration::from_secs(5) {
            let _ = src.drain_probes();
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let _ = src.drain_probes();
        assert!(!src.probes_pending());
        let cache = src.probe_cache.borrow();
        assert_eq!(
            cache.get(&id_of("empty")),
            Some(&false),
            "빈 폴더 = 글리프 없음"
        );
        assert_eq!(cache.get(&id_of("full")), Some(&true));
        drop(cache);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
