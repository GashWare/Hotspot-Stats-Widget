import math
from PIL import Image, ImageDraw

def create_icon(size):
    img = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)
    center = size / 2.0
    r = size * 0.46

    # Outer hex/circle dark cockpit badge
    draw.ellipse([center - r, center - r, center + r, center + r], fill=(16, 18, 24, 250), outline=(0, 255, 204, 255), width=max(1, int(size * 0.05)))

    # Inner tachometer / radar ring
    r_inner = r * 0.75
    draw.ellipse([center - r_inner, center - r_inner, center + r_inner, center + r_inner], outline=(255, 136, 0, 200), width=max(1, int(size * 0.03)))

    # Speedometer tick marks & signal bars
    num_ticks = 12
    for i in range(num_ticks):
        angle = math.radians(135 + i * (270 / (num_ticks - 1)))
        t1 = r * 0.82
        t2 = r * 0.94
        x1 = center + t1 * math.cos(angle)
        y1 = center + t1 * math.sin(angle)
        x2 = center + t2 * math.cos(angle)
        y2 = center + t2 * math.sin(angle)
        col = (255, 0, 85, 255) if i >= 9 else (0, 255, 204, 255)
        draw.line([(x1, y1), (x2, y2)], fill=col, width=max(1, int(size * 0.04)))

    # Center gauge needle / pulse dot
    needle_angle = math.radians(225)
    nx = center + (r * 0.65) * math.cos(needle_angle)
    ny = center + (r * 0.65) * math.sin(needle_angle)
    draw.line([(center, center), (nx, ny)], fill=(255, 204, 0, 255), width=max(1, int(size * 0.06)))

    # Glowing center cap
    cr = size * 0.12
    draw.ellipse([center - cr, center - cr, center + cr, center + cr], fill=(0, 255, 136, 255), outline=(255, 255, 255, 255), width=max(1, int(size * 0.02)))

    return img

sizes = [16, 24, 32, 48, 64, 128, 256]
images = [create_icon(s) for s in sizes]

# Save to Rust/test_gui/app_icon.ico and Rust/app_icon.ico
images[0].save(r"C:\Users\User\Desktop\Python\Stats Widget\Rust\test_gui\app_icon.ico", format="ICO", sizes=[(s, s) for s in sizes], append_images=images[1:])
images[0].save(r"C:\Users\User\Desktop\Python\Stats Widget\Rust\app_icon.ico", format="ICO", sizes=[(s, s) for s in sizes], append_images=images[1:])
print("Icon generated successfully!")
