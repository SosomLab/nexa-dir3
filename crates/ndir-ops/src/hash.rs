//! 체크섬(T-167 · NEW-038 · dir3 신규 · 사용자 10-05) — CRC32 · MD5 · SHA-1 · SHA-256 · SHA-512를 **외부 crate 없이**(DR-8) 한 번
//! 읽으며 함께 계산한다. 스트리밍(조각 입력) · 취소 · 바이트 진행 통지. 알고리즘 구현은 각 표준(RFC 1321 · RFC 3174 · FIPS 180-4)의
//! 참조 절차 그대로 — 성능보다 명료함을 택했고, 디스크 읽기가 보통 더 느리다(이 PC 실측은 창 쪽 문서에).

use std::fmt::Write as _;
use std::io::{self, Read};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

/// 지원 알고리즘(표시 순서).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Algo {
    Crc32,
    Md5,
    Sha1,
    Sha256,
    Sha512,
}

impl Algo {
    pub const ALL: [Algo; 5] = [
        Algo::Crc32,
        Algo::Md5,
        Algo::Sha1,
        Algo::Sha256,
        Algo::Sha512,
    ];

    /// 표시 이름(고유 명칭 — i18n 대상 아님).
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Algo::Crc32 => "CRC32",
            Algo::Md5 => "MD5",
            Algo::Sha1 => "SHA-1",
            Algo::Sha256 => "SHA-256",
            Algo::Sha512 => "SHA-512",
        }
    }

    /// 설정 · 저장용 키.
    #[must_use]
    pub fn key(self) -> &'static str {
        match self {
            Algo::Crc32 => "crc32",
            Algo::Md5 => "md5",
            Algo::Sha1 => "sha1",
            Algo::Sha256 => "sha256",
            Algo::Sha512 => "sha512",
        }
    }

    #[must_use]
    pub fn from_key(k: &str) -> Option<Algo> {
        Algo::ALL.into_iter().find(|a| a.key() == k)
    }

    /// 16진 결과 글자 수(비교 입력의 알고리즘 추정용).
    #[must_use]
    pub fn hex_len(self) -> usize {
        match self {
            Algo::Crc32 => 8,
            Algo::Md5 => 32,
            Algo::Sha1 => 40,
            Algo::Sha256 => 64,
            Algo::Sha512 => 128,
        }
    }
}

/// 조각을 받아 상태를 갱신하고 끝에 16진 소문자 다이제스트를 낸다.
pub trait Digest {
    fn update(&mut self, data: &[u8]);
    fn finish(self: Box<Self>) -> String;
}

/// 알고리즘 → 새 계산기.
#[must_use]
pub fn new_digest(a: Algo) -> Box<dyn Digest> {
    match a {
        Algo::Crc32 => Box::new(Crc32::default()),
        Algo::Md5 => Box::new(Md5::default()),
        Algo::Sha1 => Box::new(Sha1::default()),
        Algo::Sha256 => Box::new(Sha256::default()),
        Algo::Sha512 => Box::new(Sha512::default()),
    }
}

/// 한 번에(시험 · 짧은 입력).
#[must_use]
pub fn digest(a: Algo, data: &[u8]) -> String {
    let mut d = new_digest(a);
    d.update(data);
    d.finish()
}

fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let _ = write!(s, "{b:02x}");
    }
    s
}

/// 사용자가 붙여 넣은 글이 어느 알고리즘의 16진 결과 모양인가(공백 · 하이픈 · 콜론은 무시 · 대소문자 무관). 길이가 어느 것에도
/// 맞지 않으면 `None`.
#[must_use]
pub fn guess_algo(text: &str) -> Option<Algo> {
    let n = normalize_hex(text)?.len();
    Algo::ALL.into_iter().find(|a| a.hex_len() == n)
}

/// 16진 글 정규화(구분 문자 제거 · 소문자) — 16진이 아닌 글자가 있으면 `None`.
#[must_use]
pub fn normalize_hex(text: &str) -> Option<String> {
    let mut out = String::new();
    for c in text.chars() {
        if c.is_ascii_hexdigit() {
            out.push(c.to_ascii_lowercase());
        } else if !(c.is_whitespace() || matches!(c, '-' | ':' | '_')) {
            return None;
        }
    }
    (!out.is_empty()).then_some(out)
}

