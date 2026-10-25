//! RFC 1951 DEFLATE **압축기**(10-26 · DR-32 ⑦ · 외부 crate 0 · DR-8): LZ77(해시 체인 · 창 32 KiB · 길이 3~258) + **고정 허프먼** 블록(BTYPE = 01).
//! 블록 = 넘긴 조각 하나(스트리밍 · 조각 사이는 참조하지 않는다 — 조각마다 창을 새로 시작) · 비트는 조각 경계를 넘겨 이어 쓴다.
//! 해제기([`super::extract::inflate`])가 그대로 푼다(왕복 시험) · 표준 해제기 호환 = RFC 1951 그대로(로컬 Python zlib `decompress(…, -15)` 대조 10-26).
//! 동적 허프먼은 두지 않았다(코드 양 대비 이득이 작다 — 후속 후보).

/// LZ77 창(RFC 1951 최대 32 KiB).
const WINDOW: usize = 32 * 1024;
const MIN_MATCH: usize = 3;
const MAX_MATCH: usize = 258;
/// 해시 표 크기(2^15).
const HASH_BITS: u32 = 15;
/// 체인 추적 상한(속도/비율 절충).
const MAX_CHAIN: usize = 48;

/// 길이 부호(257..=285)의 바탕 길이 · 추가 비트.
const LEN_BASE: [u16; 29] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258,
];
const LEN_EXTRA: [u8; 29] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
];
/// 거리 부호(0..=29)의 바탕 거리 · 추가 비트.
const DIST_BASE: [u16; 30] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
];
const DIST_EXTRA: [u8; 30] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13,
];

/// 스트리밍 압축기 — [`Deflater::block`]으로 조각을 넣고 [`Deflater::take`]로 완성된 바이트를 꺼낸다 · 끝은 [`Deflater::finish`].
#[derive(Debug, Default)]
pub struct Deflater {
    bits: u64,
    nbits: u32,
    out: Vec<u8>,
    head: Vec<u32>,
    prev: Vec<u32>,
    finished: bool,
}

impl Deflater {
    #[must_use]
    pub fn new() -> Self {
        Deflater {
            head: vec![u32::MAX; 1 << HASH_BITS],
            ..Default::default()
        }
    }

    /// LSB-first로 `n`비트(추가 비트 · 블록 머리).
    fn put(&mut self, v: u32, n: u32) {
        debug_assert!(n <= 32);
        self.bits |= u64::from(v) << self.nbits;
        self.nbits += n;
        while self.nbits >= 8 {
            self.out.push(self.bits as u8);
            self.bits >>= 8;
            self.nbits -= 8;
        }
    }

    /// 허프먼 부호는 **MSB부터** 흘러간다 → 뒤집어 넣는다.
    fn put_code(&mut self, code: u32, len: u32) {
        let mut rev = 0u32;
        for i in 0..len {
            rev |= ((code >> i) & 1) << (len - 1 - i);
        }
        self.put(rev, len);
    }

    /// 고정 허프먼 리터럴/길이 부호(RFC 1951 3.2.6).
    fn put_lit(&mut self, sym: u32) {
        match sym {
            0..=143 => self.put_code(0x30 + sym, 8),
            144..=255 => self.put_code(0x190 + sym - 144, 9),
            256..=279 => self.put_code(sym - 256, 7),
            _ => self.put_code(0xC0 + sym - 280, 8),
        }
    }

    fn put_length(&mut self, len: usize) {
        let i = LEN_BASE
            .iter()
            .rposition(|&b| usize::from(b) <= len)
            .unwrap_or(0);
        // 258은 285 전용(284의 추가 비트 범위는 227..=257).
        let i = if len == 258 { 28 } else { i.min(27) };
        self.put_lit(257 + i as u32);
        let extra = LEN_EXTRA[i];
        if extra > 0 {
            self.put((len - usize::from(LEN_BASE[i])) as u32, u32::from(extra));
        }
    }

    fn put_distance(&mut self, dist: usize) {
        let i = DIST_BASE
            .iter()
            .rposition(|&b| usize::from(b) <= dist)
            .unwrap_or(0);
        self.put_code(i as u32, 5);
        let extra = DIST_EXTRA[i];
        if extra > 0 {
            self.put((dist - usize::from(DIST_BASE[i])) as u32, u32::from(extra));
        }
    }

    fn hash(d: &[u8], i: usize) -> usize {
        let v = (u32::from(d[i]) << 16) | (u32::from(d[i + 1]) << 8) | u32::from(d[i + 2]);
        (v.wrapping_mul(2_654_435_761) >> (32 - HASH_BITS)) as usize
    }

    /// 조각 하나를 **블록 하나**로 눌러 넣는다(`last` = BFINAL). 빈 조각도 블록(EOB만)이 된다.
    pub fn block(&mut self, data: &[u8], last: bool) {
        assert!(!self.finished, "finish 뒤에는 넣을 수 없다");
        self.put(u32::from(last), 1);
        self.put(1, 2); // BTYPE = 01(고정 허프먼)
        self.head.fill(u32::MAX);
        if self.prev.len() < data.len() {
            self.prev.resize(data.len(), u32::MAX);
        }
        let n = data.len();
        let mut i = 0usize;
        while i < n {
            let mut best_len = 0usize;
            let mut best_dist = 0usize;
            if i + MIN_MATCH <= n {
                let h = Self::hash(data, i);
                let mut cand = self.head[h];
                let mut steps = 0;
                while cand != u32::MAX && steps < MAX_CHAIN {
                    let c = cand as usize;
                    let dist = i - c;
                    if dist > WINDOW {
                        break;
                    }
                    let max = (n - i).min(MAX_MATCH);
                    let mut l = 0usize;
                    while l < max && data[c + l] == data[i + l] {
                        l += 1;
                    }
                    if l > best_len {
                        best_len = l;
                        best_dist = dist;
                        if l == max {
                            break;
                        }
                    }
                    cand = self.prev[c];
                    steps += 1;
                }
                self.prev[i] = self.head[h];
                self.head[h] = i as u32;
            }
            if best_len >= MIN_MATCH {
                self.put_length(best_len);
                self.put_distance(best_dist);
                // 건너뛴 자리도 해시에 넣는다(다음 일치를 위해).
                for k in 1..best_len {
                    let p = i + k;
                    if p + MIN_MATCH <= n {
                        let h = Self::hash(data, p);
                        self.prev[p] = self.head[h];
                        self.head[h] = p as u32;
                    }
                }
                i += best_len;
            } else {
                self.put_lit(u32::from(data[i]));
                i += 1;
            }
        }
        self.put_lit(256); // EOB
    }

