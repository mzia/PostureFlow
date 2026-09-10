use crate::profile::Profile;

/// Generates a set of ARGB32 pixmaps (network byte order / Big Endian)
/// compatible with org.kde.StatusNotifierItem across COSMIC and GNOME AppIndicator.
pub fn get_icon_pixmap(profile: Profile) -> Vec<(i32, i32, Vec<u8>)> {
    vec![
        render_pixmap(profile, 24, 24),
        render_pixmap(profile, 32, 32),
    ]
}

fn render_pixmap(profile: Profile, width: i32, height: i32) -> (i32, i32, Vec<u8>) {
    let w = width as usize;
    let h = height as usize;
    let mut data = vec![0u8; w * h * 4];

    let (r_base, g_base, b_base) = match profile {
        Profile::Home => (16, 185, 129),   // Emerald Green
        Profile::Work => (59, 130, 246),   // Sapphire Blue
        Profile::Dev => (245, 158, 11),    // Amber Gold
        Profile::Travel => (239, 68, 68),  // Crimson Red
    };



    for y in 0..h {
        let ny = y as f32 / h as f32 * 24.0;
        for x in 0..w {
            let nx = x as f32 / w as f32 * 24.0;
            let offset = (y * w + x) * 4;

            // Shield geometry on 24x24 grid:
            // Center is nx = 11.5, top rim is ny = 2.5, tip is ny = 21.5
            let dx = (nx - 11.5).abs();
            let max_dx = if ny < 2.5 || ny > 21.5 {
                0.0
            } else if ny <= 13.0 {
                8.0 - (2.5 - ny.min(4.0)).abs() * 0.4
            } else {
                let factor = (ny - 13.0) / 8.5;
                8.0 * (1.0 - factor * factor).max(0.0).sqrt()
            };

            if dx > max_dx {
                // Background transparent
                continue;
            }

            let dist_to_edge = max_dx - dx;
            let edge_alpha = (dist_to_edge * 255.0).clamp(0.0, 255.0) as u8;

            // Border or inner fill
            let is_border = dist_to_edge < 1.3 || (ny - 2.5).abs() < 1.0 || (21.5 - ny).abs() < 1.0;

            // Determine if pixel is part of inner emblem
            let emblem = is_emblem_pixel(profile, nx, ny);

            let (a, r, g, b) = if emblem {
                (255, 255, 255, 255) // Bright white emblem
            } else if is_border {
                (edge_alpha, 255, 255, 255) // Crisp white shield highlight border
            } else {
                // Subtle gradient inside shield
                let grad = 1.0 - (ny / 24.0) * 0.25;
                let r_fill = ((r_base as f32 * grad).clamp(0.0, 255.0)) as u8;
                let g_fill = ((g_base as f32 * grad).clamp(0.0, 255.0)) as u8;
                let b_fill = ((b_base as f32 * grad).clamp(0.0, 255.0)) as u8;
                (255, r_fill, g_fill, b_fill)
            };

            // Network byte order ARGB: [A, R, G, B]
            data[offset] = a;
            data[offset + 1] = r;
            data[offset + 2] = g;
            data[offset + 3] = b;
        }
    }

    (width, height, data)
}

