use super::color_space::{Oklch, hex_to_oklch};
use super::mix::oklch_mix;

/// Near-black anchor for shade generation. Not pure black — `#0C0C0C`
/// preserves some chroma so shades don't collapse to fully desaturated mud.
const SHADE_ANCHOR: &str = "#0C0C0C";

/// Tint ratios: how much of the base color to mix into the background.
/// Indices 0-5 map to steps 50, 100, 200, 300, 400, 500.
const TINT_RATIOS: [f32; 6] = [0.08, 0.15, 0.25, 0.38, 0.54, 0.73];

/// Shade ratios: how much of the shade anchor to mix into the base.
/// Indices 0-2 map to steps 700, 800, 900.
const SHADE_RATIOS: [f32; 3] = [0.18, 0.36, 0.55];

/// Generate a 10-step color scale from a base color and background.
///
/// Returns `[Oklch; 10]` where:
/// - `[0..6]` (steps 50–500): tints — `mix(background, base, ratio)`
/// - `[6]` (step 600): the base color itself
/// - `[7..10]` (steps 700–900): shades — `mix(base, #0C0C0C, ratio)`
///
/// This function is used identically for primary, danger, warning, and success.
pub fn generate_color_scale(base: Oklch, background: Oklch) -> [Oklch; 10] {
    let shade_anchor = hex_to_oklch(SHADE_ANCHOR).expect("hardcoded anchor is valid");

    let mut scale = [base; 10];

    // Tints: mix background toward base
    for (i, &t) in TINT_RATIOS.iter().enumerate() {
        scale[i] = oklch_mix(background, base, t);
    }

    // Base stays at index 6 (already set by array init)

    // Shades: mix base toward near-black
    for (i, &s) in SHADE_RATIOS.iter().enumerate() {
        scale[7 + i] = oklch_mix(base, shade_anchor, s);
    }

    scale
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::color_space::hex_to_oklch;

    fn approx_eq(a: f32, b: f32, tol: f32) -> bool {
        (a - b).abs() <= tol
    }

    #[test]
    fn base_is_at_index_6() {
        let base = hex_to_oklch("#2563EB").unwrap();
        let bg = hex_to_oklch("#FFFFFF").unwrap();
        let scale = generate_color_scale(base, bg);
        assert!(approx_eq(scale[6].l, base.l, 0.001));
        assert!(approx_eq(scale[6].c, base.c, 0.001));
    }

    #[test]
    fn tints_lighter_than_base() {
        let base = hex_to_oklch("#2563EB").unwrap();
        let bg = hex_to_oklch("#FFFFFF").unwrap();
        let scale = generate_color_scale(base, bg);
        for i in 0..6 {
            assert!(
                scale[i].l >= base.l,
                "tint {i} (L={}) should be >= base (L={})",
                scale[i].l,
                base.l
            );
        }
    }

    #[test]
    fn shades_darker_than_base() {
        let base = hex_to_oklch("#2563EB").unwrap();
        let bg = hex_to_oklch("#FFFFFF").unwrap();
        let scale = generate_color_scale(base, bg);
        for i in 7..10 {
            assert!(
                scale[i].l <= base.l,
                "shade {i} (L={}) should be <= base (L={})",
                scale[i].l,
                base.l
            );
        }
    }

    #[test]
    fn scale_monotonic_lightness() {
        let base = hex_to_oklch("#2563EB").unwrap();
        let bg = hex_to_oklch("#FFFFFF").unwrap();
        let scale = generate_color_scale(base, bg);
        // Lightness should generally decrease from index 0 to 9
        for i in 0..9 {
            assert!(
                scale[i].l >= scale[i + 1].l - 0.01,
                "lightness not monotonic at index {i}: {} vs {}",
                scale[i].l,
                scale[i + 1].l
            );
        }
    }
}
