//! 도크 **미리보기 워커**(10-18 · 사용자 "파일 목록에서 마우스 선택이 키보드보다 체감이 느리다"): 미리보기 계산(텍스트 읽기 · WASM 플러그인
//! 실행(≤ 1.5 s) · 압축 목록 · 첫 사용 때 플러그인 컴파일)과 사진 디코드(수 MP = 수백 ms)를 UI 스레드에서 하던 것을 이 워커가 한다.
//! 키보드는 연타가 디바운스로 합쳐져 덜 느껴지고, 마우스는 클릭마다 한 건씩 UI가 멈췄다.
//!
//! 구조(docs/01 §3): 워커 스레드 1(첫 요청 때 생성 · 자기 공급자 캐시 · 자기 SVG 글꼴) · 요청은 도크마다 **마지막 것만**(연타 = 중간 것 건너뜀 =
//! 작업 큐 겸 취소) · 세대 번호로 낡은 결과 폐기 · 결과는 `mpsc` 큐를 유휴 틱이 폴링(30 ms) · 사진은 nexa-ctl 216차 `image_cache::decode_fit`로
//! 표시 상자 크기까지 축소해 UI 스레드가 `put`으로 캐시에 넣는다 · 결과는 도크마다 **키**(선택 수 · 첫 경로 · 스탬프 · 시임 설정)로 보관해
//! 같은 대상의 재계산(폴더 크기 틱 · 상태줄 갱신)을 막는다. 그동안 도크는 "불러오는 중…" 한 줄.

use crate::dockinfo::{self, PreviewOut};
use crate::*;
use nexa_ctl::raster::image_cache::{self, Decoded};
use std::sync::mpsc::{channel, Receiver, Sender};

struct Req {
    gen: u64,
    dock: usize,
    key: String,
    selected: Vec<PathBuf>,
    map: String,
    disabled: String,
    dark: bool,
    w: u32,
    h: u32,
}

enum Msg {
    Job(Req),
    /// 공급자 캐시 버리기(플러그인 설치/삭제 · 설정 창 재구성).
    Invalidate,
}

struct Res {
    gen: u64,
    dock: usize,
    key: String,
    out: PreviewOut,
    decoded: Option<(String, Decoded)>,
}

/// 요청 보내기 · 결과 받기 · 도크별 세대 · 완료 결과 보관.
pub(crate) struct PreviewDecoder {
    tx: Option<Sender<Msg>>,
    rx: Option<Receiver<Res>>,
    gen: [u64; 2],
    /// 지금 워커에 가 있는 요청의 키(같은 키를 다시 요청하지 않는다).
    pending: [Option<String>; 2],
    /// 도크별 마지막 완료 결과(키, 산출) — `update_docks`가 같은 키면 그대로 쓴다.
    ready: [Option<(String, PreviewOut)>; 2],
}

impl PreviewDecoder {
    pub(crate) fn new() -> Self {
        PreviewDecoder {
            tx: None,
            rx: None,
            gen: [0; 2],
            pending: [None, None],
            ready: [None, None],
        }
    }

    /// 워커 스레드는 첫 요청 때 만든다(미리보기를 한 번도 안 보면 스레드 0). `font_face` = UI 글꼴 얼굴(SVG 글자용 · 비면 시스템).
    fn ensure_worker(&mut self, font_face: &str) {
        if self.tx.is_some() {
            return;
        }
        let (tx, rx_req) = channel::<Msg>();
        let (res_tx, rx) = channel::<Res>();
        let face = font_face.to_string();
        std::thread::Builder::new()
            .name("ndir-preview".into())
            .spawn(move || {
                platform::lower_thread_priority();
                // SVG 글자 글꼴(스레드 로컬) — 이 스레드용으로 한 번 더 연다.
                if let Some(l) = nexa_font::ui_font((!face.is_empty()).then_some(face.as_str())) {
                    preview::set_svg_font(std::rc::Rc::new(l.font));
                }
                while let Ok(first) = rx_req.recv() {
                    // 밀린 메시지는 도크마다 마지막 요청만(연타 취소) · 무효화는 즉시.
                    let mut latest: [Option<Req>; 2] = [None, None];
                    let mut take = |m: Msg| match m {
                        Msg::Job(r) => {
                            let d = r.dock.min(1);
                            latest[d] = Some(r);
                        }
                        Msg::Invalidate => preview::invalidate(),
                    };
                    take(first);
                    while let Ok(m) = rx_req.try_recv() {
                        take(m);
                    }
                    for req in latest.into_iter().flatten() {
                        preview::set_dark(req.dark);
                        let out = dockinfo::preview_content(&req.selected, &req.map, &req.disabled);
                        let decoded = out
                            .image
                            .as_ref()
                            .map(|p| (p.clone(), image_cache::decode_fit(p, req.w, req.h)));
                        if res_tx
                            .send(Res {
                                gen: req.gen,
                                dock: req.dock,
                                key: req.key,
                                out,
                                decoded,
                            })
                            .is_err()
                        {
                            return;
                        }
                    }
                }
            })
            .expect("spawn preview worker");
        self.tx = Some(tx);
        self.rx = Some(rx);
    }

