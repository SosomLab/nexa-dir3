//! X11 **XDND 드래그 발신**(T-147 Linux 2차 · XDND 프로토콜 v5 · x11rb — 추가 crate 0 · DR-8): 선택한 파일을 다른 프로그램
//! (노틸러스 · 돌핀 · 터미널 · 편집기 · 다른 dir3 창)으로 끌어다 놓는다. 앱 창은 `DISPLAY`가 있으면 늘 X11(XWayland 포함 · main.rs)이고,
//! Wayland 네이티브 창(노틸러스)과는 mutter의 XDND ↔ Wayland 다리가 가운데서 통역한다(포인터 아래에 XdndAware 프록시 창을 둔다).
//!
//! - **비모달**(10-10 사용자 실기 "pointer grab failed: ALREADY_GRABBED"): 버튼을 누른 순간 X 서버가 winit 연결에 **암시적 잡기**를
//!   걸어 두므로 따로 연 연결은 포인터를 잡을 수 없다(windrag처럼 모달로 돌 수 없다). 대신 포인터 사건은 winit이 계속 받으므로
//!   (암시적 잡기 = 창 밖에서도 MotionNotify/ButtonRelease가 온다) **창 안 드래그**(`App::dnd_internal_*`)가 그 사건을 받고, 이 모듈의
//!   [`Session`]은 그 위에 XDND 메시지(Enter/Position/Leave/Drop)만 얹는다. 대상의 응답(XdndStatus · XdndFinished)과 데이터 요청
//!   (SelectionRequest)은 **호스트 틱**이 [`Session::tick`]으로 폴링한다(자체 스레드 · 타이머 없음 · docs/01 §3).
//! - **대상 찾기**: 루트부터 포인터 아래 자식으로 내려가며 `XdndAware`가 처음 붙은 창(WM 프레임 안의 클라이언트 창). 자식 추적이
//!   비면 기하(창 사각형) 보완. 우리 자신의 창은 대상이 아니다(창 안 드래그가 처리).
//! - **제안 동작**: Ctrl = 복사 · Shift = 이동 · 그 밖 = 복사(대상이 `XdndActionList` = [복사, 이동]을 보고 고른다). 돌아온 동작이
//!   이동이어도 **원본을 지우지 않는다**(windrag와 같은 안전한 쪽 · 호출부는 결과와 무관하게 다시 읽는다).
//! - **데이터**: `text/uri-list`(`file:///…` 줄마다 CRLF) · `text/plain` · `UTF8_STRING`(경로를 줄로) — 터미널은 글로 받는다.
//! - **시험**: 순수 부분(대상 탐색 · 메시지 패킹 · 동작 매핑 · URI 목록)은 늘 · 실제 X 서버 왕복은 옵트인(`NDIR_XDND_TEST=1` —
//!   가짜 XDND 대상 창 ← [`inject_drop`] 프로토콜 소스 · [`Session`]을 합성 포인터로 몰기) · 수신은 T4 `xdnd-drop.scn`(`xdnd.drop` 기동 명령).

use super::DragOutcome;
use super::DropChoice;
use std::cell::RefCell;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use x11rb::connection::Connection;
use x11rb::protocol::xproto::{
    Atom, AtomEnum, ClientMessageEvent, ConnectionExt, CreateWindowAux, EventMask, KeyButMask,
    MapState, PropMode, SelectionNotifyEvent, SelectionRequestEvent, Window, WindowClass,
    CLIENT_MESSAGE_EVENT, SELECTION_NOTIFY_EVENT,
};
use x11rb::protocol::Event;
use x11rb::rust_connection::RustConnection;
use x11rb::wrapper::ConnectionExt as _;
use x11rb::{CURRENT_TIME, NONE};

/// 우리가 말하는 XDND 판(대상이 더 낮으면 그쪽에 맞춘다 · 3 미만은 모르는 창으로 본다).
pub(crate) const XDND_VERSION: u32 = 5;
/// 놓은 뒤 `XdndFinished`를 기다리는 상한(대상이 안 보내도 돌아온다).
const FINISH_TIMEOUT: Duration = Duration::from_secs(3);
/// 세션이 살아 있는 동안 호스트 틱의 폴링 간격.
pub(crate) const TICK_MS: u64 = 10;

// ───────────────────────── 순수 부분(시험) ─────────────────────────

/// 포인터 아래의 XDND 대상(순수): 루트에서 시작해 포인터 아래 자식으로 내려가며 **처음** `XdndAware`(판 ≥ 3)가 붙은 창.
/// `query(창) -> (포인터 아래 자식, 그 창의 XdndAware 판)` · 돌려주는 값 = (창, 판 — 우리 판과의 최솟값). 없으면 `None`.
pub(crate) fn find_target(
    root: Window,
    query: &mut dyn FnMut(Window) -> (Option<Window>, u32),
) -> Option<(Window, u32)> {
    let mut w = root;
    for _ in 0..64 {
        let (child, version) = query(w);
        if w != root && version >= 3 {
            return Some((w, version.min(XDND_VERSION)));
        }
        w = child?;
    }
    None
}

