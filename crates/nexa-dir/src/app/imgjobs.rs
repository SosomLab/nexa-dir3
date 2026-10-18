//! 도크 미리보기 **사진 디코드 워커**(10-18 · 사용자 "파일 목록에서 마우스 선택이 키보드보다 체감이 느리다"): 종전에는 사진을 고르면
//! 60 ms 뒤 도크가 그 경로를 받고 **첫 paint가 UI 스레드에서 전체 디코드**(수 MP JPEG/PNG = 수백 ms)를 했다 — 키보드는 연타가 디바운스로
//! 합쳐져 덜 느껴지고, 마우스는 클릭마다 한 장씩 디코드해 매번 멈췄다. 이제 디코드는 이 워커가 하고(nexa-ctl 216차 `image_cache::decode_fit`
//! · 표시 상자 크기로 축소까지) 결과를 UI 스레드가 `put`으로 캐시에 넣은 뒤 도크를 다시 계산한다. 그동안 도크는 "불러오는 중…" 한 줄.
//!
//! 규칙(docs/01 §3): 워커 통지는 `mpsc` 큐를 유휴 틱이 폴링 · 세대 번호로 낡은 결과를 버린다 · 같은 도크의 밀린 요청은 워커가 **마지막 것만**
//! 디코드한다(클릭 연타 = 중간 사진은 건너뜀 = 작업 큐 겸 취소).

use crate::*;
use nexa_ctl::raster::image_cache::{self, Decoded};
use std::sync::mpsc::{channel, Receiver, Sender};

struct Req {
    gen: u64,
    dock: usize,
    path: String,
    w: u32,
    h: u32,
}

struct Res {
    gen: u64,
    dock: usize,
    path: String,
    decoded: Decoded,
}

/// 요청 보내기 · 결과 받기 · 도크별 세대.
pub(crate) struct PreviewDecoder {
    tx: Option<Sender<Req>>,
    rx: Option<Receiver<Res>>,
    gen: [u64; 2],
    pending: [bool; 2],
}

impl PreviewDecoder {
    pub(crate) fn new() -> Self {
        PreviewDecoder {
            tx: None,
            rx: None,
            gen: [0; 2],
            pending: [false; 2],
        }
    }

    /// 워커 스레드는 첫 요청 때 만든다(사진을 한 번도 안 보면 스레드 0).
    fn ensure_worker(&mut self) {
        if self.tx.is_some() {
            return;
        }
        let (tx, rx_req) = channel::<Req>();
        let (res_tx, rx) = channel::<Res>();
        std::thread::Builder::new()
            .name("ndir-preview-decode".into())
            .spawn(move || {
                while let Ok(first) = rx_req.recv() {
                    // 밀린 요청은 도크마다 마지막 것만(연타 취소).
                    let mut latest: [Option<Req>; 2] = [None, None];
                    let d0 = first.dock.min(1);
                    latest[d0] = Some(first);
                    while let Ok(r) = rx_req.try_recv() {
                        let d = r.dock.min(1);
                        latest[d] = Some(r);
                    }
                    for req in latest.into_iter().flatten() {
                        let decoded = image_cache::decode_fit(&req.path, req.w, req.h);
                        if res_tx
                            .send(Res {
                                gen: req.gen,
                                dock: req.dock,
                                path: req.path,
                                decoded,
                            })
                            .is_err()
                        {
                            return;
                        }
                    }
                }
            })
            .expect("spawn preview decode worker");
        self.tx = Some(tx);
        self.rx = Some(rx);
    }

    /// 도크 `dock`에 `path`를 `(w, h)` 상자로 디코드해 달라고(세대 올림 — 앞선 결과는 버려진다).
    pub(crate) fn request(&mut self, dock: usize, path: String, w: u32, h: u32) {
        self.ensure_worker();
        let dock = dock.min(1);
        self.gen[dock] += 1;
        self.pending[dock] = true;
        if let Some(tx) = &self.tx {
            let _ = tx.send(Req {
                gen: self.gen[dock],
                dock,
                path,
                w,
                h,
            });
        }
    }

    /// 결과 수거(UI 스레드) — 현재 세대의 결과만 캐시에 넣고 그 도크 번호를 돌려준다.
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
            let _ = image_cache::put(&res.path, res.decoded);
            self.pending[dock] = false;
            done.push(dock);
        }
        done
    }

    /// 디코드를 기다리는 도크가 있는가(이벤트 루프가 짧게 깬다).
    pub(crate) fn busy(&self) -> bool {
        self.pending.iter().any(|p| *p)
    }
}

impl App {
    /// 도크 `i`의 사진 표시 상자(px) — `InfoDock::paint`의 `area`와 같은 식(좌우 pad 6 · 위 2 · 아래 2).
    pub(crate) fn dock_image_box(&self, i: usize) -> (u32, u32) {
        let cr = self.docks[i].content_rect();
        ((cr.w - 12).max(1) as u32, (cr.h - 4).max(1) as u32)
    }

    /// 유휴 틱: 끝난 디코드가 있으면 캐시에 넣고 도크를 다시 계산한다(`true` = 다시 그릴 것).
    pub(crate) fn preview_decode_tick(&mut self) -> bool {
        if self.preview_decode.poll().is_empty() {
            return false;
        }
        self.update_docks();
        true
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

    /// 요청 → 워커 디코드 → poll = 캐시에 상자 크기 사본 · 낡은 세대(연타) 결과는 버림 · busy 전이.
    #[test]
    fn worker_decodes_latest_request_and_fills_cache() {
        let dir = std::env::temp_dir().join(format!("ndir-imgjobs-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let a = dir.join("a.bmp");
        let b = dir.join("b.bmp");
        std::fs::write(&a, bmp_bytes(40, 20)).unwrap();
        std::fs::write(&b, bmp_bytes(60, 30)).unwrap();
        let (ka, kb) = (
            a.to_string_lossy().into_owned(),
            b.to_string_lossy().into_owned(),
        );
        image_cache::clear();
        let mut d = PreviewDecoder::new();
        assert!(!d.busy());
        d.request(0, ka, 20, 20);
        d.request(0, kb.clone(), 20, 20); // 연타 = a는 낡은 세대
        assert!(d.busy());
        let at = Instant::now();
        let mut done = Vec::new();
        while done.is_empty() && at.elapsed() < Duration::from_secs(5) {
            done = d.poll();
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(done, vec![0]);
        assert!(!d.busy());
        assert!(image_cache::has_fit(&kb, 20, 20), "마지막 요청 = 캐시");
        let img = image_cache::get_fit(&kb, 20, 20).unwrap();
        assert_eq!((img.w, img.h), (20, 10));
        // a는 워커가 건너뛰었거나 낡은 세대로 버려졌다 — 어느 쪽이든 b가 현재.
        let _ = std::fs::remove_dir_all(&dir);
    }
}
