//! X11 **XDND 드래그 발신**(T-147 Linux 2차 · XDND 프로토콜 v5 · x11rb — 추가 crate 0 · DR-8): 선택한 파일을 다른 X11 프로그램
//! (노틸러스 · 돌핀 · 터미널 · 편집기 · 다른 dir3 창)으로 끌어다 놓는다. XWayland 위의 X11 클라이언트끼리도 통한다.
//!
//! - **언제 쓰는가**: winit이 X11 창을 쓸 때만(`DISPLAY`가 있고 `WAYLAND_DISPLAY`가 비어 있음 — [`x11_session`]). Wayland 네이티브
//!   창(GNOME 기본)에서는 winit 0.30.13에 발신·수신이 없고 `wayland-client`는 DR-8로 들이지 않는다 → 종전대로 창 안 드래그
//!   (`App::dnd_internal_*`).
//! - **모양**: Windows OLE(`windrag`)와 같이 `begin_drag`는 **모달**(버튼을 뗄 때까지 돌아오지 않는다). 따로 연 x11rb 연결에서
//!   1×1 InputOnly 창을 만들어 `XdndSelection` 주인이 되고 포인터·키보드를 잡는다(winit 연결은 그동안 사건을 못 받는다 → 돌아온 뒤
//!   호출부가 누름 상태를 정리한다 — `drag_out_after`).
//! - **대상 찾기**: 루트부터 포인터 아래 자식으로 내려가며 `XdndAware`가 처음 붙은 창(WM 프레임 안의 클라이언트 창).
//!   우리 자신의 창(`_NET_WM_PID` = 이 프로세스)이면 XDND 메시지 대신 **실시간 수신기**(`platform::live_drop`)로 바로 넣어
//!   Windows와 같이 머물면 열기 · 강조 · 커서 판정이 산다.
//! - **제안 동작**: Ctrl = 복사 · Shift = 이동 · 그 밖 = 복사(대상이 `XdndActionList` = [복사, 이동]을 보고 고른다 — GTK 파일
//!   관리자는 같은 파일 시스템이면 이동을 고른다). 돌아온 동작이 이동이어도 **원본을 지우지 않는다**(windrag와 같은 안전한 쪽 ·
//!   호출부는 결과와 무관하게 다시 읽는다).
//! - **데이터**: `text/uri-list`(`file:///…` 줄마다 CRLF) · `text/plain` · `UTF8_STRING`(경로를 줄로) — 터미널은 글로 받는다.
//! - 순수 부분(대상 탐색 · 메시지 패킹 · 동작 매핑 · URI 목록)은 시험 · 실제 교환은 실기(XWayland: `WAYLAND_DISPLAY=` 비우고 기동).

use super::{live_drop, DragOutcome, DragSource, DropChoice, DropEvent, PlatformError};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use x11rb::connection::Connection;
use x11rb::protocol::xproto::{
    Atom, AtomEnum, ClientMessageEvent, ConnectionExt, CreateWindowAux, EventMask, GrabMode,
    GrabStatus, KeyButMask, PropMode, SelectionNotifyEvent, SelectionRequestEvent, Window,
    WindowClass, CLIENT_MESSAGE_EVENT, SELECTION_NOTIFY_EVENT,
};
use x11rb::protocol::Event;
use x11rb::rust_connection::RustConnection;
use x11rb::wrapper::ConnectionExt as _;
use x11rb::{CURRENT_TIME, NONE};

/// 우리가 말하는 XDND 판(대상이 더 낮으면 그쪽에 맞춘다 · 3 미만은 모르는 창으로 본다).
pub(crate) const XDND_VERSION: u32 = 5;
/// 놓은 뒤 `XdndFinished`를 기다리는 상한(대상이 안 보내도 돌아온다).
const FINISH_TIMEOUT: Duration = Duration::from_secs(3);

pub(super) struct X11Drag;

impl DragSource for X11Drag {
    fn begin_drag(&self, paths: &[PathBuf]) -> Result<DragOutcome, PlatformError> {
        if paths.is_empty() {
            return Ok(DragOutcome::Cancelled);
        }
        if !x11_session() {
            return Err(PlatformError::Unsupported("drag source (wayland)"));
        }
        run_drag(paths).map_err(|e| PlatformError::Failed(format!("xdnd: {e}")))
    }
    fn supports_os_drag(&self) -> bool {
        x11_session()
    }
}