    /// 지금까지 완성된 바이트(스트리밍 쓰기 · 비트 잔여는 남긴다).
    pub fn take(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.out)
    }

    /// 끝 — 비트 잔여를 0으로 채워 마지막 바이트까지 돌려준다. 블록을 하나도 넣지 않았으면 빈 최종 블록을 낸다.
    pub fn finish(mut self) -> Vec<u8> {
        if self.out.is_empty() && self.nbits == 0 {
            self.block(&[], true);
        }
        if self.nbits > 0 {
            self.out.push(self.bits as u8);
            self.bits = 0;
            self.nbits = 0;
        }
        self.finished = true;
        std::mem::take(&mut self.out)
    }
}

/// 한 번에(전부 메모리 · 블록 하나).
#[must_use]
pub fn deflate(data: &[u8]) -> Vec<u8> {
    let mut d = Deflater::new();
    d.block(data, true);
    d.finish()
}

#[cfg(test)]
mod tests {
    use super::super::extract::inflate;
    use super::*;

    fn roundtrip(data: &[u8]) -> Vec<u8> {
        let c = deflate(data);
        let back = inflate(&c).expect("inflate");
        assert_eq!(back, data, "왕복");
        c
    }

    /// 왕복(기존 inflate): 빈 · 반복(크게 줆) · 글 · 무작위(조금 커질 수 있음) · 길이 258/거리 32 KiB 경계 · 조각 스트리밍(비트가 조각 경계를 넘김).
    #[test]
    fn deflate_round_trips_through_inflate() {
        assert_eq!(inflate(&deflate(b"")).unwrap(), b"");
        let rep = vec![b'a'; 100_000];
        let c = roundtrip(&rep);
        assert!(c.len() < 1_000, "반복은 크게 준다: {}", c.len());
        let text: Vec<u8> = (0..2_000)
            .flat_map(|i| {
                format!("line {i}: the quick brown fox jumps over the lazy dog\n").into_bytes()
            })
            .collect();
        let c = roundtrip(&text);
        assert!(
            c.len() * 3 < text.len(),
            "글은 1/3 아래: {} / {}",
            c.len(),
            text.len()
        );
        // 유사 난수(LCG) 200 KB — 고정 허프먼은 9비트 리터럴 때문에 커질 수 있다(≤ +13 %) → zip 쪽이 store로 되돌린다.
        let mut x = 12345u32;
        let rnd: Vec<u8> = (0..200_000)
            .map(|_| {
                x = x.wrapping_mul(1_103_515_245).wrapping_add(12_345);
                (x >> 16) as u8
            })
            .collect();
        let c = roundtrip(&rnd);
        assert!(c.len() <= rnd.len() * 113 / 100, "{}", c.len());
        // 길이 258 · 거리 1 · 거리 32 KiB(창 끝) · 창 밖(32 KiB + 1)은 참조 안 함.
        let mut edge = vec![0u8; 0];
        edge.extend((0..=255u8).cycle().take(WINDOW));
        edge.extend((0..=255u8).cycle().take(300)); // 거리 32768로 일치
        edge.extend(std::iter::repeat_n(b'z', 258 * 3 + 7));
        roundtrip(&edge);
        // 스트리밍: 조각 1,000바이트씩 · 마지막은 빈 최종 블록 — 한 번에 누른 것과 같은 원본으로 풀린다.
        let mut d = Deflater::new();
        let mut out = Vec::new();
        for chunk in text.chunks(1_000) {
            d.block(chunk, false);
            out.extend(d.take());
        }
        d.block(&[], true);
        out.extend(d.finish());
        assert_eq!(inflate(&out).unwrap(), text, "조각 스트리밍 왕복");
        // 넣은 것 없이 finish = 빈 최종 블록.
        assert_eq!(inflate(&Deflater::new().finish()).unwrap(), b"");
    }

    /// 외부 해제기 대조용 표본(수동 · `cargo test -p ndir-vfs dump_deflate_sample -- --ignored` → `python -c "import zlib;…decompress(open(p,'rb').read(), -15)"`).
    #[test]
    #[ignore = "수동 외부 대조"]
    fn dump_deflate_sample() {
        let text: Vec<u8> = (0..500)
            .flat_map(|i| format!("sample {i} — 한글 포함 · deflate 고정 허프먼\n").into_bytes())
            .collect();
        let out = std::env::temp_dir().join("ndir-deflate-sample");
        std::fs::create_dir_all(&out).unwrap();
        std::fs::write(out.join("plain.txt"), &text).unwrap();
        std::fs::write(out.join("plain.deflate"), deflate(&text)).unwrap();
        eprintln!("표본: {}", out.display());
    }
}