/// 파일을 한 번 읽으며 여러 알고리즘을 함께 계산한다 — `on_bytes` = 읽은 증분 · `cancel` = `Interrupted`.
pub fn hash_file(
    path: &Path,
    algos: &[Algo],
    on_bytes: &mut dyn FnMut(u64),
    cancel: &AtomicBool,
) -> io::Result<Vec<(Algo, String)>> {
    let mut f = std::fs::File::open(path)?;
    let mut ds: Vec<(Algo, Box<dyn Digest>)> = algos.iter().map(|&a| (a, new_digest(a))).collect();
    let mut buf = vec![0u8; 1024 * 1024];
    loop {
        if cancel.load(Ordering::Relaxed) {
            return Err(io::Error::new(io::ErrorKind::Interrupted, "canceled"));
        }
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        for (_, d) in &mut ds {
            d.update(&buf[..n]);
        }
        on_bytes(n as u64);
    }
    Ok(ds.into_iter().map(|(a, d)| (a, d.finish())).collect())
}

// ── CRC32(IEEE 802.3 · 반사형 · 다항식 0xEDB88320) ────────────────────────────────

const CRC_TABLE: [u32; 256] = {
    let mut t = [0u32; 256];
    let mut i = 0;
    while i < 256 {
        let mut c = i as u32;
        let mut k = 0;
        while k < 8 {
            c = if c & 1 != 0 {
                0xEDB8_8320 ^ (c >> 1)
            } else {
                c >> 1
            };
            k += 1;
        }
        t[i] = c;
        i += 1;
    }
    t
};

#[derive(Debug)]
struct Crc32(u32);

impl Default for Crc32 {
    fn default() -> Self {
        Crc32(0xFFFF_FFFF)
    }
}

impl Digest for Crc32 {
    fn update(&mut self, data: &[u8]) {
        let mut c = self.0;
        for &b in data {
            c = CRC_TABLE[((c ^ u32::from(b)) & 0xFF) as usize] ^ (c >> 8);
        }
        self.0 = c;
    }
    fn finish(self: Box<Self>) -> String {
        format!("{:08x}", !self.0)
    }
}

// ── 블록 버퍼(MD5 · SHA 공용) ───────────────────────────────────────────────────

/// 고정 블록 단위로 `process`를 부르는 버퍼 — 남는 조각은 다음 조각과 이어 붙인다.
struct Blocks<const N: usize> {
    buf: [u8; N],
    len: usize,
    /// 지금까지 받은 총 바이트.
    total: u64,
}

impl<const N: usize> Blocks<N> {
    fn new() -> Self {
        Blocks {
            buf: [0; N],
            len: 0,
            total: 0,
        }
    }

    fn feed(&mut self, mut data: &[u8], mut process: impl FnMut(&[u8; N])) {
        self.total += data.len() as u64;
        if self.len > 0 {
            let take = (N - self.len).min(data.len());
            self.buf[self.len..self.len + take].copy_from_slice(&data[..take]);
            self.len += take;
            data = &data[take..];
            if self.len == N {
                process(&self.buf);
                self.len = 0;
            }
        }
        while data.len() >= N {
            let mut block = [0u8; N];
            block.copy_from_slice(&data[..N]);
            process(&block);
            data = &data[N..];
        }
        if !data.is_empty() {
            self.buf[..data.len()].copy_from_slice(data);
            self.len = data.len();
        }
    }

    /// 패딩: 0x80 · 0 채움 · 마지막 `len_bytes`바이트에 비트 길이(`big`이면 빅 엔디언). 블록 1 ~ 2개를 `process`한다.
    fn pad(&mut self, len_bytes: usize, big: bool, mut process: impl FnMut(&[u8; N])) {
        let bits = self.total.wrapping_mul(8);
        let mut tail = vec![0x80u8];
        let mut need = N - ((self.len + 1) % N);
        if need < len_bytes {
            need += N;
        }
        tail.extend(std::iter::repeat_n(0u8, need - len_bytes));
        // 비트 길이는 64비트만 쓴다(SHA-512의 128비트 길이 칸의 윗 64비트는 0 — 2^64비트 이상의 파일은 없다).
        let mut lenb = vec![0u8; len_bytes - 8];
        if big {
            lenb.extend_from_slice(&bits.to_be_bytes());
        } else {
            lenb = bits.to_le_bytes().to_vec();
            lenb.extend(std::iter::repeat_n(0u8, len_bytes - 8));
        }
        tail.extend_from_slice(&lenb);
        let total = self.total;
        self.feed(&tail, &mut process);
        self.total = total;
        debug_assert_eq!(self.len, 0);
    }
}

