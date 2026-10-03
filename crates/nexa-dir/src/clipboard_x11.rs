//! X11 `CLIPBOARD` selection **직접 구현**(사용자 09-26 — *"클립보드 관련 프로그램이 없다고 앱에서 클립보드가 안 되는 게 정상이야?"*).
//!
//! 정상이 아니다. 클립보드는 프로그램이 아니라 **프로토콜**이고, 창을 띄운 앱은 이미 X 서버와 연결돼 있으므로 직접 참여하면 된다
//! (VS Code·Sublime·Firefox가 `xclip` 없이 되는 이유). 종전 Linux 경로는 `wl-copy`/`xclip`/`xsel` **외부 프로그램**을 불렀고,
//! 그 패키지가 없는 기기에서는 복사·붙여넣기 기능 자체가 사라졌다. Windows(`SetClipboardData`)·macOS(`pbcopy`)와 격이 달랐다.
//!
//! 추가 의존은 **0** — `x11rb`는 91차에 Linux 모달(`WM_TRANSIENT_FOR`)용으로 이미 들어와 있다(DR-3 예외 원장 · docs/10 §4).
//!
//! 구조:
//! - **쓰기** = 전용 스레드가 1×1 InputOnly 창으로 `SetSelectionOwner(CLIPBOARD)`를 잡고 살아 있는 동안 소유권을 유지한다.
//!   `SelectionRequest`가 오면 `TARGETS`·`UTF8_STRING`·`STRING`·`text/plain;charset=utf-8`에 답한다. 큰 값은 **INCR**로 나눠 보낸다.
//!   → 외부 도구 경로의 고질(넣어 준 `xclip` 프로세스가 죽으면 클립보드가 비는 것)도 같이 사라진다.
//! - **읽기** = 우리가 소유자면 보관 중인 값을 그대로, 아니면 `ConvertSelection` → `SelectionNotify` → property(필요하면 INCR 수신).
//! - 스레드는 **처음 복사할 때 한 번** 뜬다(붙여넣기만 하면 안 뜬다 · 부하원 원장 39 §3). 설정 `clipboard.x11_native`로 끄면 종전 CLI 경로.
//! - **파일 목록**(T-53 · docs/port/19 §4-4): 같은 소유자가 `text/uri-list`(RFC 2483 · `file:///퍼센트-인코딩` · `\r\n`) ·
//!   `x-special/gnome-copied-files`(`copy|cut\nfile:///a\nfile:///b` — Nautilus·Nemo·Caja·Thunar) · `application/x-kde-cutselection`(`1`=잘라내기)
//!   을 **동시 게시**하고 텍스트 타깃에는 경로 줄 목록을 준다. 읽기는 gnome → uri-list(+kde cut) 순.
//!
//! ★ 출처: nexa-sql/crates/nexa-sql/src/clipboard_x11.rs(10-03 복사 · docs/port/40 SKEL-403 — `NSQL_*`→`NDIR_*` · 제품명만 개명 · 로직 불변 · 파일 타깃은 dir3 추가).

use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Sender};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use x11rb::connection::Connection;
use x11rb::protocol::xproto::{
    Atom, AtomEnum, ConnectionExt as _, CreateWindowAux, EventMask, PropMode, Property,
    SelectionNotifyEvent, SelectionRequestEvent, Window, WindowClass, SELECTION_NOTIFY_EVENT,
};
use x11rb::protocol::Event;
use x11rb::rust_connection::RustConnection;
use x11rb::wrapper::ConnectionExt as _;
use x11rb::{CURRENT_TIME, NONE};

/// 한 번에 보내는 최대 바이트(이보다 크면 INCR). X11 요청 상한보다 넉넉히 작게.
const CHUNK: usize = 128 * 1024;
/// 붙여넣기 응답 대기 상한 — 상대 앱이 죽었거나 답이 없으면 여기서 포기한다(UI를 잡지 않는다).
const READ_TIMEOUT: Duration = Duration::from_millis(1200);

struct Atoms {
    clipboard: Atom,
    targets: Atom,
    utf8: Atom,
    text_plain: Atom,
    incr: Atom,
    prop: Atom,
    uri_list: Atom,
    gnome: Atom,
    kde_cut: Atom,
}