/// `XdndEnter` data[1](순수): 판은 상위 바이트 · bit0 = 종류가 3개를 넘어 `XdndTypeList`를 봐야 함(우리는 늘 3개 → 0).
pub(crate) fn enter_flags(version: u32, more_than_three: bool) -> u32 {
    (version << 24) | u32::from(more_than_three)
}

/// `XdndPosition` data[2](순수): 루트 좌표 (x << 16) | y.
pub(crate) fn pack_pos(x: i16, y: i16) -> u32 {
    ((x as u16 as u32) << 16) | (y as u16 as u32)
}

/// 제안 동작(순수): Ctrl = 복사 · Shift = 이동 · 그 밖 = 복사(대상이 목록에서 고른다).
pub(crate) fn proposed(ctrl: bool, shift: bool) -> DropChoice {
    if ctrl || !shift {
        DropChoice::Copy
    } else {
        DropChoice::Move
    }
}

/// 결과(순수): 놓지 않았거나 대상이 거부 = 취소 · 받았고 동작이 이동 = 이동 · 그 밖 = 복사.
pub(crate) fn outcome(dropped: bool, accepted: bool, moved: bool) -> DragOutcome {
    match (dropped && accepted, moved) {
        (false, _) => DragOutcome::Cancelled,
        (true, true) => DragOutcome::Moved,
        (true, false) => DragOutcome::Copied,
    }
}

/// `text/uri-list`(순수 · RFC 2483): 줄마다 `file:///…` + CRLF.
pub(crate) fn uri_list(paths: &[PathBuf]) -> String {
    let mut s = String::new();
    for p in paths {
        s.push_str(&crate::clipboard_x11::file_uri(p));
        s.push_str("\r\n");
    }
    s
}

/// `text/plain`(순수): 경로를 줄로(터미널 · 편집기가 글로 받는 경우).
pub(crate) fn plain_list(paths: &[PathBuf]) -> String {
    paths
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("\n")
}

// ───────────────────────── X11 교환 ─────────────────────────

struct Atoms {
    aware: Atom,
    selection: Atom,
    enter: Atom,
    position: Atom,
    status: Atom,
    leave: Atom,
    drop: Atom,
    finished: Atom,
    action_copy: Atom,
    action_move: Atom,
    action_list: Atom,
    uri_list: Atom,
    text_plain: Atom,
    utf8: Atom,
    targets: Atom,
    net_wm_pid: Atom,
}

impl Atoms {
    fn intern(conn: &RustConnection) -> Result<Atoms, Box<dyn std::error::Error>> {
        let names: [&[u8]; 16] = [
            b"XdndAware",
            b"XdndSelection",
            b"XdndEnter",
            b"XdndPosition",
            b"XdndStatus",
            b"XdndLeave",
            b"XdndDrop",
            b"XdndFinished",
            b"XdndActionCopy",
            b"XdndActionMove",
            b"XdndActionList",
            b"text/uri-list",
            b"text/plain",
            b"UTF8_STRING",
            b"TARGETS",
            b"_NET_WM_PID",
        ];
        let cookies: Vec<_> = names
            .iter()
            .map(|n| conn.intern_atom(false, n))
            .collect::<Result<_, _>>()?;
        let mut a = [0 as Atom; 16];
        for (slot, c) in a.iter_mut().zip(cookies) {
            *slot = c.reply()?.atom;
        }
        Ok(Atoms {
            aware: a[0],
            selection: a[1],
            enter: a[2],
            position: a[3],
            status: a[4],
            leave: a[5],
            drop: a[6],
            finished: a[7],
            action_copy: a[8],
            action_move: a[9],
            action_list: a[10],
            uri_list: a[11],
            text_plain: a[12],
            utf8: a[13],
            targets: a[14],
            net_wm_pid: a[15],
        })
    }

    fn action_of(&self, choice: DropChoice) -> Atom {
        match choice {
            DropChoice::Move => self.action_move,
            DropChoice::Copy => self.action_copy,
            DropChoice::None => NONE,
        }
    }
}

/// 대상 창의 상태.
struct Target {
    win: Window,
    version: u32,
    /// 마지막 `XdndStatus`: (받음, 동작).
    accepted: bool,
    action: Atom,
    /// `XdndStatus`를 기다리는 중(그동안 온 위치는 하나만 보관해 뒤에 보낸다 — 프로토콜 규약).
    awaiting_status: bool,
    pending: Option<(i16, i16, u32, DropChoice)>,
}

struct Payload {
    uri_list: Vec<u8>,
    plain: Vec<u8>,
}

/// 소스 쪽 X 자원(연결 · 아톰 · 헬퍼 창 · 데이터) — 세션과 주입이 같이 쓴다.
struct Source {
    conn: RustConnection,
    root: Window,
    at: Atoms,
    win: Window,
    data: Payload,
}