    /// 도크 `dock`의 완료 결과(키가 같을 때만).
    pub(crate) fn ready(&self, dock: usize, key: &str) -> Option<&PreviewOut> {
        self.ready[dock.min(1)]
            .as_ref()
            .filter(|(k, _)| k == key)
            .map(|(_, o)| o)
    }

    /// 같은 키가 이미 워커에 가 있는가.
    pub(crate) fn is_pending(&self, dock: usize, key: &str) -> bool {
        self.pending[dock.min(1)].as_deref() == Some(key)
    }

    /// 도크 `dock`에 미리보기 계산을 맡긴다(세대 올림 — 앞선 결과는 버려진다).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn request(
        &mut self,
        dock: usize,
        key: String,
        selected: Vec<PathBuf>,
        map: &str,
        disabled: &str,
        dark: bool,
        (w, h): (u32, u32),
        font_face: &str,
    ) {
        self.ensure_worker(font_face);
        let dock = dock.min(1);
        self.gen[dock] += 1;
        self.pending[dock] = Some(key.clone());
        if let Some(tx) = &self.tx {
            let _ = tx.send(Msg::Job(Req {
                gen: self.gen[dock],
                dock,
                key,
                selected,
                map: map.to_string(),
                disabled: disabled.to_string(),
                dark,
                w,
                h,
            }));
        }
    }

    /// 공급자 캐시 무효화(워커 쪽) + 보관 결과 버리기.
    pub(crate) fn invalidate(&mut self) {
        self.ready = [None, None];
        if let Some(tx) = &self.tx {
            let _ = tx.send(Msg::Invalidate);
        }
    }

    /// 결과 수거(UI 스레드) — 현재 세대의 결과만 보관하고(사진은 캐시에 넣고) 그 도크 번호를 돌려준다.
    pub(crate) fn poll(&mut self) -> Vec<usize> {
        let mut done = Vec::new();
        let Some(rx) = &self.rx else {
            return done;
        };
        while let Ok(res) = rx.try_recv() {
            let dock = res.dock.min(1);
            if res.gen != self.gen[dock] {
                continue; // 낡은 결과(그 사이 다른 파일을 골랐다)
            }
            if let Some((path, d)) = res.decoded {
                let _ = image_cache::put(&path, d);
            }
            self.ready[dock] = Some((res.key, res.out));
            self.pending[dock] = None;
            done.push(dock);
        }
        done
    }

    /// 계산을 기다리는 도크가 있는가(이벤트 루프가 짧게 깬다).
    pub(crate) fn busy(&self) -> bool {
        self.pending.iter().any(Option::is_some)
    }
}

impl App {
    /// 도크 `i`의 사진 표시 상자(px) — `InfoDock::paint`의 `area`와 같은 식(좌우 pad 6 · 위 2 · 아래 2).
    pub(crate) fn dock_image_box(&self, i: usize) -> (u32, u32) {
        let cr = self.docks[i].content_rect();
        ((cr.w - 12).max(1) as u32, (cr.h - 4).max(1) as u32)
    }

    /// 미리보기 결과 키: 선택 수 · 첫 경로 · 파일 스탬프(수정 시각 · 크기 — 바뀐 파일은 다시 계산) · 시임 설정.
    pub(crate) fn preview_key(selected: &[PathBuf], map: &str, disabled: &str) -> String {
        let first = selected.first();
        let stamp = first
            .and_then(|p| std::fs::metadata(p).ok())
            .map(|m| {
                let t = m
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map_or(0, |d| d.as_millis());
                format!("{t}:{}", m.len())
            })
            .unwrap_or_default();
        format!(
            "{}|{}|{stamp}|{map}|{disabled}",
            selected.len(),
            first.map(|p| p.display().to_string()).unwrap_or_default()
        )
    }

    /// 유휴 틱: 끝난 계산이 있으면 도크를 다시 계산한다(`true` = 다시 그릴 것).
    pub(crate) fn preview_decode_tick(&mut self) -> bool {
        if self.preview_decode.poll().is_empty() {
            return false;
        }
        self.update_docks();
        true
    }