/// winit이 X11 창을 쓰는 세션인가(순수 판정은 [`x11_session_of`]): `DISPLAY` 있음 ∧ `WAYLAND_DISPLAY` 비어 있음 — winit은
/// Wayland가 있으면 그쪽을 먼저 고른다(`wayland-dlopen`).
pub(crate) fn x11_session() -> bool {
    let has = |k: &str| std::env::var_os(k).is_some_and(|v| !v.is_empty());
    x11_session_of(has("DISPLAY"), has("WAYLAND_DISPLAY"))
}

/// (`DISPLAY` 있음, `WAYLAND_DISPLAY` 있음) → X11 세션인가.
pub(crate) fn x11_session_of(display: bool, wayland: bool) -> bool {
    display && !wayland
}

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
    /// 우리 자신의 창(수신기로 직접) — XDND 메시지는 보내지 않는다.
    own: bool,
    /// 마지막 `XdndStatus`: (받음, 동작).
    accepted: bool,
    action: Atom,
    /// `XdndStatus`를 기다리는 중(그동안 온 위치는 하나만 보관해 뒤에 보낸다 — 프로토콜 규약).
    awaiting_status: bool,
    pending: Option<(i16, i16, u32, DropChoice)>,
}

fn run_drag(paths: &[PathBuf]) -> Result<DragOutcome, Box<dyn std::error::Error>> {
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
        &CreateWindowAux::new()
            .override_redirect(1)
            .event_mask(EventMask::PROPERTY_CHANGE),
    )?
    .check()?;
    conn.map_window(win)?.check()?; // 잡기(GrabPointer)는 보이는(viewable) 창이어야 한다
    conn.change_property32(
        PropMode::REPLACE,
        win,
        at.action_list,
        AtomEnum::ATOM,
        &[at.action_copy, at.action_move],
    )?;
    conn.set_selection_owner(win, at.selection, CURRENT_TIME)?
        .check()?;
    let grab = conn
        .grab_pointer(
            false,
            win,
            EventMask::BUTTON_RELEASE | EventMask::POINTER_MOTION | EventMask::BUTTON_PRESS,
            GrabMode::ASYNC,
            GrabMode::ASYNC,
            NONE,
            NONE,
            CURRENT_TIME,
        )?
        .reply()?;
    if grab.status != GrabStatus::SUCCESS {
        let _ = conn.destroy_window(win);
        let _ = conn.flush();
        return Err(format!("pointer grab failed: {:?}", grab.status).into());
    }
    let _ = conn
        .grab_keyboard(false, win, CURRENT_TIME, GrabMode::ASYNC, GrabMode::ASYNC)?
        .reply(); // Esc 취소용 — 실패해도 드래그는 된다
    let escape = escape_keycodes(&conn);
    let my_pid = std::process::id();
    let data = Payload {
        uri_list: uri_list(paths).into_bytes(),
        plain: plain_list(paths).into_bytes(),
    };
    let mut target: Option<Target> = None;
    let mut dropped = false;
    let mut finished: Option<(bool, Atom)> = None;
    let mut deadline: Option<Instant> = None;
    let result = loop {
        let ev = if let Some(dl) = deadline {
            // 놓은 뒤: `XdndFinished`를 기다리되 상한을 둔다(그동안 SelectionRequest는 계속 받아 준다).
            if Instant::now() >= dl {
                break finished;
            }
            match conn.poll_for_event()? {
                Some(ev) => ev,
                None => {
                    std::thread::sleep(Duration::from_millis(5));
                    continue;
                }
            }
        } else {
            conn.wait_for_event()?
        };
        match ev {
            Event::MotionNotify(m) if deadline.is_none() => {
                let (ctrl, shift) = mods(m.state);
                let found = find_target(root, &mut |w| probe(&conn, &at, w));
                let same = matches!((&target, found), (Some(t), Some((w, _))) if t.win == w);
                if !same {
                    if let Some(t) = target.take() {
                        leave(&conn, &at, win, &t, paths);
                    }
                    if let Some((w, version)) = found {
                        let own = is_own_window(&conn, &at, w, my_pid);
                        let mut t = Target {
                            win: w,
                            version,
                            own,
                            accepted: false,
                            action: NONE,
                            awaiting_status: false,
                            pending: None,
                        };
                        if own {
                            let at_pt = local_point(&conn, root, w, m.root_x, m.root_y);
                            let choice = live_drop(&DropEvent::Enter {
                                paths: paths.to_vec(),
                                at: at_pt,
                                ctrl,
                                shift,
                            })
                            .unwrap_or_default();
                            t.accepted = choice != DropChoice::None;
                            t.action = at.action_of(choice);
                        } else {
                            send(
                                &conn,
                                w,
                                at.enter,
                                [
                                    win,
                                    enter_flags(version, false),
                                    at.uri_list,
                                    at.text_plain,
                                    at.utf8,
                                ],
                            )?;
                        }
                        target = Some(t);
                    }
                }
                if let Some(t) = target.as_mut() {
                    let choice = proposed(ctrl, shift);
                    if t.own {
                        let at_pt = local_point(&conn, root, t.win, m.root_x, m.root_y);
                        let c = live_drop(&DropEvent::Over {
                            at: at_pt,
                            ctrl,
                            shift,
                        })
                        .unwrap_or_default();
                        t.accepted = c != DropChoice::None;
                        t.action = at.action_of(c);
                    } else if t.awaiting_status {
                        t.pending = Some((m.root_x, m.root_y, m.time, choice));
                    } else {
                        position(&conn, &at, win, t, m.root_x, m.root_y, m.time, choice)?;
                    }
                }
            }
            Event::ButtonRelease(b) if b.detail == 1 && deadline.is_none() => {
                let _ = conn.ungrab_pointer(CURRENT_TIME);
                let _ = conn.ungrab_keyboard(CURRENT_TIME);
                let _ = conn.flush();
                match target.as_mut() {
                    Some(t) if t.accepted => {
                        dropped = true;
                        if t.own {
                            let (ctrl, shift) = mods(b.state);
                            let at_pt = local_point(&conn, root, t.win, b.root_x, b.root_y);
                            let c = live_drop(&DropEvent::Drop {
                                paths: paths.to_vec(),
                                at: at_pt,
                                ctrl,
                                shift,
                            })
                            .unwrap_or_default();
                            break Some((c != DropChoice::None, at.action_of(c)));
                        }
                        send(&conn, t.win, at.drop, [win, 0, b.time, 0, 0])?;
                        deadline = Some(Instant::now() + FINISH_TIMEOUT);
                    }
                    Some(t) => {
                        leave(&conn, &at, win, t, paths);
                        break None;
                    }
                    None => break None,
                }
            }
            Event::KeyPress(k) if escape.contains(&k.detail) && deadline.is_none() => {
                if let Some(t) = target.as_ref() {
                    leave(&conn, &at, win, t, paths);
                }
                break None;
            }
            Event::ClientMessage(cm) if cm.type_ == at.status => {
                let d = cm.data.as_data32();
                if let Some(t) = target.as_mut().filter(|t| t.win == d[0]) {
                    t.accepted = d[1] & 1 != 0;
                    t.action = if t.accepted { d[4] } else { NONE };
                    t.awaiting_status = false;
                    if let Some((x, y, time, choice)) = t.pending.take() {
                        position(&conn, &at, win, t, x, y, time, choice)?;
                    }
                }
            }
            Event::ClientMessage(cm) if cm.type_ == at.finished => {
                let d = cm.data.as_data32();
                let (accepted, action) = match target.as_ref() {
                    Some(t) if t.version >= 5 => (d[1] & 1 != 0, d[2]),
                    Some(t) => (t.accepted, t.action),
                    None => (false, NONE),
                };
                finished = Some((accepted, action));
                if deadline.is_some() {
                    break finished;
                }
            }
            Event::SelectionRequest(r) if r.selection == at.selection => {
                serve(&conn, &at, &data, &r)?;
            }
            _ => {}
        }
    };
    if let Some(t) = target.as_ref().filter(|t| !dropped && !t.own) {
        leave(&conn, &at, win, t, paths);
    }
    let _ = conn.ungrab_pointer(CURRENT_TIME);
    let _ = conn.ungrab_keyboard(CURRENT_TIME);
    let _ = conn.destroy_window(win);
    let _ = conn.flush();
    let (accepted, action) = result
        .or_else(|| target.as_ref().map(|t| (t.accepted, t.action)))
        .unwrap_or((false, NONE));
    Ok(outcome(dropped, accepted, action == at.action_move))
}

