"""Generate Lion logo ICO file using only stdlib (zlib + struct)."""

import struct
import zlib
from pathlib import Path

def make_png(w, h, pixels):
    """Create a PNG from a list of (R,G,B,A) pixel rows."""
    raw = b''
    for row in pixels:
        raw += b'\x00'  # filter byte (none)
        for r, g, b, a in row:
            raw += struct.pack('BBBB', r, g, b, a)

    def chunk(ctype, data):
        c = ctype + data
        crc = struct.pack('>I', zlib.crc32(c) & 0xffffffff)
        return struct.pack('>I', len(data)) + c + crc

    ihdr = struct.pack('>IIBBBBB', w, h, 8, 6, 0, 0, 0)  # 8-bit RGBA
    compressed = zlib.compress(raw)

    return (b'\x89PNG\r\n\x1a\n' +
            chunk(b'IHDR', ihdr) +
            chunk(b'IDAT', compressed) +
            chunk(b'IEND', b''))

def make_ico(sizes):
    """Create an ICO with multiple PNG-encoded sizes."""
    pngs = []
    for w, h, pixels in sizes:
        pngs.append(make_png(w, h, pixels))

    count = len(pngs)
    header = struct.pack('<HHH', 0, 1, count)
    entries = b''
    offset = 6 + count * 16

    for i, png in enumerate(pngs):
        w = sizes[i][0]
        h = sizes[i][1]
        iw = 0 if w >= 256 else w
        ih = 0 if h >= 256 else h
        entries += struct.pack('<BBBBHHII', iw, ih, 0, 0, 1, 32, len(png), offset)
        offset += len(png)

    return header + entries + b''.join(pngs)

def fill(row, col, color, grid):
    """Color a pixel in the grid."""
    if 0 <= row < len(grid) and 0 <= col < len(grid[0]):
        grid[row][col] = color

def circle(grid, cx, cy, r, color):
    """Bresenham circle fill."""
    for y in range(len(grid)):
        for x in range(len(grid[0])):
            dx = x - cx
            dy = y - cy
            if dx*dx + dy*dy <= r*r:
                grid[y][x] = color

def ellipse(grid, cx, cy, rx, ry, color):
    """Simple ellipse fill."""
    for y in range(len(grid)):
        for x in range(len(grid[0])):
            dx = (x - cx) / rx
            dy = (y - cy) / ry
            if dx*dx + dy*dy <= 1:
                grid[y][x] = color

def line(grid, x1, y1, x2, y2, color):
    """Bresenham line."""
    dx = abs(x2 - x1)
    dy = abs(y2 - y1)
    sx = 1 if x1 < x2 else -1
    sy = 1 if y1 < y2 else -1
    err = dx - dy
    while True:
        if 0 <= y1 < len(grid) and 0 <= x1 < len(grid[0]):
            grid[y1][x1] = color
        if x1 == x2 and y1 == y2:
            break
        e2 = err * 2
        if e2 > -dy:
            err -= dy
            x1 += sx
        if e2 < dx:
            err += dx
            y1 += sy

def create_lion_pixels(size):
    """Create pixel grid for lion logo at given size."""
    g = [[(0,0,0,0) for _ in range(size)] for _ in range(size)]

    # Colors
    bg = (30, 27, 75, 255)
    amber = (245, 158, 11, 255)
    orange = (249, 115, 22, 255)
    dark = (30, 27, 75, 255)
    white = (255, 255, 255, 255)
    light = (253, 230, 138, 255)
    brown = (146, 64, 14, 255)
    dk_amber = (217, 119, 6, 255)

    cx = size // 2
    cy = size // 2
    s = size / 256  # scale factor

    # Background
    for y in range(size):
        for x in range(size):
            g[y][x] = bg

    # Mane (orange ring)
    mane_r = 60 * s
    for angle in range(0, 360, 30):
        import math
        rad = math.radians(angle)
        mx = cx + int(mane_r * math.cos(rad))
        my = cy - 7 + int(mane_r * math.sin(rad))
        lr = 3 * s
        circle(g, mx, my, max(1, int(lr)), orange)

    # Face
    face_r = 42 * s
    ellipse(g, cx, cy - 5, max(4, int(48 * s / 2)), max(4, int(48 * s / 2)), amber)

    # Ears
    ear_r = 10 * s
    circle(g, cx - 30, cy - 32, max(2, int(ear_r)), amber)
    circle(g, cx + 30, cy - 32, max(2, int(ear_r)), amber)
    circle(g, cx - 30, cy - 32, max(1, int(ear_r * 0.6)), light)
    circle(g, cx + 30, cy - 32, max(1, int(ear_r * 0.6)), light)

    # Eyes
    eye_r = 7 * s
    circle(g, cx - 14, cy - 10, max(2, int(eye_r)), white)
    circle(g, cx + 14, cy - 10, max(2, int(eye_r)), white)
    pupil_r = 4 * s
    circle(g, cx - 14, cy - 10, max(1, int(pupil_r)), dark)
    circle(g, cx + 14, cy - 10, max(1, int(pupil_r)), dark)
    # Shine
    circle(g, cx - 12, cy - 12, max(1, int(1.5 * s)), white)
    circle(g, cx + 16, cy - 12, max(1, int(1.5 * s)), white)

    # Nose
    triangle_size = max(2, int(5 * s))
    for dy in range(triangle_size):
        for dx in range(-dy, dy + 1):
            nx = cx + dx
            ny = cy + 5 + dy
            if 0 <= nx < size and 0 <= ny < size:
                g[ny][nx] = brown

    # Mouth
    mouth_y = cy + 12
    for x in range(cx - 10, cx + 11):
        if 0 <= x < size and 0 <= mouth_y < size:
            g[mouth_y][x] = brown

    # Text "LION"
    # Simple approach: draw at bottom
    # Just skip text for icon - it's too small at icon sizes

    return g

# Generate ICO with multiple sizes
sizes = []
for s in [16, 32, 48, 64, 128]:
    pixels = create_lion_pixels(s)
    sizes.append((s, s, pixels))

ico_data = make_ico(sizes)

out = Path(__file__).parent / 'lion-logo.ico'
out.write_bytes(ico_data)
print(f"Created {out} ({len(ico_data)} bytes, sizes: {[s[0] for s in sizes]})")

# Also create a PNG version (largest size)
png_data = make_png(128, 128, create_lion_pixels(128))
png_out = Path(__file__).parent / 'lion-logo.png'
png_out.write_bytes(png_data)
print(f"Created {png_out} ({len(png_data)} bytes)")
