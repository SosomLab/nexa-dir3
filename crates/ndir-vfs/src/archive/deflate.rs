//! RFC 1951 DEFLATE **압축기**(10-26 · DR-32 ⑦ · 외부 crate 0 · DR-8): LZ77(해시 체인 · 창 32 KiB · 길이 3~258) + 허프먼 블록 —
//! 조각마다 토큰을 모아 **동적 허프먼**(BTYPE = 10 · 블록 전용 부호 길이 ≤ 15 · 부호 길이 알파벳 RLE 16/17/18)과 **고정 허프먼**(BTYPE = 01)의
//! 비트 수를 재어 **작은 쪽**을 낸다(10-26 2차 · 1차 고정 허프먼은 zlib 기본보다 ~20 % 컸다 — 협업 측정). 블록 = 넘긴 조각 하나(스트리밍 ·
//! 조각 사이는 참조하지 않는다 — 조각마다 창을 새로 시작) · 비트는 조각 경계를 넘겨 이어 쓴다.
//! 해제기([`super::extract::inflate`])가 그대로 푼다(왕복 시험) · 표준 해제기 호환 = RFC 1951 그대로(로컬 Python zlib `decompress(…, -15)` 대조 10-26).

/// LZ77 창(RFC 1951 최대 32 KiB).
const WINDOW: usize = 32 * 1024;
const MIN_MATCH: usize = 3;
const MAX_MATCH: usize = 258;
/// 해시 표 크기(2^15).
const HASH_BITS: u32 = 15;
/// 체인 추적 상한(속도/비율 절충).
const MAX_CHAIN: usize = 48;
/// 부호 길이 상한(리터럴/길이 · 거리) · 부호 길이 알파벳 상한.
const MAX_BITS: u8 = 15;
const MAX_CL_BITS: u8 = 7;

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
/// 부호 길이 알파벳이 머리에 실리는 순서(RFC 1951 3.2.7).
const CL_ORDER: [usize; 19] = [
    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
];

/// LZ77 토큰 — 리터럴(`dsym == NO_DIST`) 또는 (길이 부호 + 추가 비트, 거리 부호 + 추가 비트).
#[derive(Clone, Copy, Debug)]
struct Tok {
    /// 리터럴/길이 부호(0..=285).
    sym: u16,
    extra: u16,
    ebits: u8,
    /// 거리 부호(0..=29) · `NO_DIST` = 리터럴.
    dsym: u8,
    dextra: u16,
    debits: u8,
}
const NO_DIST: u8 = 255;

/// 스트리밍 압축기 — [`Deflater::block`]으로 조각을 넣고 [`Deflater::take`]로 완성된 바이트를 꺼낸다 · 끝은 [`Deflater::finish`].
#[derive(Debug, Default)]
pub struct Deflater {
    bits: u64,
    nbits: u32,
    out: Vec<u8>,
    head: Vec<u32>,
    prev: Vec<u32>,
    toks: Vec<Tok>,
    /// 고정 허프먼만(시험 · 비교용).
    fixed_only: bool,
    finished: bool,
}

/// 부호표(길이 → 정규 부호).
struct Codes {
    lens: Vec<u8>,
    codes: Vec<u16>,
}

impl Codes {
    fn from_lens(lens: &[u8]) -> Self {
        let max = lens.iter().copied().max().unwrap_or(0) as usize;
        let mut count = vec![0u32; max + 2];
        for &l in lens {
            if l > 0 {
                count[l as usize] += 1;
            }
        }
        let mut next = vec![0u16; max + 2];
        let mut code = 0u32;
        for bits in 1..=max {
            code = (code + count[bits - 1]) << 1;
            next[bits] = code as u16;
        }
        let mut codes = vec![0u16; lens.len()];
        for (i, &l) in lens.iter().enumerate() {
            if l > 0 {
                codes[i] = next[l as usize];
                next[l as usize] += 1;
            }
        }
        Codes {
            lens: lens.to_vec(),
            codes,
        }
    }
}