struct Payload {
    uri_list: Vec<u8>,
    plain: Vec<u8>,
}

/// 창 하나를 묻는다: (포인터 아래 자식, XdndAware 판).
fn probe(conn: &RustConnection, at: &Atoms, w: Window) -> (Option<Window>, u32) {
    let child = conn
        .query_pointer(w)
        .ok()
        .and_then(|c| c.reply().ok())
        .map(|r| r.child)
        .filter(|c| *c != NONE);
    let version = conn
        .get_property(false, w, at.aware, AtomEnum::ATOM, 0, 1)
        .ok()
        .and_then(|c| c.reply().ok())
        .and_then(|p| p.value32().and_then(|mut it| it.next()))
        .unwrap_or(0);
    (child, version)
}

fn is_own_window(conn: &RustConnection, at: &Atoms, w: Window, pid: u32) -> bool {
    conn.get_property(false, w, at.net_wm_pid, AtomEnum::CARDINAL, 0, 1)
        .ok()
        .and_then(|c| c.reply().ok())
        .and_then(|p| p.value32().and_then(|mut it| it.next()))
        == Some(pid)
}

/// 루트 좌표 → 그 창의 좌표.
fn local_point(conn: &RustConnection, root: Window, w: Window, x: i16, y: i16) -> (i32, i32) {
    conn.translate_coordinates(root, w, x, y)
        .ok()
        .and_then(|c| c.reply().ok())
        .map_or((i32::from(x), i32::from(y)), |r| {
            (i32::from(r.dst_x), i32::from(r.dst_y))
        })
}