impl Source {
    /// 연결 → 1×1 InputOnly 헬퍼 창(`XdndSelection` 주인 · 메시지의 소스 창) → `XdndActionList` [복사, 이동].
    fn open(paths: &[PathBuf]) -> Result<Source, Box<dyn std::error::Error>> {
        let (conn, screen_n) = x11rb::connect(None)?;
        let root = conn.setup().roots[screen_n].root;
        let at = Atoms::intern(&conn)?;
        let win = conn.generate_id()?;
        conn.create_window(
            x11rb::COPY_DEPTH_FROM_PARENT,
            win,
            root,
            0,
            0,
            1,
            1,
            0,
            WindowClass::INPUT_ONLY,
            x11rb::COPY_FROM_PARENT,
            &CreateWindowAux::new().event_mask(EventMask::PROPERTY_CHANGE),
        )?
        .check()?;
        conn.change_property32(
            PropMode::REPLACE,
            win,
            at.action_list,
            AtomEnum::ATOM,
            &[at.action_copy, at.action_move],
        )?;
        conn.set_selection_owner(win, at.selection, CURRENT_TIME)?
            .check()?;
        Ok(Source {
            conn,
            root,
            at,
            win,
            data: Payload {
                uri_list: uri_list(paths).into_bytes(),
                plain: plain_list(paths).into_bytes(),
            },
        })
    }

    fn send(
        &self,
        to: Window,
        type_: Atom,
        data: [u32; 5],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let ev = ClientMessageEvent {
            response_type: CLIENT_MESSAGE_EVENT,
            format: 32,
            sequence: 0,
            window: to,
            type_,
            data: data.into(),
        };
        self.conn.send_event(false, to, EventMask::NO_EVENT, ev)?;
        self.conn.flush()?;
        Ok(())
    }

    fn position(
        &self,
        t: &mut Target,
        x: i16,
        y: i16,
        time: u32,
        choice: DropChoice,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.send(
            t.win,
            self.at.position,
            [self.win, 0, pack_pos(x, y), time, self.at.action_of(choice)],
        )?;
        t.awaiting_status = true;
        Ok(())
    }

    fn leave(&self, t: &Target) {
        let _ = self.send(t.win, self.at.leave, [self.win, 0, 0, 0, 0]);
    }

    /// 대상이 데이터를 달라고 한다(`XdndSelection`): TARGETS · 세 종류 · 그 밖은 거절.
    fn serve(&self, r: &SelectionRequestEvent) -> Result<(), Box<dyn std::error::Error>> {
        let at = &self.at;
        let property = if r.property == NONE {
            r.target
        } else {
            r.property
        };
        let ok = if r.target == at.targets {
            self.conn.change_property32(
                PropMode::REPLACE,
                r.requestor,
                property,
                AtomEnum::ATOM,
                &[at.targets, at.uri_list, at.text_plain, at.utf8],
            )?;
            true
        } else if r.target == at.uri_list {
            self.conn.change_property8(
                PropMode::REPLACE,
                r.requestor,
                property,
                r.target,
                &self.data.uri_list,
            )?;
            true
        } else if r.target == at.text_plain || r.target == at.utf8 {
            self.conn.change_property8(
                PropMode::REPLACE,
                r.requestor,
                property,
                r.target,
                &self.data.plain,
            )?;
            true
        } else {
            false
        };
        let notify = SelectionNotifyEvent {
            response_type: SELECTION_NOTIFY_EVENT,
            sequence: 0,
            time: r.time,
            requestor: r.requestor,
            selection: r.selection,
            target: r.target,
            property: if ok { property } else { NONE },
        };
        self.conn
            .send_event(false, r.requestor, EventMask::NO_EVENT, notify)?;
        self.conn.flush()?;
        Ok(())
    }

    /// 창 하나를 묻는다: (포인터 아래 자식, XdndAware 판).
    fn probe(&self, w: Window) -> (Option<Window>, u32) {
        let child = self
            .conn
            .query_pointer(w)
            .ok()
            .and_then(|c| c.reply().ok())
            .map(|r| r.child)
            .filter(|c| *c != NONE);
        (child, self.aware(w))
    }

    fn aware(&self, w: Window) -> u32 {
        self.conn
            .get_property(false, w, self.at.aware, AtomEnum::ATOM, 0, 1)
            .ok()
            .and_then(|c| c.reply().ok())
            .and_then(|p| p.value32().and_then(|mut it| it.next()))
            .unwrap_or(0)
    }

    fn pid_of(&self, w: Window) -> Option<u32> {
        self.conn
            .get_property(false, w, self.at.net_wm_pid, AtomEnum::CARDINAL, 0, 1)
            .ok()
            .and_then(|c| c.reply().ok())
            .and_then(|p| p.value32().and_then(|mut it| it.next()))
    }