    /// 빈 폴더 프로브 결과를 기다린다(덤프·단언 · T4 결정성 · 상한 안에서).
    pub(crate) fn probe_wait(&mut self, limit: Duration) {
        let at = Instant::now();
        while self.panels.iter().any(panel::Panel::probes_pending) && at.elapsed() < limit {
            let now_ms = self.started.elapsed().as_millis() as u64;
            let mut inv = Invalidations::default();
            for p in &mut self.panels {
                p.tick(now_ms, &mut inv);
            }
            if !inv.is_empty() {
                self.redraw();
                continue;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    /// 워커 결과를 기다린다(덤프·단언 · T4 결정성 · 상한 안에서) — 창 없는 시험은 동기 경로라 기다릴 것이 없다.
    pub(crate) fn preview_wait(&mut self, limit: Duration) {
        let at = Instant::now();
        while self.preview_decode.busy() && at.elapsed() < limit {
            if self.preview_decode_tick() {
                continue;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 32bpp top-down BMP(`w`×`h` · 한 색) — 디코더가 읽는 최소 형식(시험 자산).
    pub(crate) fn bmp_bytes(w: u32, h: u32) -> Vec<u8> {
        let row = (w * 4) as usize;
        let size = 54 + row * h as usize;
        let mut b = Vec::with_capacity(size);
        b.extend_from_slice(b"BM");
        b.extend_from_slice(&(size as u32).to_le_bytes());
        b.extend_from_slice(&0u32.to_le_bytes());
        b.extend_from_slice(&54u32.to_le_bytes());
        b.extend_from_slice(&40u32.to_le_bytes());
        b.extend_from_slice(&(w as i32).to_le_bytes());
        b.extend_from_slice(&(-(h as i32)).to_le_bytes());
        b.extend_from_slice(&1u16.to_le_bytes());
        b.extend_from_slice(&32u16.to_le_bytes());
        b.extend_from_slice(&0u32.to_le_bytes());
        b.extend_from_slice(&((row * h as usize) as u32).to_le_bytes());
        b.extend_from_slice(&[0u8; 16]);
        for _ in 0..(w * h) {
            b.extend_from_slice(&[0x40, 0x80, 0xC0, 0xFF]);
        }
        b
    }

    /// 요청 → 워커(텍스트 공급자 · 사진 디코드) → poll = 결과 보관 + 캐시에 상자 크기 사본 · 낡은 세대(연타)는 버림 · 같은 키 재요청 없음 · busy 전이.
    #[test]
    fn worker_computes_latest_request_and_keeps_result() {
        ndir_i18n::activate(ndir_i18n::load("en", std::path::Path::new("nowhere")));
        let dir = std::env::temp_dir().join(format!("ndir-imgjobs-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let a = dir.join("a.txt");
        let b = dir.join("b.bmp");
        std::fs::write(&a, b"hello preview").unwrap();
        std::fs::write(&b, bmp_bytes(60, 30)).unwrap();
        let kb = b.to_string_lossy().into_owned();
        image_cache::clear();
        let mut d = PreviewDecoder::new();
        assert!(!d.busy());
        let ka = App::preview_key(std::slice::from_ref(&a), "", "");
        let kbk = App::preview_key(std::slice::from_ref(&b), "", "");
        d.request(0, ka, vec![a.clone()], "", "", true, (20, 20), "");
        d.request(0, kbk.clone(), vec![b], "", "", true, (20, 20), ""); // 연타 = a는 낡은 세대
        assert!(d.busy() && d.is_pending(0, &kbk));
        let at = Instant::now();
        let mut done = Vec::new();
        while done.is_empty() && at.elapsed() < Duration::from_secs(10) {
            done = d.poll();
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(done, vec![0]);
        assert!(!d.busy());
        let out = d.ready(0, &kbk).expect("ready");
        assert_eq!(out.image.as_deref(), Some(kb.as_str()));
        assert!(image_cache::has_fit(&kb, 20, 20), "사진은 캐시에");
        assert!(d.ready(0, "other").is_none());
        // 텍스트 파일도 워커가 계산한다.
        d.request(1, "t".into(), vec![a], "", "", true, (20, 20), "");
        let at = Instant::now();
        while d.busy() && at.elapsed() < Duration::from_secs(10) {
            let _ = d.poll();
            std::thread::sleep(Duration::from_millis(5));
        }
        let t = d.ready(1, "t").expect("text ready");
        assert!(
            t.provider.contains("text") && t.lines.iter().any(|l| l.contains("hello preview")),
            "{t:?}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