// ── MD5(RFC 1321) ──────────────────────────────────────────────────────────────

const MD5_S: [u32; 64] = [
    7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9,
    14, 20, 5, 9, 14, 20, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10, 15,
    21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
];

const MD5_K: [u32; 64] = {
    // K[i] = floor(2^32 × |sin(i + 1)|) — 상수표(RFC 1321 §3.4).
    [
        0xd76a_a478,
        0xe8c7_b756,
        0x2420_70db,
        0xc1bd_ceee,
        0xf57c_0faf,
        0x4787_c62a,
        0xa830_4613,
        0xfd46_9501,
        0x6980_98d8,
        0x8b44_f7af,
        0xffff_5bb1,
        0x895c_d7be,
        0x6b90_1122,
        0xfd98_7193,
        0xa679_438e,
        0x49b4_0821,
        0xf61e_2562,
        0xc040_b340,
        0x265e_5a51,
        0xe9b6_c7aa,
        0xd62f_105d,
        0x0244_1453,
        0xd8a1_e681,
        0xe7d3_fbc8,
        0x21e1_cde6,
        0xc337_07d6,
        0xf4d5_0d87,
        0x455a_14ed,
        0xa9e3_e905,
        0xfcef_a3f8,
        0x676f_02d9,
        0x8d2a_4c8a,
        0xfffa_3942,
        0x8771_f681,
        0x6d9d_6122,
        0xfde5_380c,
        0xa4be_ea44,
        0x4bde_cfa9,
        0xf6bb_4b60,
        0xbebf_bc70,
        0x289b_7ec6,
        0xeaa1_27fa,
        0xd4ef_3085,
        0x0488_1d05,
        0xd9d4_d039,
        0xe6db_99e5,
        0x1fa2_7cf8,
        0xc4ac_5665,
        0xf429_2244,
        0x432a_ff97,
        0xab94_23a7,
        0xfc93_a039,
        0x655b_59c3,
        0x8f0c_cc92,
        0xffef_f47d,
        0x8584_5dd1,
        0x6fa8_7e4f,
        0xfe2c_e6e0,
        0xa301_4314,
        0x4e08_11a1,
        0xf753_7e82,
        0xbd3a_f235,
        0x2ad7_d2bb,
        0xeb86_d391,
    ]
};

struct Md5 {
    st: [u32; 4],
    blk: Blocks<64>,
}

impl Default for Md5 {
    fn default() -> Self {
        Md5 {
            st: [0x6745_2301, 0xefcd_ab89, 0x98ba_dcfe, 0x1032_5476],
            blk: Blocks::new(),
        }
    }
}

fn md5_block(st: &mut [u32; 4], block: &[u8; 64]) {
    let mut m = [0u32; 16];
    for (i, w) in m.iter_mut().enumerate() {
        *w = u32::from_le_bytes([
            block[i * 4],
            block[i * 4 + 1],
            block[i * 4 + 2],
            block[i * 4 + 3],
        ]);
    }
    let [mut a, mut b, mut c, mut d] = *st;
    for i in 0..64 {
        let (f, g) = match i / 16 {
            0 => ((b & c) | (!b & d), i),
            1 => ((d & b) | (!d & c), (5 * i + 1) % 16),
            2 => (b ^ c ^ d, (3 * i + 5) % 16),
            _ => (c ^ (b | !d), (7 * i) % 16),
        };
        let f = f.wrapping_add(a).wrapping_add(MD5_K[i]).wrapping_add(m[g]);
        a = d;
        d = c;
        c = b;
        b = b.wrapping_add(f.rotate_left(MD5_S[i]));
    }
    st[0] = st[0].wrapping_add(a);
    st[1] = st[1].wrapping_add(b);
    st[2] = st[2].wrapping_add(c);
    st[3] = st[3].wrapping_add(d);
}