    /// 자식 추적이 비었을 때의 보완(XWayland: `WarpPointer`로 옮긴 자리는 컴포지터 포인터와 달라 `QueryPointer.child`가 0 — 합성
    /// 입력 시험 · 10-10): 루트의 자식들을 **위에서부터** 보며 보이는 창의 사각형이 `(x, y)`를 품으면 그 창(과 그 아래)에서 첫 XdndAware.
    fn find_by_geometry(&self, x: i16, y: i16) -> Option<(Window, u32)> {
        fn aware_in(s: &Source, w: Window, depth: u8) -> Option<(Window, u32)> {
            let version = s.aware(w);
            if version >= 3 {
                return Some((w, version.min(XDND_VERSION)));
            }
            if depth >= 4 {
                return None;
            }
            let kids = s.conn.query_tree(w).ok()?.reply().ok()?.children;
            kids.iter().rev().find_map(|c| aware_in(s, *c, depth + 1))
        }
        let kids = self.conn.query_tree(self.root).ok()?.reply().ok()?.children;
        for w in kids.iter().rev() {
            let Ok(attrs) = self.conn.get_window_attributes(*w).ok()?.reply() else {
                continue;
            };
            if attrs.map_state != MapState::VIEWABLE {
                continue;
            }
            let Ok(g) = self.conn.get_geometry(*w).ok()?.reply() else {
                continue;
            };
            let (gx, gy) = (i32::from(g.x), i32::from(g.y));
            let inside = i32::from(x) >= gx
                && i32::from(x) < gx + i32::from(g.width)
                && i32::from(y) >= gy
                && i32::from(y) < gy + i32::from(g.height);
            if inside {
                if let Some(t) = aware_in(self, *w, 0) {
                    return Some(t);
                }
            }
        }
        None
    }

    /// 포인터 아래의 외부 대상(우리 창 · 우리 프로세스 창은 제외).
    fn target_at(&self, x: i16, y: i16, own: Option<Window>, my_pid: u32) -> Option<(Window, u32)> {
        let found = find_target(self.root, &mut |w| self.probe(w))
            .or_else(|| self.find_by_geometry(x, y))?;
        if Some(found.0) == own || self.pid_of(found.0) == Some(my_pid) {
            return None;
        }
        Some(found)
    }

    fn close(&self) {
        let _ = self.conn.destroy_window(self.win);
        let _ = self.conn.flush();
    }
}

/// **비모달 발신 세션** — 창 안 드래그가 받는 포인터 사건(루트 좌표)을 [`Self::motion`] · [`Self::release`] · [`Self::cancel`]로
/// 넣고, 호스트 틱이 [`Self::tick`]으로 응답을 거둔다. 끝나면 `tick`이 결과를 한 번 돌려준다.
pub(crate) struct Session {
    src: Source,
    own: Option<Window>,
    my_pid: u32,
    target: Option<Target>,
    dropped: bool,
    finished: Option<(bool, Atom)>,
    deadline: Option<Instant>,
    done: Option<DragOutcome>,
}

impl Session {
    /// 드래그 시작(버튼을 누른 채 임계를 넘은 순간). `own` = 우리 메인 창(X11 id · 대상에서 제외).
    pub(crate) fn start(paths: &[PathBuf], own: Option<u32>) -> Result<Session, String> {
        if paths.is_empty() {
            return Err("empty drag".into());
        }
        let src = Source::open(paths).map_err(|e| format!("xdnd: {e}"))?;
        Ok(Session {
            src,
            own,
            my_pid: std::process::id(),
            target: None,
            dropped: false,
            finished: None,
            deadline: None,
            done: None,
        })
    }

    /// 포인터가 움직였다(루트 좌표 · 수식키). 외부 대상이 바뀌면 Leave/Enter · 같으면 Position(Status 대기 중이면 보류).
    pub(crate) fn motion(&mut self, x: i16, y: i16, ctrl: bool, shift: bool) {
        if self.dropped || self.done.is_some() {
            return;
        }
        let found = self.src.target_at(x, y, self.own, self.my_pid);
        let same = matches!((&self.target, found), (Some(t), Some((w, _))) if t.win == w);
        if !same {
            if let Some(t) = self.target.take() {
                self.src.leave(&t);
            }
            if let Some((w, version)) = found {
                let at = &self.src.at;
                let _ = self.src.send(
                    w,
                    at.enter,
                    [
                        self.src.win,
                        enter_flags(version, false),
                        at.uri_list,
                        at.text_plain,
                        at.utf8,
                    ],
                );
                self.target = Some(Target {
                    win: w,
                    version,
                    accepted: false,
                    action: NONE,
                    awaiting_status: false,
                    pending: None,
                });
            }
        }
        if let Some(t) = self.target.as_mut() {
            let choice = proposed(ctrl, shift);
            if t.awaiting_status {
                t.pending = Some((x, y, CURRENT_TIME, choice));
            } else {
                let _ = self.src.position(t, x, y, CURRENT_TIME, choice);
            }
        }
    }

    /// 외부 대상이 지금 받겠다고 했는가(창 안 드래그가 자기 드롭을 양보할지 판정).
    pub(crate) fn external_accepts(&self) -> bool {
        self.target.as_ref().is_some_and(|t| t.accepted)
    }

