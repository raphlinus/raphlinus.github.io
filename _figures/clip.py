# Figures for the clipping post

def svg_header(w = 600, h = 400):
    print(f'<svg width="{w}" height="{h}" viewBox="0 0 {w} {h}" fill="none" xmlns="http://www.w3.org/2000/svg">')
    print('  <style>')
    print('    text {')
    print('      font-family: Arial, sans-serif;')
    print('    }')
    print('  </style>')

# source for clip_tiles.svg
def clip_tile_fig():
    svg_header()
    c = 40
    w = 14
    h = 9
    x0 = 10
    y0 = 10
    # cell data - this is by hand
    gray = [
        [2],
        [1, 2, 3, 4, 8, 9, 10],
        [1, 4, 5, 6, 7, 8, 10],
        [1, 10],
        [1, 2, 10, 11],
        [2, 3, 11],
        [3, 4, 5, 11],
        [5, 6, 7, 8, 9, 11],
        [9, 10, 11, 12]
    ]
    black = [
        [],
        [],
        [2, 3, 9],
        [2, 3, 4, 5, 6, 7, 8, 9],
        [3, 4, 5, 6, 7, 8, 9],
        [4, 5, 6, 7, 8, 9, 10],
        [6, 7, 8, 9, 10],
        [10]
    ]
    for (y, xs) in enumerate(gray):
        for x in xs:
            print(f'  <rect x="{x0 + x * c}" y="{y0 + y * c}" width="{c}" height="{c}" fill="#ccc" />')
    for (y, xs) in enumerate(black):
        for x in xs:
            print(f'  <rect x="{x0 + x * c}" y="{y0 + y * c}" width="{c}" height="{c}" fill="#444" />')
    # grid
    for i in range(0, w + 1):
        print(f'  <line x1="{x0 + i * c}" y1="{y0}" x2="{x0 + i * c}" y2="{y0 + h * c}" stroke="#000" />')
    for i in range(0, h + 1):
        print(f'  <line x1="{x0}" y1="{y0 + i * c}" x2="{x0 + w * c}" y2="{y0 + i * c}" stroke="#000" />')
    # path
    print('  <path d="M60 150L100 40 250 120 420 70 495 360 180 280z" stroke="#008" stroke-width="2" fill="none" />')
    print('</svg>')

clip_tile_fig()