impl Digest for Md5 {
    fn update(&mut self, data: &[u8]) {
        let st = &mut self.st;
        self.blk.feed(data, |b| md5_block(st, b));
    }
    fn finish(mut self: Box<Self>) -> String {
        let st = &mut self.st;
        self.blk.pad(8, false, |b| md5_block(st, b));
        let mut out = Vec::with_capacity(16);
        for w in self.st {
            out.extend_from_slice(&w.to_le_bytes());
        }
        hex(&out)
    }
}

// ── SHA-1(RFC 3174) ────────────────────────────────────────────────────────────

struct Sha1 {
    st: [u32; 5],
    blk: Blocks<64>,
}

impl Default for Sha1 {
    fn default() -> Self {
        Sha1 {
            st: [
                0x6745_2301,
                0xefcd_ab89,
                0x98ba_dcfe,
                0x1032_5476,
                0xc3d2_e1f0,
            ],
            blk: Blocks::new(),
        }
    }
}

fn be_words(block: &[u8], n: usize) -> Vec<u32> {
    (0..n)
        .map(|i| {
            u32::from_be_bytes([
                block[i * 4],
                block[i * 4 + 1],
                block[i * 4 + 2],
                block[i * 4 + 3],
            ])
        })
        .collect()
}

fn sha1_block(st: &mut [u32; 5], block: &[u8; 64]) {
    let mut w = [0u32; 80];
    w[..16].copy_from_slice(&be_words(block, 16));
    for i in 16..80 {
        w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
    }
    let [mut a, mut b, mut c, mut d, mut e] = *st;
    for (i, wi) in w.iter().enumerate() {
        let (f, k) = match i / 20 {
            0 => ((b & c) | (!b & d), 0x5a82_7999),
            1 => (b ^ c ^ d, 0x6ed9_eba1),
            2 => ((b & c) | (b & d) | (c & d), 0x8f1b_bcdc),
            _ => (b ^ c ^ d, 0xca62_c1d6),
        };
        let t = a
            .rotate_left(5)
            .wrapping_add(f)
            .wrapping_add(e)
            .wrapping_add(k)
            .wrapping_add(*wi);
        e = d;
        d = c;
        c = b.rotate_left(30);
        b = a;
        a = t;
    }
    for (s, v) in st.iter_mut().zip([a, b, c, d, e]) {
        *s = s.wrapping_add(v);
    }
}

impl Digest for Sha1 {
    fn update(&mut self, data: &[u8]) {
        let st = &mut self.st;
        self.blk.feed(data, |b| sha1_block(st, b));
    }
    fn finish(mut self: Box<Self>) -> String {
        let st = &mut self.st;
        self.blk.pad(8, true, |b| sha1_block(st, b));
        let mut out = Vec::with_capacity(20);
        for w in self.st {
            out.extend_from_slice(&w.to_be_bytes());
        }
        hex(&out)
    }
}

// ── SHA-256(FIPS 180-4) ────────────────────────────────────────────────────────