    /// 버튼을 뗐다 — 외부 대상이 받겠다고 했으면 `XdndDrop`을 보내고 `true`(이후 `tick`이 Finished까지 본다) · 아니면 Leave + 취소로
    /// 끝내고 `false`(호출부가 창 안 드롭을 처리).
    pub(crate) fn release(&mut self) -> bool {
        if self.dropped || self.done.is_some() {
            return false;
        }
        match self.target.as_ref() {
            Some(t) if t.accepted => {
                self.dropped = true;
                let _ = self.src.send(
                    t.win,
                    self.src.at.drop,
                    [self.src.win, 0, CURRENT_TIME, 0, 0],
                );
                self.deadline = Some(Instant::now() + FINISH_TIMEOUT);
                true
            }
            Some(t) => {
                self.src.leave(t);
                self.target = None;
                self.finish(DragOutcome::Cancelled);
                false
            }
            None => {
                self.finish(DragOutcome::Cancelled);
                false
            }
        }
    }

    /// 취소(Esc · 포커스 잃음).
    pub(crate) fn cancel(&mut self) {
        if self.done.is_some() {
            return;
        }
        if let Some(t) = self.target.take() {
            if !self.dropped {
                self.src.leave(&t);
            }
        }
        self.finish(DragOutcome::Cancelled);
    }

    fn finish(&mut self, out: DragOutcome) {
        if self.done.is_none() {
            self.done = Some(out);
            self.src.close();
        }
    }

    /// 틱 — 대상의 응답 · 데이터 요청을 거둔다. 세션이 끝났으면 결과를 **한 번** 돌려준다(그 뒤는 `None` · 호출부가 버린다).
    pub(crate) fn tick(&mut self) -> Option<DragOutcome> {
        if let Some(out) = self.done {
            return Some(out);
        }
        loop {
            let ev = match self.src.conn.poll_for_event() {
                Ok(Some(ev)) => ev,
                Ok(None) => break,
                Err(_) => {
                    self.finish(DragOutcome::Cancelled);
                    return self.done;
                }
            };
            let at = &self.src.at;
            match ev {
                Event::ClientMessage(cm) if cm.type_ == at.status => {
                    let d = cm.data.as_data32();
                    if let Some(t) = self.target.as_mut().filter(|t| t.win == d[0]) {
                        t.accepted = d[1] & 1 != 0;
                        t.action = if t.accepted { d[4] } else { NONE };
                        t.awaiting_status = false;
                        if let Some((x, y, time, choice)) = t.pending.take() {
                            let _ = self.src.position(t, x, y, time, choice);
                        }
                    }
                }
                Event::ClientMessage(cm) if cm.type_ == at.finished => {
                    let d = cm.data.as_data32();
                    let (accepted, action) = match self.target.as_ref() {
                        Some(t) if t.version >= 5 => (d[1] & 1 != 0, d[2]),
                        Some(t) => (t.accepted, t.action),
                        None => (false, NONE),
                    };
                    self.finished = Some((accepted, action));
                    if self.dropped {
                        break;
                    }
                }
                Event::SelectionRequest(r) if r.selection == at.selection => {
                    let _ = self.src.serve(&r);
                }
                _ => {}
            }
        }
        if self.dropped {
            let timed_out = self.deadline.is_some_and(|dl| Instant::now() >= dl);
            if self.finished.is_some() || timed_out {
                let (accepted, action) = self
                    .finished
                    .or_else(|| self.target.as_ref().map(|t| (t.accepted, t.action)))
                    .unwrap_or((false, NONE));
                let out = outcome(true, accepted, action == self.src.at.action_move);
                self.finish(out);
                return Some(out);
            }
        }
        None
    }
}

// ───────────────────────── 포인터 조회(수신 쪽) ─────────────────────────

thread_local! {
    /// 포인터 조회용 연결(틱마다 다시 잇지 않게 · 끊기면 버리고 다시).
    static PTR_CONN: RefCell<Option<(RustConnection, Window)>> = const { RefCell::new(None) };
}

/// 화면 좌표의 포인터 자리 + Ctrl/Shift(X11 `QueryPointer` · XWayland 포함 · 10-10). X 연결이 없으면 `None`(Wayland 네이티브).
/// 외부 드래그가 들어와 있는 동안 호스트 틱이 부른다(`event_loop.rs` — 종전 Linux = `None`이라 놓을 자리가 드래그 전 마지막
/// 포인터 자리였다 → 폴더 행이 아니라 패널 폴더로 떨어졌다 · 사용자 10-10 실기).
pub(super) fn pointer_state() -> Option<super::PointerState> {
    PTR_CONN.with(|c| {
        let mut slot = c.borrow_mut();
        if slot.is_none() {
            let (conn, n) = x11rb::connect(None).ok()?;
            let root = conn.setup().roots[n].root;
            *slot = Some((conn, root));
        }
        let reply = {
            let (conn, root) = slot.as_ref()?;
            conn.query_pointer(*root).ok().and_then(|c| c.reply().ok())
        };
        let Some(r) = reply else {
            *slot = None; // 연결이 끊겼다 — 다음에 다시 잇는다
            return None;
        };
        let (ctrl, shift) = mods(r.mask);
        Some(super::PointerState {
            x: i32::from(r.root_x),
            y: i32::from(r.root_y),
            ctrl,
            shift,
        })
    })
}