fn is_emblem_pixel(profile: Profile, x: f32, y: f32) -> bool {
    match profile {
        Profile::Home => {
            // Home Roof: triangle from (11.5, 6.5) down to (6.5, 11.5) and (16.5, 11.5)
            let in_roof = y >= 6.5 && y <= 11.5 && (x - 11.5).abs() <= (y - 6.5) * 1.0 + 0.5;
            // Home Base: rect (8.0..15.0, 11.5..16.5)
            let in_base = x >= 8.0 && x <= 15.0 && y >= 11.5 && y <= 16.5;
            // Door cutout: rect (10.5..12.5, 13.5..16.5)
            let in_door = x >= 10.5 && x <= 12.5 && y >= 13.5 && y <= 16.5;
            (in_roof || in_base) && !in_door
        }
        Profile::Work => {
            // Briefcase Handle: arch at y=7.0..9.0, x=9.5..13.5
            let in_handle = y >= 7.0 && y <= 9.0 && x >= 9.5 && x <= 13.5
                && !(y >= 8.2 && x >= 10.7 && x <= 12.3);
            // Briefcase Body: rect at x=6.5..16.5, y=9.0..16.0
            let in_body = x >= 6.5 && x <= 16.5 && y >= 9.0 && y <= 16.0;
            // Latch cutout: rect at x=11.0..12.0, y=11.0..13.0
            let in_latch = x >= 11.0 && x <= 12.0 && y >= 11.0 && y <= 13.0;
            (in_handle || in_body) && !in_latch
        }
        Profile::Dev => {
            // Left bracket <: lines (10.0, 7.5) -> (6.5, 11.5) -> (10.0, 15.5)
            let in_left = (y >= 7.5 && y <= 11.5 && ((10.0 - (y - 7.5) * 0.88) - x).abs() <= 0.9)
                || (y >= 11.5 && y <= 15.5 && ((6.5 + (y - 11.5) * 0.88) - x).abs() <= 0.9);
            // Right bracket >: lines (13.0, 7.5) -> (16.5, 11.5) -> (13.0, 15.5)
            let in_right = (y >= 7.5 && y <= 11.5 && ((13.0 + (y - 7.5) * 0.88) - x).abs() <= 0.9)
                || (y >= 11.5 && y <= 15.5 && ((16.5 - (y - 11.5) * 0.88) - x).abs() <= 0.9);
            // Slash /: line from (13.0, 7.0) to (10.0, 16.0)
            let in_slash = y >= 7.0 && y <= 16.0 && ((13.0 - (y - 7.0) * 0.33) - x).abs() <= 0.65;
            in_left || in_right || in_slash
        }
        Profile::Travel => {
            // Padlock Shackle: arch at y=5.5..10.0, x=8.5..14.5
            let in_shackle = y >= 5.5 && y <= 10.0 && x >= 8.5 && x <= 14.5
                && !(y >= 7.2 && x >= 10.2 && x <= 12.8);
            // Padlock Body: rect at x=7.5..15.5, y=10.0..16.5
            let in_body = x >= 7.5 && x <= 15.5 && y >= 10.0 && y <= 16.5;
            // Keyhole cutout: circle at (11.5, 12.5) + slot at 13.0..15.0
            let in_keyhole = ((x - 11.5).powi(2) + (y - 12.5).powi(2)) <= 1.2
                || (x >= 11.0 && x <= 12.0 && y >= 12.5 && y <= 15.0);
            (in_shackle || in_body) && !in_keyhole
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile::Profile;

    #[test]
    fn test_pixmap_generation_all_profiles() {
        for profile in Profile::ALL {
            let pixmaps = get_icon_pixmap(profile);
            assert_eq!(pixmaps.len(), 2);

            let (w1, h1, ref data1) = pixmaps[0];
            assert_eq!(w1, 24);
            assert_eq!(h1, 24);
            assert_eq!(data1.len(), 24 * 24 * 4);

            let (w2, h2, ref data2) = pixmaps[1];
            assert_eq!(w2, 32);
            assert_eq!(h2, 32);
            assert_eq!(data2.len(), 32 * 32 * 4);

            // Ensure there are non-zero opaque pixels
            let has_opaque_1 = data1.chunks_exact(4).any(|p| p[0] > 0);
            assert!(has_opaque_1, "24x24 pixmap for {:?} must contain visible pixels", profile);

            let has_opaque_2 = data2.chunks_exact(4).any(|p| p[0] > 0);
            assert!(has_opaque_2, "32x32 pixmap for {:?} must contain visible pixels", profile);
        }
    }
}

