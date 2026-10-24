#!/usr/bin/env python3
"""cap2png — nexa-dir `capture:<파일>` 프레임 덤프(NDIRCAP w h + u32 XRGB LE)를 PNG로(표준 라이브러리만 · zlib/struct · DR-8 무관).

사용: python scripts/cap2png.py <덤프.bin> <출력.png> [x y w h [배율]]
  x y w h = 잘라낼 영역(장치 px · 기본 전체) · 배율 = 정수 확대(픽셀 검수용 · 기본 1)
검수 절차(docs/18 §5 · journal 10-24 §19): NDIR_STARTUP_CMD에 `ui.down/move` + `capture:<파일>`로 드래그 중 화면을 찍고 → 이 스크립트로 PNG → 보기.
"""
import struct, sys, zlib

src, dst = sys.argv[1], sys.argv[2]
crop = None
if len(sys.argv) >= 7:
    crop = tuple(int(v) for v in sys.argv[3:7])  # x y w h
raw = open(src, 'rb').read()
nl = raw.index(b'\n')
head = raw[:nl].decode()
_, w, h = head.split()
w, h = int(w), int(h)
px = raw[nl + 1:]
scale = 1
if len(sys.argv) >= 8:
    scale = int(sys.argv[7])

def pixel(x, y):
    o = (y * w + x) * 4
    b, g, r, _ = px[o], px[o + 1], px[o + 2], px[o + 3]
    return r, g, b

x0, y0, cw, ch = crop if crop else (0, 0, w, h)
rows = []
for y in range(y0, y0 + ch):
    row = bytearray([0])
    for x in range(x0, x0 + cw):
        r, g, b = pixel(x, y)
        row += bytes([r, g, b]) * scale
    for _ in range(scale):
        rows.append(bytes(row))
data = b''.join(rows)

def chunk(t, d):
    c = struct.pack('>I', len(d)) + t + d
    return c + struct.pack('>I', zlib.crc32(t + d) & 0xffffffff)

out = b'\x89PNG\r\n\x1a\n'
out += chunk(b'IHDR', struct.pack('>IIBBBBB', cw * scale, ch * scale, 8, 2, 0, 0, 0))
out += chunk(b'IDAT', zlib.compress(data, 6))
out += chunk(b'IEND', b'')
open(dst, 'wb').write(out)
print('png', cw * scale, ch * scale)