fn mods(state: KeyButMask) -> (bool, bool) {
    (
        state.contains(KeyButMask::CONTROL),
        state.contains(KeyButMask::SHIFT),
    )
}

// ───────────────────────── 시험 주입(수신 쪽 자동화) ─────────────────────────

/// 시험 자동화(기동 명령 `xdnd.drop` · T4 `xdnd-drop.scn`): 이 프로세스의 **다른 X 연결**이 XDND 소스가 되어 `target` 창의
/// 루트 좌표 `at`에 `paths`를 놓는다 — 포인터 잡기 없이 **프로토콜만**(Enter → Position → Status 대기 → Drop → Finished 대기 ·
/// 그동안 SelectionRequest 응대). 먼저 포인터를 그 자리로 옮겨(`WarpPointer`) 수신 쪽 `pointer_state`가 같은 자리를 읽게 한다.
/// 스레드로 돌고 결과는 `JoinHandle`(`Err` = 대상이 거부/무응답).
pub(crate) fn inject_drop(
    target: u32,
    at: (i16, i16),
    paths: Vec<PathBuf>,
) -> std::thread::JoinHandle<Result<(), String>> {
    std::thread::Builder::new()
        .name("ndir-xdnd-inject".into())
        .spawn(move || inject_drop_blocking(target, at, &paths).map_err(|e| e.to_string()))
        .expect("spawn xdnd inject thread")
}