const K256: [u32; 64] = [
    0x428a_2f98,
    0x7137_4491,
    0xb5c0_fbcf,
    0xe9b5_dba5,
    0x3956_c25b,
    0x59f1_11f1,
    0x923f_82a4,
    0xab1c_5ed5,
    0xd807_aa98,
    0x1283_5b01,
    0x2431_85be,
    0x550c_7dc3,
    0x72be_5d74,
    0x80de_b1fe,
    0x9bdc_06a7,
    0xc19b_f174,
    0xe49b_69c1,
    0xefbe_4786,
    0x0fc1_9dc6,
    0x240c_a1cc,
    0x2de9_2c6f,
    0x4a74_84aa,
    0x5cb0_a9dc,
    0x76f9_88da,
    0x983e_5152,
    0xa831_c66d,
    0xb003_27c8,
    0xbf59_7fc7,
    0xc6e0_0bf3,
    0xd5a7_9147,
    0x06ca_6351,
    0x1429_2967,
    0x27b7_0a85,
    0x2e1b_2138,
    0x4d2c_6dfc,
    0x5338_0d13,
    0x650a_7354,
    0x766a_0abb,
    0x81c2_c92e,
    0x9272_2c85,
    0xa2bf_e8a1,
    0xa81a_664b,
    0xc24b_8b70,
    0xc76c_51a3,
    0xd192_e819,
    0xd699_0624,
    0xf40e_3585,
    0x106a_a070,
    0x19a4_c116,
    0x1e37_6c08,
    0x2748_774c,
    0x34b0_bcb5,
    0x391c_0cb3,
    0x4ed8_aa4a,
    0x5b9c_ca4f,
    0x682e_6ff3,
    0x748f_82ee,
    0x78a5_636f,
    0x84c8_7814,
    0x8cc7_0208,
    0x90be_fffa,
    0xa450_6ceb,
    0xbef9_a3f7,
    0xc671_78f2,
];

struct Sha256 {
    st: [u32; 8],
    blk: Blocks<64>,
}

impl Default for Sha256 {
    fn default() -> Self {
        Sha256 {
            st: [
                0x6a09_e667,
                0xbb67_ae85,
                0x3c6e_f372,
                0xa54f_f53a,
                0x510e_527f,
                0x9b05_688c,
                0x1f83_d9ab,
                0x5be0_cd19,
            ],
            blk: Blocks::new(),
        }
    }
}

fn sha256_block(st: &mut [u32; 8], block: &[u8; 64]) {
    let mut w = [0u32; 64];
    w[..16].copy_from_slice(&be_words(block, 16));
    for i in 16..64 {
        let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
        let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
        w[i] = w[i - 16]
            .wrapping_add(s0)
            .wrapping_add(w[i - 7])
            .wrapping_add(s1);
    }
    let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = *st;
    for i in 0..64 {
        let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
        let ch = (e & f) ^ (!e & g);
        let t1 = h
            .wrapping_add(s1)
            .wrapping_add(ch)
            .wrapping_add(K256[i])
            .wrapping_add(w[i]);
        let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
        let maj = (a & b) ^ (a & c) ^ (b & c);
        let t2 = s0.wrapping_add(maj);
        h = g;
        g = f;
        f = e;
        e = d.wrapping_add(t1);
        d = c;
        c = b;
        b = a;
        a = t1.wrapping_add(t2);
    }
    for (s, v) in st.iter_mut().zip([a, b, c, d, e, f, g, h]) {
        *s = s.wrapping_add(v);
    }
}

impl Digest for Sha256 {
    fn update(&mut self, data: &[u8]) {
        let st = &mut self.st;
        self.blk.feed(data, |b| sha256_block(st, b));
    }
    fn finish(mut self: Box<Self>) -> String {
        let st = &mut self.st;
        self.blk.pad(8, true, |b| sha256_block(st, b));
        let mut out = Vec::with_capacity(32);
        for w in self.st {
            out.extend_from_slice(&w.to_be_bytes());
        }
        hex(&out)
    }
}

// ── SHA-512(FIPS 180-4) ────────────────────────────────────────────────────────