fn atoms(conn: &RustConnection) -> Result<Atoms, Box<dyn std::error::Error>> {
    let a = |n: &[u8]| -> Result<Atom, Box<dyn std::error::Error>> {
        Ok(conn.intern_atom(false, n)?.reply()?.atom)
    };
    Ok(Atoms {
        clipboard: a(b"CLIPBOARD")?,
        targets: a(b"TARGETS")?,
        utf8: a(b"UTF8_STRING")?,
        text_plain: a(b"text/plain;charset=utf-8")?,
        incr: a(b"INCR")?,
        prop: a(b"NEXA_DIR_CLIP")?,
        uri_list: a(b"text/uri-list")?,
        gnome: a(b"x-special/gnome-copied-files")?,
        kde_cut: a(b"application/x-kde-cutselection")?,
    })
}

/// 소유 중인 내용 — 텍스트는 항상 · 파일 목록이면 uri-list/gnome/kde 표현도 같이 게시한다.
#[derive(Clone, Default)]
struct Payload {
    text: Vec<u8>,
    files: Option<(Vec<PathBuf>, bool)>,
    uri_list: Vec<u8>,
    gnome: Vec<u8>,
}

impl Payload {
    fn text(text: &str) -> Self {
        Payload {
            text: text.as_bytes().to_vec(),
            ..Default::default()
        }
    }

    fn files(paths: &[PathBuf], cut: bool) -> Self {
        let uris: Vec<String> = paths.iter().map(|p| file_uri(p)).collect();
        let text = paths
            .iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join("\n");
        let mut uri_list = String::new();
        for u in &uris {
            uri_list.push_str(u);
            uri_list.push_str("\r\n");
        }
        let gnome = format!("{}\n{}", if cut { "cut" } else { "copy" }, uris.join("\n"));
        Payload {
            text: text.into_bytes(),
            files: Some((paths.to_vec(), cut)),
            uri_list: uri_list.into_bytes(),
            gnome: gnome.into_bytes(),
        }
    }
}

/// 1×1 InputOnly 창(화면에 안 보인다 · selection 주고받기용 주소).
fn helper_window(
    conn: &RustConnection,
    screen: usize,
) -> Result<Window, Box<dyn std::error::Error>> {
    let root = conn.setup().roots[screen].root;
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
    .check()?; // ★ 오류(BadMatch 등)를 여기서 받는다 — 흘려보내면 소유권도 응답도 조용히 실패한다(09-26 실측)
    Ok(win)
}

// ───────────────────────── URI 변환(순수) ─────────────────────────

/// 경로 → `file:///…`(RFC 2483 · 비예약 문자와 `/`만 그대로 · 나머지 바이트는 `%XX`).
pub(crate) fn file_uri(path: &Path) -> String {
    let mut s = String::from("file://");
    let bytes = path.as_os_str().as_encoded_bytes();
    if !bytes.starts_with(b"/") {
        s.push('/');
    }
    for &b in bytes {
        let keep = b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~' | b'/');
        if keep {
            s.push(b as char);
        } else {
            s.push_str(&format!("%{b:02X}"));
        }
    }
    s
}