fn inject_drop_blocking(
    target: Window,
    at: (i16, i16),
    paths: &[PathBuf],
) -> Result<(), Box<dyn std::error::Error>> {
    const WAIT: Duration = Duration::from_secs(3);
    let src = Source::open(paths)?;
    let version = src.aware(target);
    if version < 3 {
        src.close();
        return Err(format!("target {target:#x} is not XdndAware").into());
    }
    let version = version.min(XDND_VERSION);
    src.conn
        .warp_pointer(NONE, src.root, 0, 0, 0, 0, at.0, at.1)?;
    src.conn.flush()?;
    let a = &src.at;
    src.send(
        target,
        a.enter,
        [
            src.win,
            enter_flags(version, false),
            a.uri_list,
            a.text_plain,
            a.utf8,
        ],
    )?;
    src.send(
        target,
        a.position,
        [
            src.win,
            0,
            pack_pos(at.0, at.1),
            CURRENT_TIME,
            a.action_copy,
        ],
    )?;
    // 받겠다는 XdndStatus까지(그동안 데이터 요청 응대 — winit은 Position 때 uri-list를 미리 읽는다).
    let mut accepted = None;
    let deadline = Instant::now() + WAIT;
    while accepted.is_none() {
        if Instant::now() >= deadline {
            let _ = src.send(target, a.leave, [src.win, 0, 0, 0, 0]);
            src.close();
            return Err("no XdndStatus from target".into());
        }
        match src.conn.poll_for_event()? {
            Some(Event::ClientMessage(cm)) if cm.type_ == a.status => {
                accepted = Some(cm.data.as_data32()[1] & 1 != 0);
            }
            Some(Event::SelectionRequest(r)) if r.selection == a.selection => src.serve(&r)?,
            Some(_) => {}
            None => std::thread::sleep(Duration::from_millis(5)),
        }
    }
    if accepted != Some(true) {
        let _ = src.send(target, a.leave, [src.win, 0, 0, 0, 0]);
        src.close();
        return Err("target rejected the drop".into());
    }
    src.send(target, a.drop, [src.win, 0, CURRENT_TIME, 0, 0])?;
    let deadline = Instant::now() + WAIT;
    let result = loop {
        if Instant::now() >= deadline {
            break Err("no XdndFinished from target".into());
        }
        match src.conn.poll_for_event()? {
            Some(Event::ClientMessage(cm)) if cm.type_ == a.finished => break Ok(()),
            Some(Event::SelectionRequest(r)) if r.selection == a.selection => src.serve(&r)?,
            Some(_) => {}
            None => std::thread::sleep(Duration::from_millis(5)),
        }
    };
    src.close();
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 대상 탐색(순수): 루트 → 프레임 → 클라이언트(XdndAware) 순으로 내려가 처음 aware 창 · 판은 우리 판과 최솟값 · 루트 자체는
    /// 대상이 아니다 · 아무 창도 aware가 아니면 없음 · 판 3 미만은 모르는 창.
    #[test]
    fn find_target_walks_down_to_first_aware_window() {
        // 1 = 루트(aware 표시가 있어도 무시) → 10 = WM 프레임 → 11 = 클라이언트(판 4) → 12 = 자식(판 5 · 도달 안 함)
        let tree = |w: Window| -> (Option<Window>, u32) {
            match w {
                1 => (Some(10), 5),
                10 => (Some(11), 0),
                11 => (Some(12), 4),
                12 => (None, 5),
                _ => (None, 0),
            }
        };
        assert_eq!(find_target(1, &mut { tree }), Some((11, 4)));
        let newer = |w: Window| -> (Option<Window>, u32) {
            match w {
                1 => (Some(2), 0),
                2 => (None, 7),
                _ => (None, 0),
            }
        };
        assert_eq!(
            find_target(1, &mut { newer }),
            Some((2, XDND_VERSION)),
            "판은 우리 쪽으로 자른다"
        );
        let old = |w: Window| -> (Option<Window>, u32) {
            match w {
                1 => (Some(2), 0),
                2 => (Some(3), 2),
                _ => (None, 0),
            }
        };
        assert_eq!(
            find_target(1, &mut { old }),
            None,
            "판 3 미만 · 그 아래 없음"
        );
        let mut loops = |_: Window| -> (Option<Window>, u32) { (Some(9), 0) };
        assert_eq!(find_target(1, &mut loops), None, "순환 방어(깊이 상한)");
    }

    #[test]
    fn packing_actions_outcome_and_lists_are_pure() {
        assert_eq!(enter_flags(5, false), 5 << 24);
        assert_eq!(enter_flags(4, true), (4 << 24) | 1);
        assert_eq!(pack_pos(100, 7), (100 << 16) | 7);
        assert_eq!(
            pack_pos(-1, 2),
            (0xFFFF << 16) | 2,
            "음수 루트 좌표도 16비트 그대로"
        );
        assert_eq!(
            proposed(false, false),
            DropChoice::Copy,
            "수식키 없음 = 복사 제안(대상이 고른다)"
        );
        assert_eq!(proposed(true, true), DropChoice::Copy, "Ctrl 우선");
        assert_eq!(proposed(false, true), DropChoice::Move);
        assert_eq!(outcome(false, true, true), DragOutcome::Cancelled);
        assert_eq!(
            outcome(true, false, true),
            DragOutcome::Cancelled,
            "대상 거부 = 취소"
        );
        assert_eq!(outcome(true, true, true), DragOutcome::Moved);
        assert_eq!(outcome(true, true, false), DragOutcome::Copied);
        let paths = vec![PathBuf::from("/tmp/a b"), PathBuf::from("/tmp/한글.txt")];
        let u = uri_list(&paths);
        assert_eq!(
            u,
            "file:///tmp/a%20b\r\nfile:///tmp/%ED%95%9C%EA%B8%80.txt\r\n"
        );
        assert_eq!(plain_list(&paths), "/tmp/a b\n/tmp/한글.txt");
    }

    /// 실제 X 서버 왕복(옵트인 `NDIR_XDND_TEST=1` + `DISPLAY` · XWayland 포함 · 화면에 80×80 검은 창이 잠깐 뜬다): 가짜 XDND 대상 창을
    /// 만들어 ① `inject_drop`(프로토콜 소스)이 놓은 uri-list를 받는지 ② [`Session`](실제 발신부)을 합성 포인터(루트 좌표 · 포인터
    /// Warp)로 몰았을 때 Enter → Position → Status → Drop → Finished가 돌고 결과가 이동(제안 그대로)인지 ③ 대상 밖에서 떼면 취소인지
    /// 본다. 서버가 없으면 건너뛴다(CI `cargo test`는 화면 없음).
    #[test]
    fn x11_roundtrip_with_fake_target_opt_in() {
        use std::sync::mpsc::channel;
        if std::env::var_os("NDIR_XDND_TEST").is_none() || std::env::var_os("DISPLAY").is_none() {
            eprintln!("skip: NDIR_XDND_TEST=1 + DISPLAY 필요");
            return;
        }
        let (conn, n) = x11rb::connect(None).expect("X");
        let root = conn.setup().roots[n].root;
        let at = Atoms::intern(&conn).expect("atoms");
        let prop = conn
            .intern_atom(false, b"NDIR_XDND_TEST_PROP")
            .unwrap()
            .reply()
            .unwrap()
            .atom;
        let (tx0, ty0, side) = (300i16, 300i16, 80u16);
        let target = conn.generate_id().unwrap();
        conn.create_window(
            x11rb::COPY_DEPTH_FROM_PARENT,
            target,
            root,
            tx0,
            ty0,
            side,
            side,
            0,
            WindowClass::INPUT_OUTPUT,
            x11rb::COPY_FROM_PARENT,
            &CreateWindowAux::new()
                .override_redirect(1)
                .background_pixel(0)
                .event_mask(EventMask::PROPERTY_CHANGE),
        )
        .unwrap()
        .check()
        .unwrap();
        conn.change_property32(
            PropMode::REPLACE,
            target,
            at.aware,
            AtomEnum::ATOM,
            &[XDND_VERSION],
        )
        .unwrap();
        conn.map_window(target).unwrap();
        conn.flush().unwrap();
        let (tx, rx) = channel::<(Vec<u8>, u32)>();
        let send_to = |conn: &RustConnection, to: Window, type_: Atom, data: [u32; 5]| {
            let ev = ClientMessageEvent {
                response_type: CLIENT_MESSAGE_EVENT,
                format: 32,
                sequence: 0,
                window: to,
                type_,
                data: data.into(),
            };
            let _ = conn.send_event(false, to, EventMask::NO_EVENT, ev);
            let _ = conn.flush();
        };
        // 가짜 대상: Enter 기억 · Position = Status(받음 · 소스가 제안한 동작) · Drop = uri-list 읽기 → Finished.
        std::thread::spawn(move || {
            let mut source = NONE;
            let mut action = NONE;
            loop {
                let Ok(ev) = conn.wait_for_event() else {
                    return;
                };
                match ev {
                    Event::ClientMessage(cm) if cm.type_ == at.enter => {
                        source = cm.data.as_data32()[0];
                    }
                    Event::ClientMessage(cm) if cm.type_ == at.position => {
                        let d = cm.data.as_data32();
                        source = d[0];
                        action = d[4];
                        send_to(&conn, source, at.status, [target, 1, 0, 0, action]);
                    }
                    Event::ClientMessage(cm) if cm.type_ == at.drop => {
                        let _ = conn.convert_selection(
                            target,
                            at.selection,
                            at.uri_list,
                            prop,
                            CURRENT_TIME,
                        );
                        let _ = conn.flush();
                    }
                    Event::SelectionNotify(sn) if sn.property != NONE => {
                        let bytes = conn
                            .get_property(true, target, prop, AtomEnum::ANY, 0, u32::MAX)
                            .unwrap()
                            .reply()
                            .map(|p| p.value)
                            .unwrap_or_default();
                        send_to(&conn, source, at.finished, [target, 1, action, 0, 0]);
                        let _ = tx.send((bytes, action));
                    }
                    _ => {}
                }
            }
        });
        let paths = vec![PathBuf::from("/tmp/x y.txt")];
        let want = b"file:///tmp/x%20y.txt\r\n".to_vec();
        let center = (tx0 + side as i16 / 2, ty0 + side as i16 / 2);
        // ① 프로토콜 소스.
        let h = inject_drop(target, center, paths.clone());
        assert_eq!(h.join().unwrap(), Ok(()));
        let (got, act) = rx
            .recv_timeout(Duration::from_secs(5))
            .expect("drop received");
        assert_eq!(got, want);
        assert_eq!(act, at.action_copy, "제안 = 복사");
        // ② 실제 발신부(세션): 포인터를 대상 가운데로 옮기고(합성 — Warp로 자리 맞춤) 두 번 움직인 뒤 놓는다 · 틱으로 응답 수거.
        let warp = |x: i16, y: i16| {
            let (c, n) = x11rb::connect(None).unwrap();
            let r = c.setup().roots[n].root;
            c.warp_pointer(NONE, r, 0, 0, 0, 0, x, y).unwrap();
            c.flush().unwrap();
        };
        let mut s = Session::start(&paths, None).expect("session");
        warp(center.0, center.1);
        s.motion(center.0, center.1, false, false);
        let pump = |s: &mut Session, ms: u64| -> Option<DragOutcome> {
            let until = Instant::now() + Duration::from_millis(ms);
            while Instant::now() < until {
                if let Some(o) = s.tick() {
                    return Some(o);
                }
                std::thread::sleep(Duration::from_millis(TICK_MS));
            }
            None
        };
        assert_eq!(pump(&mut s, 300), None);
        assert!(s.external_accepts(), "대상이 Status(받음)를 보냈다");
        s.motion(center.0 + 1, center.1, false, true); // Shift = 이동 제안
        assert_eq!(pump(&mut s, 200), None);
        assert!(s.release(), "받는 대상 위에서 뗌 = XdndDrop");
        let out = pump(&mut s, 3000).expect("Finished");
        assert_eq!(
            out,
            DragOutcome::Moved,
            "대상이 제안(이동)을 그대로 돌려줬다"
        );
        let (got2, act2) = rx
            .recv_timeout(Duration::from_secs(5))
            .expect("second drop");
        assert_eq!(got2, want);
        assert_eq!(act2, at.action_move);
        // ③ 대상 밖(빈 루트)에서 떼면 취소 · 세션은 끝난 뒤 결과를 한 번만 준다.
        let mut s3 = Session::start(&paths, None).expect("session");
        warp(center.0, center.1);
        s3.motion(center.0, center.1, false, false);
        assert_eq!(pump(&mut s3, 300), None);
        warp(5, ty0 - 50);
        s3.motion(5, ty0 - 50, false, false); // 대상을 벗어남 = Leave
        assert!(!s3.external_accepts());
        assert!(!s3.release(), "대상 없음 = 창 안 드롭에 맡김");
        assert_eq!(s3.tick(), Some(DragOutcome::Cancelled));
        let _ = rx.recv_timeout(Duration::from_millis(300)).err();
    }
}