fn mods(state: KeyButMask) -> (bool, bool) {
    (
        state.contains(KeyButMask::CONTROL),
        state.contains(KeyButMask::SHIFT),
    )
}

fn send(
    conn: &RustConnection,
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
    conn.send_event(false, to, EventMask::NO_EVENT, ev)?;
    conn.flush()?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn position(
    conn: &RustConnection,
    at: &Atoms,
    src: Window,
    t: &mut Target,
    x: i16,
    y: i16,
    time: u32,
    choice: DropChoice,
) -> Result<(), Box<dyn std::error::Error>> {
    send(
        conn,
        t.win,
        at.position,
        [src, 0, pack_pos(x, y), time, at.action_of(choice)],
    )?;
    t.awaiting_status = true;
    Ok(())
}

fn leave(conn: &RustConnection, at: &Atoms, src: Window, t: &Target, _paths: &[PathBuf]) {
    if t.own {
        let _ = live_drop(&DropEvent::Leave);
    } else {
        let _ = send(conn, t.win, at.leave, [src, 0, 0, 0, 0]);
    }
}

/// 대상이 데이터를 달라고 한다(`XdndSelection`): TARGETS · 세 종류 · 그 밖은 거절.
fn serve(
    conn: &RustConnection,
    at: &Atoms,
    data: &Payload,
    r: &SelectionRequestEvent,
) -> Result<(), Box<dyn std::error::Error>> {
    let property = if r.property == NONE {
        r.target
    } else {
        r.property
    };
    let ok = if r.target == at.targets {
        conn.change_property32(
            PropMode::REPLACE,
            r.requestor,
            property,
            AtomEnum::ATOM,
            &[at.targets, at.uri_list, at.text_plain, at.utf8],
        )?;
        true
    } else if r.target == at.uri_list {
        conn.change_property8(
            PropMode::REPLACE,
            r.requestor,
            property,
            r.target,
            &data.uri_list,
        )?;
        true
    } else if r.target == at.text_plain || r.target == at.utf8 {
        conn.change_property8(
            PropMode::REPLACE,
            r.requestor,
            property,
            r.target,
            &data.plain,
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
    conn.send_event(false, r.requestor, EventMask::NO_EVENT, notify)?;
    conn.flush()?;
    Ok(())
}

/// Esc 키코드(키 배치에서 keysym 0xff1b를 찾는다 · 못 찾으면 비어 있음 = Esc 취소 없음).
fn escape_keycodes(conn: &RustConnection) -> Vec<u8> {
    const XK_ESCAPE: u32 = 0xff1b;
    let setup = conn.setup();
    let (min, max) = (setup.min_keycode, setup.max_keycode);
    let Some(reply) = conn
        .get_keyboard_mapping(min, max - min + 1)
        .ok()
        .and_then(|c| c.reply().ok())
    else {
        return Vec::new();
    };
    let per = usize::from(reply.keysyms_per_keycode.max(1));
    reply
        .keysyms
        .chunks(per)
        .enumerate()
        .filter(|(_, syms)| syms.contains(&XK_ESCAPE))
        .filter_map(|(i, _)| u8::try_from(i).ok().map(|i| min + i))
        .collect()
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
        assert!(
            x11_session_of(true, false)
                && !x11_session_of(true, true)
                && !x11_session_of(false, false)
        );
    }
}