const K512: [u64; 80] = [
    0x428a_2f98_d728_ae22,
    0x7137_4491_23ef_65cd,
    0xb5c0_fbcf_ec4d_3b2f,
    0xe9b5_dba5_8189_dbbc,
    0x3956_c25b_f348_b538,
    0x59f1_11f1_b605_d019,
    0x923f_82a4_af19_4f9b,
    0xab1c_5ed5_da6d_8118,
    0xd807_aa98_a303_0242,
    0x1283_5b01_4570_6fbe,
    0x2431_85be_4ee4_b28c,
    0x550c_7dc3_d5ff_b4e2,
    0x72be_5d74_f27b_896f,
    0x80de_b1fe_3b16_96b1,
    0x9bdc_06a7_25c7_1235,
    0xc19b_f174_cf69_2694,
    0xe49b_69c1_9ef1_4ad2,
    0xefbe_4786_384f_25e3,
    0x0fc1_9dc6_8b8c_d5b5,
    0x240c_a1cc_77ac_9c65,
    0x2de9_2c6f_592b_0275,
    0x4a74_84aa_6ea6_e483,
    0x5cb0_a9dc_bd41_fbd4,
    0x76f9_88da_8311_53b5,
    0x983e_5152_ee66_dfab,
    0xa831_c66d_2db4_3210,
    0xb003_27c8_98fb_213f,
    0xbf59_7fc7_beef_0ee4,
    0xc6e0_0bf3_3da8_8fc2,
    0xd5a7_9147_930a_a725,
    0x06ca_6351_e003_826f,
    0x1429_2967_0a0e_6e70,
    0x27b7_0a85_46d2_2ffc,
    0x2e1b_2138_5c26_c926,
    0x4d2c_6dfc_5ac4_2aed,
    0x5338_0d13_9d95_b3df,
    0x650a_7354_8baf_63de,
    0x766a_0abb_3c77_b2a8,
    0x81c2_c92e_47ed_aee6,
    0x9272_2c85_1482_353b,
    0xa2bf_e8a1_4cf1_0364,
    0xa81a_664b_bc42_3001,
    0xc24b_8b70_d0f8_9791,
    0xc76c_51a3_0654_be30,
    0xd192_e819_d6ef_5218,
    0xd699_0624_5565_a910,
    0xf40e_3585_5771_202a,
    0x106a_a070_32bb_d1b8,
    0x19a4_c116_b8d2_d0c8,
    0x1e37_6c08_5141_ab53,
    0x2748_774c_df8e_eb99,
    0x34b0_bcb5_e19b_48a8,
    0x391c_0cb3_c5c9_5a63,
    0x4ed8_aa4a_e341_8acb,
    0x5b9c_ca4f_7763_e373,
    0x682e_6ff3_d6b2_b8a3,
    0x748f_82ee_5def_b2fc,
    0x78a5_636f_4317_2f60,
    0x84c8_7814_a1f0_ab72,
    0x8cc7_0208_1a64_39ec,
    0x90be_fffa_2363_1e28,
    0xa450_6ceb_de82_bde9,
    0xbef9_a3f7_b2c6_7915,
    0xc671_78f2_e372_532b,
    0xca27_3ece_ea26_619c,
    0xd186_b8c7_21c0_c207,
    0xeada_7dd6_cde0_eb1e,
    0xf57d_4f7f_ee6e_d178,
    0x06f0_67aa_7217_6fba,
    0x0a63_7dc5_a2c8_98a6,
    0x113f_9804_bef9_0dae,
    0x1b71_0b35_131c_471b,
    0x28db_77f5_2304_7d84,
    0x32ca_ab7b_40c7_2493,
    0x3c9e_be0a_15c9_bebc,
    0x431d_67c4_9c10_0d4c,
    0x4cc5_d4be_cb3e_42b6,
    0x597f_299c_fc65_7e2a,
    0x5fcb_6fab_3ad6_faec,
    0x6c44_198c_4a47_5817,
];

struct Sha512 {
    st: [u64; 8],
    blk: Blocks<128>,
}

impl Default for Sha512 {
    fn default() -> Self {
        Sha512 {
            st: [
                0x6a09_e667_f3bc_c908,
                0xbb67_ae85_84ca_a73b,
                0x3c6e_f372_fe94_f82b,
                0xa54f_f53a_5f1d_36f1,
                0x510e_527f_ade6_82d1,
                0x9b05_688c_2b3e_6c1f,
                0x1f83_d9ab_fb41_bd6b,
                0x5be0_cd19_137e_2179,
            ],
            blk: Blocks::new(),
        }
    }
}