/// `file://` URI → 경로(호스트 비움/`localhost`만 · 다른 스킴 = None · `%XX` 복원).
pub(crate) fn uri_path(uri: &str) -> Option<PathBuf> {
    let rest = uri.trim().strip_prefix("file://")?;
    let path = if let Some(p) = rest.strip_prefix("localhost/") {
        format!("/{p}")
    } else if rest.starts_with('/') {
        rest.to_string()
    } else {
        return None; // 원격 호스트
    };
    let mut out: Vec<u8> = Vec::with_capacity(path.len());
    let b = path.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let (Some(h), Some(l)) = (hex(b[i + 1]), hex(b[i + 2])) {
                out.push(h << 4 | l);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    let s = String::from_utf8_lossy(&out).into_owned();
    (!s.is_empty()).then(|| PathBuf::from(s))
}

fn hex(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

/// `text/uri-list` 본문 → 경로들(`#` 주석 · 빈 줄 · 비파일 스킴 제외).
pub(crate) fn parse_uri_list(bytes: &[u8]) -> Vec<PathBuf> {
    String::from_utf8_lossy(bytes)
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .filter_map(uri_path)
        .collect()
}

/// `x-special/gnome-copied-files` 본문 → (경로들, 잘라내기). 첫 줄 `cut`/`copy`.
pub(crate) fn parse_gnome(bytes: &[u8]) -> Option<(Vec<PathBuf>, bool)> {
    let text = String::from_utf8_lossy(bytes);
    let mut lines = text.lines();
    let head = lines.next()?.trim();
    let cut = match head {
        "cut" => true,
        "copy" => false,
        _ => return None,
    };
    let paths: Vec<PathBuf> = lines.map(str::trim).filter_map(uri_path).collect();
    (!paths.is_empty()).then_some((paths, cut))
}

// ───────────────────────── 쓰기(소유권 유지 스레드) ─────────────────────────

static OWNER: OnceLock<Option<Sender<Payload>>> = OnceLock::new();
/// 지금 우리가 들고 있는 내용 — 우리가 소유자일 때의 읽기는 왕복 없이 여기서.
static MINE: Mutex<Option<Payload>> = Mutex::new(None);

fn publish(payload: Payload, what: &str) -> bool {
    let tx = OWNER.get_or_init(|| spawn_owner().ok());
    let Some(tx) = tx.as_ref() else {
        if std::env::var_os("NDIR_TRACE_CLIP").is_some() {
            eprintln!("[clip] x11 write({what}): owner thread unavailable → fallback");
        }
        return false;
    };
    let len = payload.text.len();
    *MINE.lock().unwrap_or_else(|e| e.into_inner()) = Some(payload.clone());
    let sent = tx.send(payload).is_ok();
    if std::env::var_os("NDIR_TRACE_CLIP").is_some() {
        eprintln!("[clip] x11 write({what}): {len} bytes queued={sent}");
    }
    sent
}

/// 텍스트를 CLIPBOARD에 올린다(소유자가 된다). 실패 = `false` → 호출측이 CLI 경로로 폴백.
pub(crate) fn write(text: &str) -> bool {
    publish(Payload::text(text), "text")
}

/// 파일 목록을 CLIPBOARD에 올린다(uri-list · gnome-copied-files · kde cut · 텍스트 = 경로 줄). 실패 = `false`.
pub(crate) fn write_files(paths: &[PathBuf], cut: bool) -> bool {
    publish(Payload::files(paths, cut), "files")
}

fn spawn_owner() -> Result<Sender<Payload>, Box<dyn std::error::Error>> {
    let (tx, rx) = channel::<Payload>();
    // 연결·창은 스레드 안에서 만든다(RustConnection은 Send가 아니어도 되게).
    let (ready_tx, ready_rx) = channel::<bool>();
    std::thread::Builder::new()
        .name("ndir-clipboard".into())
        .spawn(move || {
            let Ok((conn, screen)) = x11rb::connect(None) else {
                let _ = ready_tx.send(false);
                return;
            };
            let (Ok(at), Ok(win)) = (atoms(&conn), helper_window(&conn, screen)) else {
                let _ = ready_tx.send(false);
                return;
            };
            let _ = ready_tx.send(true);
            let mut data = Payload::default();
            // 나눠 보내는 중인 요청들(INCR): (요청자 창, property, 남은 자리)
            let mut incr: Vec<(Window, Atom, usize)> = Vec::new();
            loop {
                // 새 복사가 있으면 내용 교체 + 소유권(다시) 잡기.
                while let Ok(v) = rx.try_recv() {
                    data = v;
                    let owned = conn
                        .set_selection_owner(win, at.clipboard, CURRENT_TIME)
                        .ok()
                        .and_then(|c| c.check().ok())
                        .is_some()
                        && conn
                            .get_selection_owner(at.clipboard)
                            .ok()
                            .and_then(|c| c.reply().ok())
                            .is_some_and(|r| r.owner == win);
                    if !owned {
                        // 서버가 소유권을 안 줬다 — 보관분을 비워 `read()`가 "복사됐다"고 거짓말하지 않게 한다.
                        *MINE.lock().unwrap_or_else(|e| e.into_inner()) = None;
                        if std::env::var_os("NDIR_TRACE_CLIP").is_some() {
                            eprintln!(
                                "[clip] x11: set_selection_owner not honoured (win {win:#x})"
                            );
                        }
                    }
                }
                let Ok(ev) = conn.poll_for_event() else {
                    return;
                };
                match ev {
                    Some(Event::SelectionRequest(r)) => serve(&conn, &at, &data, &r, &mut incr),
                    Some(Event::SelectionClear(_)) => {
                        // 다른 앱이 가져갔다 — 보관분은 버린다(읽기는 그쪽에 물어본다).
                        *MINE.lock().unwrap_or_else(|e| e.into_inner()) = None;
                    }
                    Some(Event::PropertyNotify(p)) => {
                        // INCR 계속: 상대가 앞 조각을 지웠다 = 다음 조각을 올릴 차례.
                        if p.state == Property::DELETE {
                            step_incr(&conn, &at, &data.text, p.window, p.atom, &mut incr);
                        }
                    }
                    Some(Event::Error(e)) => {
                        if std::env::var_os("NDIR_TRACE_CLIP").is_some() {
                            eprintln!("[clip] x11 owner: {e:?}");
                        }
                    }
                    Some(_) => {}
                    None => std::thread::sleep(Duration::from_millis(15)),
                }
            }
        })?;
    match ready_rx.recv_timeout(Duration::from_secs(2)) {
        Ok(true) => Ok(tx),
        _ => Err("x11 clipboard thread not ready".into()),
    }
}

/// `SelectionRequest` 한 건에 답한다.
fn serve(
    conn: &RustConnection,
    at: &Atoms,
    data: &Payload,
    r: &SelectionRequestEvent,
    incr: &mut Vec<(Window, Atom, usize)>,
) {
    let prop = if r.property == NONE {
        r.target
    } else {
        r.property
    };
    let has_files = data.files.is_some();
    let ok = if r.target == at.targets {
        let mut list = vec![
            at.targets,
            at.utf8,
            at.text_plain,
            u32::from(AtomEnum::STRING),
        ];
        if has_files {
            list.extend([at.uri_list, at.gnome, at.kde_cut]);
        }
        conn.change_property32(PropMode::REPLACE, r.requestor, prop, AtomEnum::ATOM, &list)
            .is_ok()
    } else if has_files && r.target == at.uri_list {
        conn.change_property8(
            PropMode::REPLACE,
            r.requestor,
            prop,
            r.target,
            &data.uri_list,
        )
        .is_ok()
    } else if has_files && r.target == at.gnome {
        conn.change_property8(PropMode::REPLACE, r.requestor, prop, r.target, &data.gnome)
            .is_ok()
    } else if has_files && r.target == at.kde_cut {
        let v: &[u8] = if data.files.as_ref().is_some_and(|(_, cut)| *cut) {
            b"1"
        } else {
            b"0"
        };
        conn.change_property8(PropMode::REPLACE, r.requestor, prop, r.target, v)
            .is_ok()
    } else if r.target == at.utf8
        || r.target == at.text_plain
        || r.target == u32::from(AtomEnum::STRING)
    {
        if data.text.len() <= CHUNK {
            conn.change_property8(PropMode::REPLACE, r.requestor, prop, r.target, &data.text)
                .is_ok()
        } else {
            // INCR 시작: 전체 크기를 알리고, 상대가 property를 지울 때마다 한 조각씩.
            let _ = conn.change_window_attributes(
                r.requestor,
                &x11rb::protocol::xproto::ChangeWindowAttributesAux::new()
                    .event_mask(EventMask::PROPERTY_CHANGE),
            );
            let total = [data.text.len() as u32];
            let ok = conn
                .change_property32(PropMode::REPLACE, r.requestor, prop, at.incr, &total)
                .is_ok();
            if ok {
                incr.push((r.requestor, prop, 0));
            }
            ok
        }
    } else {
        false
    };
    let ev = SelectionNotifyEvent {
        response_type: SELECTION_NOTIFY_EVENT,
        sequence: 0,
        time: r.time,
        requestor: r.requestor,
        selection: r.selection,
        target: r.target,
        property: if ok { prop } else { NONE },
    };
    let _ = conn.send_event(false, r.requestor, EventMask::NO_EVENT, ev);
    let _ = conn.flush();
}

/// INCR 다음 조각(빈 조각 = 끝).
fn step_incr(
    conn: &RustConnection,
    at: &Atoms,
    data: &[u8],
    win: Window,
    prop: Atom,
    incr: &mut Vec<(Window, Atom, usize)>,
) {
    let Some(i) = incr.iter().position(|(w, p, _)| *w == win && *p == prop) else {
        return;
    };
    let sent = incr[i].2;
    let end = (sent + CHUNK).min(data.len());
    let chunk = &data[sent..end];
    let _ = conn.change_property8(PropMode::REPLACE, win, prop, at.utf8, chunk);
    let _ = conn.flush();
    if chunk.is_empty() {
        incr.remove(i); // 빈 조각을 보냈다 = 전송 끝
    } else {
        incr[i].2 = end;
    }
}

// ───────────────────────── 읽기 ─────────────────────────

/// CLIPBOARD 텍스트. 우리가 소유자면 보관분을 그대로 돌려준다(왕복 0).
pub(crate) fn read() -> Option<String> {
    if let Some(p) = MINE.lock().unwrap_or_else(|e| e.into_inner()).clone() {
        return String::from_utf8(p.text).ok();
    }
    read_target(Target::Utf8)
        .ok()
        .flatten()
        .map(|v| String::from_utf8_lossy(&v).into_owned())
}

/// CLIPBOARD 파일 목록(gnome-copied-files → uri-list + kde cut). 우리가 소유자면 보관분.
pub(crate) fn read_files() -> Option<(Vec<PathBuf>, bool)> {
    if let Some(p) = MINE.lock().unwrap_or_else(|e| e.into_inner()).clone() {
        return p.files;
    }
    if let Some(g) = read_target(Target::Gnome).ok().flatten() {
        if let Some(r) = parse_gnome(&g) {
            return Some(r);
        }
    }
    let list = read_target(Target::UriList).ok().flatten()?;
    let paths = parse_uri_list(&list);
    if paths.is_empty() {
        return None;
    }
    let cut = read_target(Target::KdeCut)
        .ok()
        .flatten()
        .is_some_and(|v| v.first() == Some(&b'1'));
    Some((paths, cut))
}

#[derive(Clone, Copy)]
enum Target {
    Utf8,
    UriList,
    Gnome,
    KdeCut,
}

fn target_atom(at: &Atoms, t: Target) -> Atom {
    match t {
        Target::Utf8 => at.utf8,
        Target::UriList => at.uri_list,
        Target::Gnome => at.gnome,
        Target::KdeCut => at.kde_cut,
    }
}

fn read_target(t: Target) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error>> {
    let (conn, screen) = x11rb::connect(None)?;
    let at = atoms(&conn)?;
    if conn.get_selection_owner(at.clipboard)?.reply()?.owner == NONE {
        return Ok(None); // 아무도 안 들고 있다 = 빈 클립보드
    }
    let win = helper_window(&conn, screen)?;
    let target = target_atom(&at, t);
    conn.convert_selection(win, at.clipboard, target, at.prop, CURRENT_TIME)?;
    conn.flush()?;
    let deadline = Instant::now() + READ_TIMEOUT;
    while Instant::now() < deadline {
        match conn.poll_for_event()? {
            Some(Event::SelectionNotify(n)) => {
                if n.property == NONE {
                    return Ok(None); // 상대가 그 타깃을 못 준다
                }
                let r = conn
                    .get_property(true, win, at.prop, AtomEnum::ANY, 0, u32::MAX / 4)?
                    .reply()?;
                if r.type_ == at.incr {
                    return recv_incr(&conn, &at, win, deadline);
                }
                return Ok(Some(r.value));
            }
            Some(Event::Error(e)) => {
                if std::env::var_os("NDIR_TRACE_CLIP").is_some() {
                    eprintln!("[clip] x11 read: {e:?}");
                }
            }
            Some(_) => {}
            None => std::thread::sleep(Duration::from_millis(5)),
        }
    }
    Ok(None)
}

/// INCR 수신: property를 지울 때마다 상대가 다음 조각을 올린다. 빈 조각 = 끝.
fn recv_incr(
    conn: &RustConnection,
    at: &Atoms,
    win: Window,
    deadline: Instant,
) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error>> {
    let mut buf: Vec<u8> = Vec::new();
    while Instant::now() < deadline {
        match conn.poll_for_event()? {
            Some(Event::PropertyNotify(p)) if p.window == win && p.state == Property::NEW_VALUE => {
                let r = conn
                    .get_property(true, win, at.prop, AtomEnum::ANY, 0, u32::MAX / 4)?
                    .reply()?;
                if r.value.is_empty() {
                    return Ok(Some(buf));
                }
                buf.extend_from_slice(&r.value);
            }
            Some(_) => {}
            None => std::thread::sleep(Duration::from_millis(5)),
        }
    }
    Ok((!buf.is_empty()).then_some(buf))
}

#[cfg(test)]
mod tests {
    //! 순수 변환 시험은 어디서나 돈다. 진짜 X 서버가 있어야 도는 왕복 시험(`cargo test -p nexa-dir clipboard_x11 -- --ignored`)은
    //! CI(헤드리스)에서 건너뛴다. 상대편은 **다른 프로세스**(python3 Gtk)여야 selection 전송이 실제로 일어난다. 환경 변수:
    //!   NDIR_CLIP_EXPECT = 다른 프로세스가 미리 올려 둔 글(읽기 시험) · NDIR_CLIP_HOLD_MS = 쓰기 뒤 소유권을 유지할 시간.
    use super::*;

    /// 경로 ↔ file URI(공백·한글·`#` 퍼센트 인코딩 · localhost 호스트 · 비파일 스킴 거부) · uri-list(주석/CRLF) · gnome(cut/copy) 파싱 · Payload 표현.
    #[test]
    fn uri_round_trip_and_list_parsing() {
        let p = PathBuf::from("/home/u/내 문서/a #1.txt");
        let u = file_uri(&p);
        assert_eq!(
            u,
            "file:///home/u/%EB%82%B4%20%EB%AC%B8%EC%84%9C/a%20%231.txt"
        );
        assert_eq!(uri_path(&u), Some(p.clone()));
        assert_eq!(
            uri_path("file://localhost/tmp/x"),
            Some(PathBuf::from("/tmp/x"))
        );
        assert_eq!(uri_path("file://host/tmp/x"), None);
        assert_eq!(uri_path("http://example.com/a"), None);
        let list = b"# comment\r\nfile:///a/b.txt\r\n\r\nfile:///c%20d\r\nhttp://x/y\r\n";
        assert_eq!(
            parse_uri_list(list),
            vec![PathBuf::from("/a/b.txt"), PathBuf::from("/c d")]
        );
        assert_eq!(
            parse_gnome(b"cut\nfile:///a\nfile:///b"),
            Some((vec![PathBuf::from("/a"), PathBuf::from("/b")], true))
        );
        assert_eq!(parse_gnome(b"copy\nfile:///a").map(|(_, c)| c), Some(false));
        assert_eq!(parse_gnome(b"move\nfile:///a"), None);
        let pl = Payload::files(&[PathBuf::from("/a"), p.clone()], true);
        assert_eq!(
            pl.uri_list,
            b"file:///a\r\nfile:///home/u/%EB%82%B4%20%EB%AC%B8%EC%84%9C/a%20%231.txt\r\n".to_vec()
        );
        assert!(pl.gnome.starts_with(b"cut\nfile:///a\n"));
        assert_eq!(
            String::from_utf8(pl.text).unwrap(),
            format!("/a\n{}", p.display())
        );
        assert_eq!(pl.files, Some((vec![PathBuf::from("/a"), p], true)));
        assert!(Payload::text("x").files.is_none());
    }

    #[test]
    #[ignore]
    fn read_what_another_process_put() {
        let want = std::env::var("NDIR_CLIP_EXPECT").expect("NDIR_CLIP_EXPECT");
        assert_eq!(read().as_deref(), Some(want.as_str()));
    }

    #[test]
    #[ignore]
    fn write_and_hold_for_another_process() {
        let text = std::env::var("NDIR_CLIP_TEXT")
            .unwrap_or_else(|_| "nexa-dir X11 clipboard ✓ 한글".into());
        assert!(write(&text), "write");
        assert_eq!(
            read().as_deref(),
            Some(text.as_str()),
            "우리가 소유자일 때 읽기 = 보관분"
        );
        let hold: u64 = std::env::var("NDIR_CLIP_HOLD_MS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(3000);
        std::thread::sleep(Duration::from_millis(hold)); // 그동안 다른 프로세스가 읽는다
    }

    #[test]
    #[ignore]
    fn write_large_uses_incr() {
        // CHUNK보다 큰 값 = INCR 경로. 상대가 조각을 받아 이어 붙이는지는 다른 프로세스가 본다.
        let text: String = (0..(CHUNK * 3 / 40))
            .map(|i| format!("line {i} 한글 텍스트 줄\n"))
            .collect();
        assert!(text.len() > CHUNK);
        std::fs::write(
            std::env::var("NDIR_CLIP_OUT").expect("NDIR_CLIP_OUT"),
            &text,
        )
        .expect("write NDIR_CLIP_OUT");
        assert!(write(&text));
        let hold: u64 = std::env::var("NDIR_CLIP_HOLD_MS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(4000);
        std::thread::sleep(Duration::from_millis(hold));
    }
}
