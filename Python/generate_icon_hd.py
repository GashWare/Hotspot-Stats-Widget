import math
from PIL import Image, ImageDraw, ImageFilter

def create_hd_icon(size=1024):
    # Render at 1024x1024 with 4x supersampling for buttery smooth anti-aliased curves
    img = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)

    center = size / 2.0
    r_outer = size * 0.46

    # 1. Subtle Outer Glow
    glow = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    glow_draw = ImageDraw.Draw(glow)
    glow_draw.ellipse(
        [center - r_outer - 15, center - r_outer - 15, center + r_outer + 15, center + r_outer + 15],
        outline=(0, 255, 204, 120),
        width=int(size * 0.04)
    )
    glow = glow.filter(ImageFilter.GaussianBlur(radius=int(size * 0.03)))
    img = Image.alpha_composite(img, glow)
    draw = ImageDraw.Draw(img)

    # 2. Main Dial Background: Rich Dark Cockpit Gradient Plate
    for i in range(int(r_outer), 0, -1):
        factor = i / r_outer
        # Radial shading from dark slate (28, 32, 42) at center to deep black (12, 14, 18) at edge
        r_c = int(12 + 16 * factor)
        g_c = int(14 + 18 * factor)
        b_c = int(18 + 24 * factor)
        draw.ellipse([center - i, center - i, center + i, center + i], fill=(r_c, g_c, b_c, 255))

    # 3. Outer Polished Titanium Ring & Neon Rim
    draw.ellipse(
        [center - r_outer, center - r_outer, center + r_outer, center + r_outer],
        outline=(0, 255, 204, 255),
        width=int(size * 0.035)
    )

    r_bezel = r_outer * 0.94
    draw.ellipse(
        [center - r_bezel, center - r_bezel, center + r_bezel, center + r_bezel],
        outline=(30, 42, 52, 255),
        width=int(size * 0.02)
    )

    # 4. Tachometer / Speedometer Scale Ticks (270 degree sweep)
    num_ticks = 36
    r_tick_start = r_bezel * 0.86
    r_tick_end_minor = r_bezel * 0.93
    r_tick_end_major = r_bezel * 0.98

    for i in range(num_ticks):
        fraction = i / (num_ticks - 1)
        angle_deg = 135.0 + fraction * 270.0
        angle_rad = math.radians(angle_deg)
        cos_a = math.cos(angle_rad)
        sin_a = math.sin(angle_rad)

        is_major = (i % 3 == 0)
        t_end = r_tick_end_major if is_major else r_tick_end_minor
        w = int(size * 0.016) if is_major else int(size * 0.008)

        # Color progression: Neon Cyan (0-60%) -> Amber/Yellow (60-80%) -> Redline (80-100%)
        if fraction > 0.80:
            col = (255, 30, 60, 255) # Redline Red
        elif fraction > 0.60:
            col = (255, 175, 0, 255) # Warning Amber
        else:
            col = (0, 240, 200, 255) # Neon Cyan

        x1 = center + r_tick_start * cos_a
        y1 = center + r_tick_start * sin_a
        x2 = center + t_end * cos_a
        y2 = center + t_end * sin_a

        draw.line([(x1, y1), (x2, y2)], fill=col, width=max(1, w))

    # 5. Inner Gauge Track Arc
    r_track = r_bezel * 0.83
    draw.arc(
        [center - r_track, center - r_track, center + r_track, center + r_track],
        start=135,
        end=405,
        fill=(0, 255, 204, 80),
        width=int(size * 0.01)
    )

    # 6. Center Telemetry Radar Grid / Crosshairs
    cross_r = r_bezel * 0.45
    draw.line([(center - cross_r, center), (center + cross_r, center)], fill=(0, 255, 204, 60), width=int(size * 0.008))
    draw.line([(center, center - cross_r), (center, center + cross_r)], fill=(0, 255, 204, 60), width=int(size * 0.008))
    draw.ellipse([center - cross_r * 0.6, center - cross_r * 0.6, center + cross_r * 0.6, center + cross_r * 0.6], outline=(0, 255, 204, 50), width=int(size * 0.008))

    # 7. Dynamic Speedometer Needle (pointing to 75% performance)
    needle_angle = math.radians(135.0 + 0.72 * 270.0)
    cos_n = math.cos(needle_angle)
    sin_n = math.sin(needle_angle)
    tip_x = center + (r_bezel * 0.88) * cos_n
    tip_y = center + (r_bezel * 0.88) * sin_n

    # Needle base width perpendicular
    perp_cos = -sin_n
    perp_sin = cos_n
    bw = size * 0.022
    b1_x = center + bw * perp_cos
    b1_y = center + bw * perp_sin
    b2_x = center - bw * perp_cos
    b2_y = center - bw * perp_sin

    # Counter-weight tail
    tail_len = size * 0.12
    tx = center - tail_len * cos_n
    ty = center - tail_len * sin_n

    # Draw sharp polygon needle
    draw.polygon([(b1_x, b1_y), (tip_x, tip_y), (b2_x, b2_y), (tx, ty)], fill=(255, 210, 0, 255))
    # Bright needle highlight core
    draw.line([(center, center), (tip_x, tip_y)], fill=(255, 255, 255, 255), width=int(size * 0.008))

    # 8. Gleaming Center Hub Cap
    hub_r = size * 0.09
    draw.ellipse([center - hub_r, center - hub_r, center + hub_r, center + hub_r], fill=(22, 26, 34, 255), outline=(0, 255, 204, 255), width=int(size * 0.015))
    hub_inner = hub_r * 0.65
    draw.ellipse([center - hub_inner, center - hub_inner, center + hub_inner, center + hub_inner], fill=(0, 255, 170, 255))
    # Specular lens highlight on center cap
    spec_r = hub_inner * 0.45
    draw.ellipse([center - spec_r - hub_inner * 0.2, center - spec_r - hub_inner * 0.2, center + spec_r - hub_inner * 0.2, center + spec_r - hub_inner * 0.2], fill=(255, 255, 255, 200))

    return img

print("Rendering 1024x1024 Master Icon...")
master = create_hd_icon(1024)

# Create downscaled high-quality images with Lanczos filtering
resolutions = [256, 128, 64, 48, 32, 24, 16]
sub_images = [master.resize((res, res), Image.Resampling.LANCZOS) for res in resolutions]

ico_path1 = r"C:\Users\User\Desktop\Python\Stats Widget\Rust\test_gui\app_icon.ico"
ico_path2 = r"C:\Users\User\Desktop\Python\Stats Widget\Rust\app_icon.ico"

# Save properly starting from 256x256 down to 16x16 with all mipmaps included
sub_images[0].save(
    ico_path1,
    format="ICO",
    sizes=[(im.width, im.height) for im in sub_images],
    append_images=sub_images[1:]
)
sub_images[0].save(
    ico_path2,
    format="ICO",
    sizes=[(im.width, im.height) for im in sub_images],
    append_images=sub_images[1:]
)

print("Saved HD ICO with resolutions:", resolutions)