fn sha512_block(st: &mut [u64; 8], block: &[u8; 128]) {
    let mut w = [0u64; 80];
    for (i, wi) in w[..16].iter_mut().enumerate() {
        let mut b = [0u8; 8];
        b.copy_from_slice(&block[i * 8..i * 8 + 8]);
        *wi = u64::from_be_bytes(b);
    }
    for i in 16..80 {
        let s0 = w[i - 15].rotate_right(1) ^ w[i - 15].rotate_right(8) ^ (w[i - 15] >> 7);
        let s1 = w[i - 2].rotate_right(19) ^ w[i - 2].rotate_right(61) ^ (w[i - 2] >> 6);
        w[i] = w[i - 16]
            .wrapping_add(s0)
            .wrapping_add(w[i - 7])
            .wrapping_add(s1);
    }
    let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = *st;
    for i in 0..80 {
        let s1 = e.rotate_right(14) ^ e.rotate_right(18) ^ e.rotate_right(41);
        let ch = (e & f) ^ (!e & g);
        let t1 = h
            .wrapping_add(s1)
            .wrapping_add(ch)
            .wrapping_add(K512[i])
            .wrapping_add(w[i]);
        let s0 = a.rotate_right(28) ^ a.rotate_right(34) ^ a.rotate_right(39);
        let maj = (a & b) ^ (a & c) ^ (b & c);
        let t2 = s0.wrapping_add(maj);
        h = g;
        g = f;
        f = e;
        e = d.wrapping_add(t1);
        d = c;
        c = b;
        b = a;
        a = t1.wrapping_add(t2);
    }
    for (s, v) in st.iter_mut().zip([a, b, c, d, e, f, g, h]) {
        *s = s.wrapping_add(v);
    }
}