/// 빈도 → 길이 제한 허프먼 부호 길이(순수): 힙으로 트리를 만들고 깊이가 `max`를 넘으면 빈도를 반으로 눌러 다시(평평해질수록 얕아진다 ·
/// 전부 1이면 288개도 깊이 9). 쓰이는 기호가 1개면 길이 1(RFC 허용 · 거리 부호 0개도 부호 0을 길이 1로 하나 둔다).
fn huffman_lengths(freq: &[u32], max: u8) -> Vec<u8> {
    let n = freq.len();
    let mut lens = vec![0u8; n];
    let used: Vec<usize> = (0..n).filter(|&i| freq[i] > 0).collect();
    match used.len() {
        0 => return lens,
        1 => {
            lens[used[0]] = 1;
            return lens;
        }
        _ => {}
    }
    let mut f: Vec<u64> = freq.iter().map(|&x| u64::from(x)).collect();
    loop {
        // 노드: (빈도, id) · 리프 id < n · 내부 노드 id ≥ n · parent 표.
        let mut heap: std::collections::BinaryHeap<std::cmp::Reverse<(u64, usize)>> =
            used.iter().map(|&i| std::cmp::Reverse((f[i], i))).collect();
        let mut parent: Vec<usize> = vec![usize::MAX; n + used.len()];
        let mut next_id = n;
        while heap.len() > 1 {
            let (Some(std::cmp::Reverse((fa, a))), Some(std::cmp::Reverse((fb, b)))) =
                (heap.pop(), heap.pop())
            else {
                break; // len() > 1이라 둘 다 있다
            };
            parent[a] = next_id;
            parent[b] = next_id;
            heap.push(std::cmp::Reverse((fa + fb, next_id)));
            next_id += 1;
        }
        let mut too_deep = false;
        for &i in &used {
            let mut d = 0u8;
            let mut p = parent[i];
            while p != usize::MAX {
                d += 1;
                p = parent[p];
            }
            lens[i] = d;
            if d > max {
                too_deep = true;
            }
        }
        if !too_deep {
            return lens;
        }
        for &i in &used {
            f[i] = f[i].div_ceil(2);
        }
    }
}

impl Deflater {
    #[must_use]
    pub fn new() -> Self {
        Deflater {
            head: vec![u32::MAX; 1 << HASH_BITS],
            ..Default::default()
        }
    }

    /// 고정 허프먼만 쓰는 압축기(비교 · 시험).
    #[must_use]
    pub fn fixed_only() -> Self {
        Deflater {
            fixed_only: true,
            ..Self::new()
        }
    }

