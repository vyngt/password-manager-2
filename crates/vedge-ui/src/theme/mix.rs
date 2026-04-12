use super::color_space::{ACHROMATIC_THRESHOLD, Oklch};

/// Interpolate between two hue angles along the shortest arc.
///
/// Handles the 360°→0° boundary correctly (e.g. 350°→10° goes through 0°,
/// not through 180°).
pub fn hue_lerp(h_a: f32, h_b: f32, t: f32) -> f32 {
    let mut delta = h_b - h_a;
    if delta > 180.0 {
        delta -= 360.0;
    }
    if delta < -180.0 {
        delta += 360.0;
    }
    let result = h_a + t * delta;
    ((result % 360.0) + 360.0) % 360.0
}

/// Mix two OKLCH colors. `t = 0.0` returns `a`, `t = 1.0` returns `b`.
///
/// L and C are linearly interpolated. H uses shortest-arc interpolation.
/// When one color is achromatic (C ≈ 0), the other's hue is used directly
/// to avoid interpolating toward an undefined hue angle.
pub fn oklch_mix(a: Oklch, b: Oklch, t: f32) -> Oklch {
    let l = a.l + t * (b.l - a.l);
    let c = a.c + t * (b.c - a.c);

    let h = if a.c < ACHROMATIC_THRESHOLD && b.c < ACHROMATIC_THRESHOLD {
        0.0 // both achromatic
    } else if a.c < ACHROMATIC_THRESHOLD {
        b.h // a is achromatic, use b's hue
    } else if b.c < ACHROMATIC_THRESHOLD {
        a.h // b is achromatic, use a's hue
    } else {
        hue_lerp(a.h, b.h, t)
    };

    Oklch {
        l: l.clamp(0.0, 1.0),
        c: c.max(0.0),
        h,
    }
}

/// Darken an OKLCH color by reducing its lightness by `amount`.
pub fn oklch_darken(c: Oklch, amount: f32) -> Oklch {
    Oklch {
        l: (c.l - amount).clamp(0.0, 1.0),
        ..c
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f32, b: f32, tol: f32) -> bool {
        (a - b).abs() <= tol
    }

    #[test]
    fn hue_lerp_simple() {
        assert!(approx_eq(hue_lerp(0.0, 90.0, 0.5), 45.0, 0.01));
    }

    #[test]
    fn hue_lerp_across_zero() {
        // 350° → 10° should go through 0°, midpoint = 0°
        assert!(approx_eq(hue_lerp(350.0, 10.0, 0.5), 0.0, 0.01));
    }

    #[test]
    fn hue_lerp_reverse_across_zero() {
        // 10° → 350° should also go through 0°, midpoint = 0°
        assert!(approx_eq(hue_lerp(10.0, 350.0, 0.5), 0.0, 0.01));
    }

    #[test]
    fn hue_lerp_same() {
        assert!(approx_eq(hue_lerp(120.0, 120.0, 0.5), 120.0, 0.01));
    }

    #[test]
    fn mix_endpoints() {
        let a = Oklch {
            l: 0.2,
            c: 0.1,
            h: 30.0,
        };
        let b = Oklch {
            l: 0.8,
            c: 0.3,
            h: 60.0,
        };

        let at_0 = oklch_mix(a, b, 0.0);
        assert!(approx_eq(at_0.l, a.l, 0.001));
        assert!(approx_eq(at_0.c, a.c, 0.001));

        let at_1 = oklch_mix(a, b, 1.0);
        assert!(approx_eq(at_1.l, b.l, 0.001));
        assert!(approx_eq(at_1.c, b.c, 0.001));
    }

    #[test]
    fn mix_achromatic_uses_chromatic_hue() {
        let gray = Oklch {
            l: 0.5,
            c: 0.0,
            h: 0.0,
        };
        let blue = Oklch {
            l: 0.5,
            c: 0.2,
            h: 250.0,
        };

        let mixed = oklch_mix(gray, blue, 0.5);
        // Should use blue's hue, not interpolate gray's undefined hue
        assert!(approx_eq(mixed.h, 250.0, 0.01));
    }

    #[test]
    fn darken_clamps() {
        let c = Oklch {
            l: 0.1,
            c: 0.2,
            h: 30.0,
        };
        let darkened = oklch_darken(c, 0.5);
        assert!(approx_eq(darkened.l, 0.0, 0.001));
        assert!(approx_eq(darkened.c, c.c, 0.001));
    }
}