impl Digest for Sha512 {
    fn update(&mut self, data: &[u8]) {
        let st = &mut self.st;
        self.blk.feed(data, |b| sha512_block(st, b));
    }
    fn finish(mut self: Box<Self>) -> String {
        let st = &mut self.st;
        self.blk.pad(16, true, |b| sha512_block(st, b));
        let mut out = Vec::with_capacity(64);
        for w in self.st {
            out.extend_from_slice(&w.to_be_bytes());
        }
        hex(&out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 표준 시험 벡터(빈 입력 · "abc" · 긴 문장) — 알고리즘 5종.
    #[test]
    fn known_vectors() {
        let fox = b"The quick brown fox jumps over the lazy dog";
        let cases: [(Algo, &[u8], &str); 14] = [
            (Algo::Crc32, b"", "00000000"),
            (Algo::Crc32, b"abc", "352441c2"),
            (Algo::Crc32, fox, "414fa339"),
            (Algo::Md5, b"", "d41d8cd98f00b204e9800998ecf8427e"),
            (Algo::Md5, b"abc", "900150983cd24fb0d6963f7d28e17f72"),
            (Algo::Md5, fox, "9e107d9d372bb6826bd81d3542a419d6"),
            (Algo::Sha1, b"", "da39a3ee5e6b4b0d3255bfef95601890afd80709"),
            (Algo::Sha1, b"abc", "a9993e364706816aba3e25717850c26c9cd0d89d"),
            (Algo::Sha1, fox, "2fd4e1c67a2d28fced849ee1bb76e7391b93eb12"),
            (
                Algo::Sha256,
                b"",
                "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            ),
            (
                Algo::Sha256,
                b"abc",
                "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
            ),
            (
                Algo::Sha256,
                fox,
                "d7a8fbb307d7809469ca9abcb0082e4f8d5651e46d3cdb762d02d0bf37c9e592",
            ),
            (
                Algo::Sha512,
                b"",
                "cf83e1357eefb8bdf1542850d66d8007d620e4050b5715dc83f4a921d36ce9ce47d0d13c5d85f2b0ff8318d2877eec2f63b931bd47417a81a538327af927da3e",
            ),
            (
                Algo::Sha512,
                b"abc",
                "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f",
            ),
        ];
        for (a, input, want) in cases {
            assert_eq!(
                digest(a, input),
                want,
                "{a:?} {:?}",
                String::from_utf8_lossy(input)
            );
        }
        // 2블록 경계(55 · 56 · 64 · 119 · 120 · 128바이트)와 조각 입력 = 한 번에 넣은 것과 같다.
        for n in [
            55usize, 56, 63, 64, 65, 111, 112, 119, 120, 127, 128, 129, 1000,
        ] {
            let data: Vec<u8> = (0..n).map(|i| (i * 7 % 251) as u8).collect();
            for a in Algo::ALL {
                let whole = digest(a, &data);
                let mut d = new_digest(a);
                for chunk in data.chunks(13) {
                    d.update(chunk);
                }
                assert_eq!(d.finish(), whole, "{a:?} n={n}");
            }
        }
        // 1,000,000 × 'a'(FIPS 벡터).
        let million = vec![b'a'; 1_000_000];
        assert_eq!(
            digest(Algo::Sha256, &million),
            "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0"
        );
        assert_eq!(
            digest(Algo::Sha1, &million),
            "34aa973cd4c4daa4f61eeb2bdbad27316534016f"
        );
        assert_eq!(
            digest(Algo::Md5, &million),
            "7707d6ae4e027c70eea2a935c2296f21"
        );
    }

    #[test]
    fn hex_normalize_and_guess() {
        assert_eq!(
            normalize_hex(" D41D8CD9-8F00B204:e9800998 ecf8427e "),
            Some("d41d8cd98f00b204e9800998ecf8427e".into())
        );
        assert_eq!(normalize_hex("zz"), None);
        assert_eq!(normalize_hex("   "), None);
        assert_eq!(guess_algo("414fa339"), Some(Algo::Crc32));
        assert_eq!(guess_algo(&"a".repeat(64)), Some(Algo::Sha256));
        assert_eq!(guess_algo(&"a".repeat(128)), Some(Algo::Sha512));
        assert_eq!(guess_algo("abc"), None);
        assert_eq!(Algo::from_key("sha1"), Some(Algo::Sha1));
        assert_eq!(Algo::from_key("x"), None);
    }

    /// 파일: 여러 알고리즘을 한 번 읽으며 · 진행 합계 = 크기 · 취소 = Interrupted · 없는 파일 = 오류.
    #[test]
    fn hash_file_streams_and_cancels() {
        let d = std::env::temp_dir().join(format!("nexa_hash_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        let p = d.join("f.bin");
        let data: Vec<u8> = (0..3_000_000u32).map(|i| (i % 253) as u8).collect();
        std::fs::write(&p, &data).unwrap();
        let cancel = AtomicBool::new(false);
        let mut sum = 0u64;
        let out = hash_file(&p, &[Algo::Crc32, Algo::Sha256], &mut |n| sum += n, &cancel).unwrap();
        assert_eq!(sum, data.len() as u64);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].1, digest(Algo::Crc32, &data));
        assert_eq!(out[1].1, digest(Algo::Sha256, &data));
        let hit = AtomicBool::new(false);
        let e = hash_file(
            &p,
            &[Algo::Md5],
            &mut |_| hit.store(true, Ordering::Relaxed),
            &hit,
        )
        .unwrap_err();
        assert_eq!(e.kind(), io::ErrorKind::Interrupted);
        assert!(hash_file(&d.join("nope"), &[Algo::Md5], &mut |_| {}, &cancel).is_err());
        let _ = std::fs::remove_dir_all(&d);
    }

    /// 성능 측정(수동 · `cargo test -p ndir-ops --release bench_hash -- --ignored --nocapture`): 256 MiB 메모리 입력을 알고리즘별로 ·
    /// 다섯 개를 한 번에(체크섬 창의 기본 경로 = 파일을 한 번 읽으며 함께).
    #[test]
    #[ignore = "수동 성능 측정"]
    fn bench_hash() {
        let data: Vec<u8> = (0..256u32 * 1024 * 1024).map(|i| (i % 251) as u8).collect();
        let mb = data.len() as f64 / (1024.0 * 1024.0);
        for a in Algo::ALL {
            let at = std::time::Instant::now();
            let _ = digest(a, &data);
            let s = at.elapsed().as_secs_f64();
            println!("{:<8} {:>7.0} MB/s", a.name(), mb / s);
        }
        let at = std::time::Instant::now();
        let mut ds: Vec<Box<dyn Digest>> = Algo::ALL.iter().map(|&a| new_digest(a)).collect();
        for chunk in data.chunks(1 << 20) {
            for d in &mut ds {
                d.update(chunk);
            }
        }
        for d in ds {
            let _ = d.finish();
        }
        println!(
            "{:<8} {:>7.0} MB/s",
            "5종 함께",
            mb / at.elapsed().as_secs_f64()
        );
    }
}