    /// LSB-first로 `n`비트(추가 비트 · 블록 머리).
    fn put(&mut self, v: u32, n: u32) {
        debug_assert!(n <= 32);
        if n == 0 {
            return;
        }
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

    /// 고정 허프먼 리터럴/길이 부호(RFC 1951 3.2.6) — (부호, 길이).
    fn fixed_lit(sym: u32) -> (u32, u32) {
        match sym {
            0..=143 => (0x30 + sym, 8),
            144..=255 => (0x190 + sym - 144, 9),
            256..=279 => (sym - 256, 7),
            _ => (0xC0 + sym - 280, 8),
        }
    }

    fn hash(d: &[u8], i: usize) -> usize {
        let v = (u32::from(d[i]) << 16) | (u32::from(d[i + 1]) << 8) | u32::from(d[i + 2]);
        (v.wrapping_mul(2_654_435_761) >> (32 - HASH_BITS)) as usize
    }

    fn len_tok(len: usize) -> (u16, u16, u8) {
        let i = LEN_BASE
            .iter()
            .rposition(|&b| usize::from(b) <= len)
            .unwrap_or(0);
        // 258은 285 전용(284의 추가 비트 범위는 227..=257).
        let i = if len == 258 { 28 } else { i.min(27) };
        (
            257 + i as u16,
            (len - usize::from(LEN_BASE[i])) as u16,
            LEN_EXTRA[i],
        )
    }

    fn dist_tok(dist: usize) -> (u8, u16, u8) {
        let i = DIST_BASE
            .iter()
            .rposition(|&b| usize::from(b) <= dist)
            .unwrap_or(0);
        (
            i as u8,
            (dist - usize::from(DIST_BASE[i])) as u16,
            DIST_EXTRA[i],
        )
    }

    /// LZ77 — 조각을 토큰으로(창은 조각 안에서만).
    fn tokenize(&mut self, data: &[u8]) {
        self.toks.clear();
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
                let (sym, extra, ebits) = Self::len_tok(best_len);
                let (dsym, dextra, debits) = Self::dist_tok(best_dist);
                self.toks.push(Tok {
                    sym,
                    extra,
                    ebits,
                    dsym,
                    dextra,
                    debits,
                });
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
                self.toks.push(Tok {
                    sym: u16::from(data[i]),
                    extra: 0,
                    ebits: 0,
                    dsym: NO_DIST,
                    dextra: 0,
                    debits: 0,
                });
                i += 1;
            }
        }
        self.toks.push(Tok {
            sym: 256,
            extra: 0,
            ebits: 0,
            dsym: NO_DIST,
            dextra: 0,
            debits: 0,
        }); // EOB
    }

    /// 부호 길이 배열 → RLE 기호열(부호 길이 알파벳 0..=18 · (기호, 추가값, 추가비트)).
    fn rle_lengths(lens: &[u8]) -> Vec<(u8, u8, u8)> {
        let mut out = Vec::new();
        let mut i = 0;
        while i < lens.len() {
            let l = lens[i];
            let mut run = 1;
            while i + run < lens.len() && lens[i + run] == l {
                run += 1;
            }
            if l == 0 {
                let mut r = run;
                while r >= 3 {
                    if r >= 11 {
                        let n = r.min(138);
                        out.push((18, (n - 11) as u8, 7));
                        r -= n;
                    } else {
                        let n = r.min(10);
                        out.push((17, (n - 3) as u8, 3));
                        r -= n;
                    }
                }
                for _ in 0..r {
                    out.push((0, 0, 0));
                }
            } else {
                out.push((l, 0, 0));
                let mut r = run - 1;
                while r >= 3 {
                    let n = r.min(6);
                    out.push((16, (n - 3) as u8, 2));
                    r -= n;
                }
                for _ in 0..r {
                    out.push((l, 0, 0));
                }
            }
            i += run;
        }
        out
    }

    /// 조각 하나를 **블록 하나**로 눌러 넣는다(`last` = BFINAL). 빈 조각도 블록(EOB만)이 된다. 동적/고정 중 작은 쪽.
    pub fn block(&mut self, data: &[u8], last: bool) {
        assert!(!self.finished, "finish 뒤에는 넣을 수 없다");
        self.tokenize(data);
        // 빈도.
        let mut lf = vec![0u32; 286];
        let mut df = vec![0u32; 30];
        for t in &self.toks {
            lf[t.sym as usize] += 1;
            if t.dsym != NO_DIST {
                df[t.dsym as usize] += 1;
            }
        }
        // 고정 비용.
        let fixed_bits: u64 = self
            .toks
            .iter()
            .map(|t| {
                let (_, l) = Self::fixed_lit(u32::from(t.sym));
                u64::from(l)
                    + u64::from(t.ebits)
                    + if t.dsym == NO_DIST {
                        0
                    } else {
                        5 + u64::from(t.debits)
                    }
            })
            .sum();
        // 동적 부호.
        let mut ll = huffman_lengths(&lf, MAX_BITS);
        let mut dl = huffman_lengths(&df, MAX_BITS);
        if dl.iter().all(|&l| l == 0) {
            dl[0] = 1; // 거리 부호 없음 = 부호 0을 길이 1로 하나(RFC 3.2.7)
        }
        let hlit = ll
            .iter()
            .rposition(|&l| l > 0)
            .map_or(257, |p| (p + 1).max(257));
        let hdist = dl.iter().rposition(|&l| l > 0).map_or(1, |p| p + 1);
        ll.truncate(hlit);
        dl.truncate(hdist);
        let mut all = ll.clone();
        all.extend_from_slice(&dl);
        let rle = Self::rle_lengths(&all);
        let mut cf = vec![0u32; 19];
        for &(s, _, _) in &rle {
            cf[s as usize] += 1;
        }
        let cl = huffman_lengths(&cf, MAX_CL_BITS);
        let hclen = CL_ORDER
            .iter()
            .rposition(|&i| cl[i] > 0)
            .map_or(4, |p| (p + 1).max(4));
        let lcodes = Codes::from_lens(&ll);
        let dcodes = Codes::from_lens(&dl);
        let ccodes = Codes::from_lens(&cl);
        let dyn_bits: u64 = 3
            + 5
            + 5
            + 4
            + 3 * hclen as u64
            + rle
                .iter()
                .map(|&(s, _, eb)| u64::from(cl[s as usize]) + u64::from(eb))
                .sum::<u64>()
            + self
                .toks
                .iter()
                .map(|t| {
                    u64::from(lcodes.lens[t.sym as usize])
                        + u64::from(t.ebits)
                        + if t.dsym == NO_DIST {
                            0
                        } else {
                            u64::from(dcodes.lens[t.dsym as usize]) + u64::from(t.debits)
                        }
                })
                .sum::<u64>();
        let use_dynamic = !self.fixed_only && dyn_bits < fixed_bits + 3;
        self.put(u32::from(last), 1);
        if use_dynamic {
            self.put(2, 2); // BTYPE = 10
            self.put((hlit - 257) as u32, 5);
            self.put((hdist - 1) as u32, 5);
            self.put((hclen - 4) as u32, 4);
            for &i in CL_ORDER.iter().take(hclen) {
                self.put(u32::from(cl[i]), 3);
            }
            for &(s, ev, eb) in &rle {
                let (c, l) = (ccodes.codes[s as usize], ccodes.lens[s as usize]);
                self.put_code(u32::from(c), u32::from(l));
                if eb > 0 {
                    self.put(u32::from(ev), u32::from(eb));
                }
            }
            let toks = std::mem::take(&mut self.toks);
            for t in &toks {
                let (c, l) = (lcodes.codes[t.sym as usize], lcodes.lens[t.sym as usize]);
                self.put_code(u32::from(c), u32::from(l));
                if t.ebits > 0 {
                    self.put(u32::from(t.extra), u32::from(t.ebits));
                }
                if t.dsym != NO_DIST {
                    let (c, l) = (dcodes.codes[t.dsym as usize], dcodes.lens[t.dsym as usize]);
                    self.put_code(u32::from(c), u32::from(l));
                    if t.debits > 0 {
                        self.put(u32::from(t.dextra), u32::from(t.debits));
                    }
                }
            }
            self.toks = toks;
        } else {
            self.put(1, 2); // BTYPE = 01(고정 허프먼)
            let toks = std::mem::take(&mut self.toks);
            for t in &toks {
                let (c, l) = Self::fixed_lit(u32::from(t.sym));
                self.put_code(c, l);
                if t.ebits > 0 {
                    self.put(u32::from(t.extra), u32::from(t.ebits));
                }
                if t.dsym != NO_DIST {
                    self.put_code(u32::from(t.dsym), 5);
                    if t.debits > 0 {
                        self.put(u32::from(t.dextra), u32::from(t.debits));
                    }
                }
            }
            self.toks = toks;
        }
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

    /// 왕복(기존 inflate): 빈 · 반복(크게 줆) · 글 · 무작위(조금 커질 수 있음) · 길이 258/거리 32 KiB 경계 · 조각 스트리밍(비트가 조각 경계를 넘김) ·
    /// 동적 허프먼이 고정보다 작다(글) · 고정 전용도 왕복.
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
        let mut fx = Deflater::fixed_only();
        fx.block(&text, true);
        let f = fx.finish();
        assert_eq!(inflate(&f).unwrap(), text, "고정 전용 왕복");
        assert!(c.len() < f.len(), "동적 {} < 고정 {}", c.len(), f.len());
        // 유사 난수(LCG) 200 KB — 거의 줄지 않지만 커지지도 거의 않는다(동적 = 리터럴 8비트 근처).
        let mut x = 12345u32;
        let rnd: Vec<u8> = (0..200_000)
            .map(|_| {
                x = x.wrapping_mul(1_103_515_245).wrapping_add(12_345);
                (x >> 16) as u8
            })
            .collect();
        let c = roundtrip(&rnd);
        assert!(c.len() <= rnd.len() * 102 / 100, "{}", c.len());
        // 길이 258 · 거리 1 · 거리 32 KiB(창 끝) · 창 밖(32 KiB + 1)은 참조 안 함 · 거리 부호 하나만 쓰는 블록 · 리터럴 하나만 쓰는 블록.
        let mut edge = vec![0u8; 0];
        edge.extend((0..=255u8).cycle().take(WINDOW));
        edge.extend((0..=255u8).cycle().take(300)); // 거리 32768로 일치
        edge.extend(std::iter::repeat_n(b'z', 258 * 3 + 7));
        roundtrip(&edge);
        roundtrip(&[7u8; 5]); // 리터럴 1종 + 길이 1종 + 거리 1종
        roundtrip(b"ab"); // 리터럴 2종 · 거리 없음
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

    /// 허프먼 길이(순수): 빈도 비례 · 상한 안 · 기호 1개 = 길이 1 · 0개 = 전부 0 · 정규 부호는 접두 부호(크래프트 합 ≤ 1).
    #[test]
    fn huffman_lengths_respect_limit_and_prefix_property() {
        let lens = huffman_lengths(&[100, 50, 25, 12, 6, 3, 1, 1], 15);
        assert!(lens[0] < lens[7] && lens.iter().all(|&l| (1..=15).contains(&l)));
        // 피보나치 빈도 40개 = 제한 없는 트리는 깊이 39 → 상한 15 안으로.
        let mut fib = vec![1u32, 1];
        for i in 2..40 {
            fib.push(fib[i - 1].saturating_add(fib[i - 2]));
        }
        let lens = huffman_lengths(&fib, 15);
        assert!(lens.iter().all(|&l| (1..=15).contains(&l)), "{lens:?}");
        let kraft: f64 = lens.iter().map(|&l| 2f64.powi(-i32::from(l))).sum();
        assert!(kraft <= 1.0 + 1e-9, "{kraft}");
        assert_eq!(huffman_lengths(&[0, 5, 0], 15), vec![0, 1, 0]);
        assert_eq!(huffman_lengths(&[0, 0], 15), vec![0, 0]);
        let cl = huffman_lengths(
            &[9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9],
            7,
        );
        assert!(cl.iter().all(|&l| (1..=7).contains(&l)));
    }

    /// 외부 해제기 대조용 표본(수동 · `cargo test -p ndir-vfs dump_deflate_sample -- --ignored` → `python -c "import zlib;…decompress(open(p,'rb').read(), -15)"`).
    #[test]
    #[ignore = "수동 외부 대조"]
    fn dump_deflate_sample() {
        let text: Vec<u8> = (0..500)
            .flat_map(|i| format!("sample {i} — 한글 포함 · deflate 동적 허프먼\n").into_bytes())
            .collect();
        let out = std::env::temp_dir().join("ndir-deflate-sample");
        std::fs::create_dir_all(&out).unwrap();
        std::fs::write(out.join("plain.txt"), &text).unwrap();
        std::fs::write(out.join("plain.deflate"), deflate(&text)).unwrap();
        eprintln!("표본: {}", out.display());
    }
}
