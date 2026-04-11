# Color System — Deep Specification

> This document is the second layer, going deeper than **Color Token Framework**. That framework defines *what* exists. This spec explains *why* it exists there and *how* each value is calculated.
> 

---

## Part 1 — Color Science: Why It's Not Just Hex

### 1.1 The Problem with RGB and HSL

Before writing a single token, you need to understand why most naive color systems fail. Consider these two colors:

```
#0000FF   (blue,   HSL: 240° 100% 50%)
#FFFF00   (yellow, HSL: 60°  100% 50%)
```

Both have `lightness: 50%` in HSL. But on screen, yellow **looks significantly brighter** than blue. This is because HSL is not perceptually uniform — its lightness value does not reflect the brightness the human eye actually perceives.

The practical consequence: if you use HSL to "lighten" a primary color for a hover state, the result will be inconsistent across different hues. One color increases too much, another too little.

### 1.2 Relative Luminance — The Foundation of Everything

WCAG defines **relative luminance** with this formula:

```
L = 0.2126·R_lin + 0.7152·G_lin + 0.0722·B_lin
```

Where `R_lin`, `G_lin`, `B_lin` are the channels after linearization:

```
channel_linear(c_8bit):
  c = c_8bit / 255
  if c ≤ 0.04045:
    return c / 12.92
  else:
    return ((c + 0.055) / 1.055) ^ 2.4
```

This is the **gamma decode** step — converting from sRGB (gamma-corrected, how monitors store color) to linear light (how physical light behaves).

The three coefficients `0.2126 / 0.7152 / 0.0722` are not arbitrary — they are the relative sensitivity of cone cells in the human eye to red/green/blue. **The human eye is most sensitive to green**, then red, then blue. This is why white text on yellow (`#FFFF00`) fails contrast — yellow has luminance ~0.93 because green accounts for 71% of the weighting.

### 1.3 Contrast Ratio — Two Standards, Two Purposes

**WCAG 2.1 Contrast Ratio:**

```
CR = (L_lighter + 0.05) / (L_darker + 0.05)
```

Scale: 1:1 (no contrast) → 21:1 (black on white).

| Threshold | Name | Applies to |
| --- | --- | --- |
| 3.0:1 | Minimum UI | Focus rings, borders, large UI elements |
| 4.5:1 | AA Normal | Text ≤ 18px regular or ≤ 14px bold |
| 7.0:1 | AAA Normal | Text — this system's target for body text |
| 3.0:1 | AA Large | Text ≥ 18px regular or ≥ 14px bold |

**APCA (Advanced Perceptual Contrast Algorithm)** — the newer standard in the WCAG 3.0 draft:

APCA recognizes a weakness in WCAG 2.1: it is symmetric (black on white = white on black mathematically), but the human eye is not symmetric — dark text on a light background is more readable than light text on a dark background at the same contrast ratio.

**This system uses WCAG 2.1** as the primary validation standard because: WCAG 2.1 is the current legal requirement; APCA is still a draft; targets are set higher than minimum (7.0:1 for body) to be compatible with both.

---

## Part 2 — Color Space Architecture

### 2.1 Why OKLCH Is the Best Computation Color Space

Although the final injected tokens are hex/sRGB, **all internal calculations run in OKLCH**.

OKLCH is the polar form of OKLAB, published by Björn Ottosson in 2020. Three axes:

```
L  — Lightness    0.0 → 1.0   (perceptually uniform)
C  — Chroma       0.0 → ~0.4  (saturation)
H  — Hue          0° → 360°
```

| Problem | HSL | OKLCH |
| --- | --- | --- |
| Increase lightness by 10% | Inconsistent results across hues | Even increase, eye perceives as consistent |
| Preserve perceived brightness when changing hue | Not possible | Keep L unchanged |
| Create accessible color scales | Requires trial and error | Interpolate on the L-axis |
| Hue shift when lightening | Yes (blue becomes purple) | No |

### 2.2 Pipeline: User Input → OKLCH → sRGB → Hex

```
Step 1: Parse user input (hex, rgb, hsl, oklch)
        → normalize to sRGB [0,1] per channel

Step 2: Linearize sRGB
        → sRGB_linear (gamma decode)

Step 3: sRGB_linear → OKLAB
        [ l ]   [ 0.4122  0.5363  0.0514 ] [ r_lin ]
        [ m ] = [ 0.2119  0.6807  0.1074 ] [ g_lin ]
        [ s ]   [ 0.0883  0.2817  0.6300 ] [ b_lin ]

        l', m', s' = cbrt(l), cbrt(m), cbrt(s)

        [ L ]   [ 0.2104  0.7936 -0.0040 ] [ l' ]
        [ A ] = [ 1.9780 -2.4285  0.4506 ] [ m' ]
        [ B ]   [ 0.0260  0.7827 -0.8087 ] [ s' ]

Step 4: OKLAB → OKLCH (polar conversion)
        C = sqrt(A² + B²)
        H = atan2(B, A) × 180/π  (0–360°)

Step 5: Perform calculations in OKLCH space
        (lighten, darken, mix, hue shift...)

Step 6: OKLCH → OKLAB → sRGB_linear → sRGB

Step 7: Clamp to [0,1], convert to hex → inject into CSS
```

---

## Part 3 — Derivation Algorithms

Every surface token, text tier, and scale step has an explicit formula.

### 3.1 Surface Layer Derivation

**Input:** `--root-background` and `--root-foreground` (user-configurable).

**Principle:** Mix background toward foreground with increasing ratios.

```
surface_n = oklch_mix(background, foreground, ratio_n)

okclh_mix(a, b, t):
  L_result = L_a + t × (L_b - L_a)
  C_result = C_a + t × (C_b - C_a)
  H_result = hue_lerp(H_a, H_b, t)   ← see 3.1.1
  return oklch_to_hex(L_result, C_result, H_result)
```

| Token | Ratio | Why this ratio |
| --- | --- | --- |
| `--color-surface-1` | 4% | Sidebar/card: distinguishable but subtle |
| `--color-surface-2` | 8% | Hover: 2× surface-1, clearly interactive |
| `--color-surface-3` | 14% | Selected: larger jump, unmistakably active |
| `--color-surface-4` | 22% | Floating: big step, clearly above others |
| `--color-border` | 12% | Between surface-2 and surface-3 |
| `--color-border-strong` | 24% | Slightly past surface-4 |

**Why not linear (4%, 8%, 12%, 16%...)?** Linear ratios create mathematically even steps but not perceptually even ones. Near the background in light themes, steps become difficult to distinguish. The ratios 4/8/14/22 are calibrated so that the visual jump feels even regardless of background.

#### 3.1.1 Hue Interpolation (Circular Lerp)

Hue is a circle (0°–360°), so standard lerp has a bug: interpolating from 350° to 10° would go through 180° instead of crossing 0°.

```rust
fn hue_lerp(h_a: f32, h_b: f32, t: f32) -> f32 {
    let mut delta = h_b - h_a;
    if delta > 180.0  { delta -= 360.0; }
    if delta < -180.0 { delta += 360.0; }
    let result = h_a + t * delta;
    ((result % 360.0) + 360.0) % 360.0
}
```

### 3.2 Primary Scale Generation

**Input:** `--root-primary` (hex from user). **Output:** 10 steps from primary.50 to primary.900.

```
Tints (50 → 500):  mix(background, primary, t)
Base  (600):       primary  ← root input
Shades (700 → 900): mix(primary, #0C0C0C, s)

t values: [0.08, 0.15, 0.25, 0.38, 0.54, 0.73]  → 50, 100, 200, 300, 400, 500
s values: [0.18, 0.36, 0.55]                      → 700, 800, 900
```

**Why shade toward near-black `#0C0C0C` instead of pure black `#000000`?**

Pure black in OKLCH is L=0, C=0, H=undefined. Mixing any color toward pure black collapses C to 0 (fully desaturated). The result: shades look muddy and lose hue identity. `#0C0C0C` allows C to decrease gradually rather than collapsing completely.

### 3.3 Text Color Tier Derivation

```
text.primary   = foreground
text.secondary = oklch_mix(foreground, background, 0.30)  ← 30% wash
text.tertiary  = oklch_mix(foreground, background, 0.55)  ← 55% wash
```

**Required validation after derivation:**

```
contrast(text.primary,   background) ≥ 7.0  → AAA (hard requirement)
contrast(text.secondary, background) ≥ 4.5  → AA  (hard requirement)
contrast(text.tertiary,  background) ≥ 3.0  → UI minimum (warn if below)
```

**Why 30% and 55%, not 33% and 67%?** Perception is not linear with L. Tests across many backgrounds show 30/55 produces better perceived separation in the mid range. On dark backgrounds in particular, 55% wash creates text.tertiary that's readable for captions without being too faded.

### 3.4 Primary Foreground Selection (Auto White/Black)

When text sits on a `primary.600` background, the system auto-selects white or black. This is never a manual decision.

```rust
fn compute_primary_foreground(primary: [f32; 3]) -> &'static str {
    let lum = relative_luminance(primary);
    let white_contrast = (1.0 + 0.05) / (lum + 0.05);
    let black_contrast = (lum + 0.05) / (0.0 + 0.05);

    if white_contrast >= black_contrast {
        "#FFFFFF"
    } else {
        "#111827"  // near-black, not pure black (softer)
    }
}
```

**Practical threshold:** White wins when `L_oklch(primary) ≤ 0.62`. Most saturated primary colors fall below this threshold.

`#111827` is used instead of `#000000` for the dark foreground because: pure black on a light color creates harsh contrast; `#111827` (zinc-900, L≈0.12) still achieves AAA with backgrounds ≥ 50% lightness; and it is tonally compatible with the text system.

---

## Part 4 — Shadow Token System

### 4.1 Why Shadows Must Change with the Theme

On a white background, `rgba(0,0,0,0.12)` creates a clear drop shadow. On `#09090B` (zinc-950), the same shadow is nearly invisible. The solution: shadows are a function of background luminance.

```
if background_luminance > 0.18:    ← light background
    shadow tokens → drop shadows
else:                               ← dark background
    shadow tokens → glow borders (1px light ring)
```

### 4.2 Shadow Token Computation

**Light themes:**

```
--shadow-sm:  0 1px 3px  rgba(0, 0, 0, alpha_sm)
--shadow-md:  0 4px 12px rgba(0, 0, 0, alpha_md)
--shadow-lg:  0 8px 32px rgba(0, 0, 0, alpha_lg)

alpha_sm = 0.04 + (1 - background_luminance) × 0.06
alpha_md = 0.08 + (1 - background_luminance) × 0.08
alpha_lg = 0.12 + (1 - background_luminance) × 0.08
```

**Dark themes:**

```
--shadow-sm:  none
--shadow-md:  0 0 0 1px rgba(255, 255, 255, border_alpha_md)
--shadow-lg:  0 0 0 1px rgba(255, 255, 255, border_alpha_lg)

border_alpha_md = 0.08 + background_luminance × 0.04
border_alpha_lg = 0.12 + background_luminance × 0.04
```

**Scrim:**

```
light background:  rgba(0, 0, 0, 0.50)
dark background:   rgba(0, 0, 0, 0.70)  ← darker overlay needed
```

---

## Part 5 — Semantic Color Groups

### 5.1 Universal Token Structure

Every semantic color group follows the exact same 5-token structure:

```
{group}              Base interactive color
{group}-hover        Hover/active state
{group}-muted        Soft background for badges, chips, highlights
{group}-foreground   Text/icon on {group} background
{group}-text         {group}-colored text on neutral surfaces
```

### 5.2 Derivation per Group

**Danger (`#DC2626` — fixed, not user-configurable):**

```
danger           = #DC2626   (red-600)
danger-hover     = oklch_darken(danger, 0.06)          → ~#B91C1C
danger-muted     = oklch_mix(background, danger, 0.10) → very light red tint
danger-foreground = white_or_black(danger)              → #FFFFFF
danger-text      = oklch_darken(danger, 0.04)
```

**Primary (`--root-primary` — user-configurable):**

```
primary           = root_primary
primary-hover     = scale[700]   ← from primary scale (section 3.2)
primary-muted     = scale[50]
primary-foreground = white_or_black(primary)
primary-text      = scale[800]   ← dark enough to be readable on white
```

### 5.3 Focus Ring Token

```
--color-focus-ring = primary.600  (same as --color-primary)
```

But focus ring needs its own contrast validation — it appears on multiple surfaces:

```rust
let min_focus_contrast = [
    contrast(focus_ring, background),
    contrast(focus_ring, surface_1),
].iter().cloned().fold(f32::INFINITY, f32::min);

if min_focus_contrast < 3.0 {
    focus_ring = adjust_for_minimum_contrast(focus_ring, 3.0, background);
}
```

---

## Part 6 — Validation Pipeline

### 6.1 Validation Order

Every theme change runs this pipeline before injecting. Order matters — early steps can change the input for later ones.

```
Step 1: Parse & Normalize
  Parse root-background, root-foreground, root-primary → OKLCH
  Check parseable → Error if invalid

Step 2: Compute All Derived Tokens
  Surface layers, primary scale, text tiers,
  primary foreground, shadows, semantic groups

Step 3: Contrast Checks

  CHECK                         THRESHOLD   SEVERITY
  ─────────────────────────────────────────────────────
  text.primary / background     ≥ 7.0       BLOCK
  text.secondary / background   ≥ 4.5       BLOCK
  text.tertiary / background    ≥ 3.0       WARN
  primary-foreground / primary  ≥ 4.5       BLOCK
  focus-ring / background       ≥ 3.0       BLOCK
  focus-ring / surface-1        ≥ 3.0       WARN
  danger-foreground / danger    ≥ 4.5       BLOCK
  text.primary / surface-4      ≥ 4.5       WARN

Step 4: Surface Differentiation Check
  contrast(background, surface-1) ≥ 1.05    WARN
  contrast(surface-1, surface-2)  ≥ 1.05    WARN

Step 5: Build CSS String
  Render all tokens as :root { ... }
  Include computed value comments for debugging

Step 6: Inject
  Tauri: window.eval() inject style tag
```

### 6.2 Error Messages — User Language, Not Developer Code

```
BLOCK errors (cannot save):

  "Body text on this background is too faint (contrast 4.2:1).
   Try a darker background or lighter foreground."

  "White text on your primary button is hard to read (contrast 2.8:1).
   Try a darker primary — for example #1D4ED8 instead of #60A5FA."

WARN (save with warning):

  "Small captions may be slightly difficult to read with this color.
   Current contrast: 2.9:1. Recommendation: ≥ 3.0:1."
```

### 6.3 Auto-fix Suggestions

When a check fails, the system proposes an automatic adjustment:

```rust
fn suggest_fix(check: ContrastCheck) -> Option<ColorAdjustment> {
    if check.current_ratio >= check.minimum_ratio { return None; }

    // Binary search: find minimum L adjustment to reach target contrast
    let mut low = 0.0f32;
    let mut high = 1.0f32;

    for _ in 0..16 {  // 16 iterations → 1/65536 precision
        let mid = (low + high) / 2.0;
        let adjusted = adjust_lightness(check.color, mid);
        if contrast(adjusted, check.surface) >= check.minimum_ratio {
            high = mid;
        } else {
            low = mid;
        }
    }

    Some(ColorAdjustment {
        token: check.token_name,
        suggested: adjust_lightness(check.color, high),
        new_contrast: contrast(adjust_lightness(check.color, high), check.surface),
    })
}
```

---

## Part 7 — Token Dependency Graph

A map of: changing `X` will affect which `Y`.

```
┌──────────────────────────────────────────────────────────────┐
│                      USER INPUTS                              │
│  ┌───────────────┐  ┌────────────────┐  ┌─────────────────┐  │
│  │root-background│  │root-foreground │  │  root-primary   │  │
│  └───────┬───────┘  └───────┬────────┘  └────────┬────────┘  │
└──────────┼─────────────────┼───────────────────── ┼──────────┘
           │                 │                       │
           ▼                 ▼                       ▼
   ┌──────────────┐  ┌────────────────┐  ┌────────────────────┐
   │Surface Layers│  │  Text Tiers    │  │  Primary Scale     │
   │  surface-1   │  │ text.primary   │  │  primary (600)     │
   │  surface-2   │  │ text.secondary │  │  primary-hover(700)│
   │  surface-3   │  │ text.tertiary  │  │  primary-muted(50) │
   │  surface-4   │  └───────┬────────┘  │  primary.50→900    │
   │  border      │          │            └─────────┬──────────┘
   │  border-strong│         │                      │
   └──────┬───────┘          │                      ▼
          │                  │           ┌───────────────────────┐
          │                  │           │  primary-foreground   │
          │                  │           │  (auto white/black)   │
          │                  │           └───────────────────────┘
          ▼                  ▼
   ┌──────────────────────────────────────────────────────────┐
   │                    SHADOW TOKENS                          │
   │  Computed from background_luminance                       │
   │  shadow-sm / shadow-md / shadow-lg / shadow-scrim         │
   └──────────────────────────────────────────────────────────┘
```

**Impact summary — what changing each input affects:**

| Change | Tokens affected | Components affected |
| --- | --- | --- |
| `--root-background` | All surface tokens, text tiers, shadows, all `*-muted` | Every component on the page |
| `--root-foreground` | Text tiers, surface layers (mixing), borders | Text, dividers, borders |
| `--root-primary` | Primary scale, primary-foreground, focus-ring | Buttons, links, focus indicators, badges |

---

## Part 8 — Rust Implementation Reference

### 8.1 ThemeConfig Struct

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeConfig {
    pub id:              Uuid,
    pub name:            String,
    pub created_at:      DateTime<Utc>,

    // User-configurable roots
    pub root_background: Color,
    pub root_foreground: Color,
    pub root_primary:    Color,

    // Fixed semantic groups (locked in UI, stored for completeness)
    pub danger_base:     Color,   // always red-600
    pub warning_base:    Color,   // always amber-600
    pub success_base:    Color,   // always green-600
}
```

### 8.2 Core Computation Function

```rust
impl ThemeConfig {
    pub fn compute(&self) -> Result<ComputedTheme, ThemeError> {
        let bg   = parse_oklch(&self.root_background)?;
        let fg   = parse_oklch(&self.root_foreground)?;
        let prim = parse_oklch(&self.root_primary)?;

        let mut vars: HashMap<String, String> = HashMap::new();

        // Surfaces
        vars.insert("--color-background".into(),     oklch_to_hex(bg));
        vars.insert("--color-surface-1".into(),      oklch_to_hex(oklch_mix(bg, fg, 0.04)));
        vars.insert("--color-surface-2".into(),      oklch_to_hex(oklch_mix(bg, fg, 0.08)));
        vars.insert("--color-surface-3".into(),      oklch_to_hex(oklch_mix(bg, fg, 0.14)));
        vars.insert("--color-surface-4".into(),      oklch_to_hex(oklch_mix(bg, fg, 0.22)));
        vars.insert("--color-border".into(),         oklch_to_hex(oklch_mix(bg, fg, 0.12)));
        vars.insert("--color-border-strong".into(),  oklch_to_hex(oklch_mix(bg, fg, 0.24)));

        // Text tiers
        vars.insert("--color-text-primary".into(),   oklch_to_hex(fg));
        vars.insert("--color-text-secondary".into(), oklch_to_hex(oklch_mix(fg, bg, 0.30)));
        vars.insert("--color-text-tertiary".into(),  oklch_to_hex(oklch_mix(fg, bg, 0.55)));

        // Primary scale
        let scale = generate_primary_scale(prim, bg);
        vars.insert("--color-primary".into(),            oklch_to_hex(scale[6]));
        vars.insert("--color-primary-hover".into(),      oklch_to_hex(scale[7]));
        vars.insert("--color-primary-muted".into(),      oklch_to_hex(scale[0]));
        vars.insert("--color-primary-foreground".into(),
                    compute_foreground(scale[6]).to_string());

        // Shadows
        let bg_lum = relative_luminance(oklch_to_srgb(bg));
        vars.extend(compute_shadows(bg_lum));

        // Fixed semantic groups
        vars.extend(compute_semantic_group(parse_oklch("#DC2626")?, bg, "danger"));
        // ... warning, success

        // Focus ring
        vars.insert("--color-focus-ring".into(),
                    vars["--color-primary"].clone());

        // Validate
        let validation = validate_computed_theme(&vars)?;

        Ok(ComputedTheme { css_vars: vars, validation_results: validation })
    }
}
```

### 8.3 OKLCH Math

```rust
pub fn oklch_mix(a: Oklch, b: Oklch, t: f32) -> Oklch {
    Oklch {
        l: a.l + t * (b.l - a.l),
        c: a.c + t * (b.c - a.c),
        h: hue_lerp(a.h, b.h, t),
    }
}

fn hue_lerp(h_a: f32, h_b: f32, t: f32) -> f32 {
    let mut delta = h_b - h_a;
    if delta > 180.0  { delta -= 360.0; }
    if delta < -180.0 { delta += 360.0; }
    let result = h_a + t * delta;
    ((result % 360.0) + 360.0) % 360.0
}

pub fn relative_luminance(rgb: [f32; 3]) -> f32 {
    let lin = |c: f32| {
        if c <= 0.04045 { c / 12.92 }
        else { ((c + 0.055) / 1.055).powf(2.4) }
    };
    let [r, g, b] = rgb;
    0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b)
}

pub fn contrast_ratio(l1: f32, l2: f32) -> f32 {
    let lighter = l1.max(l2);
    let darker  = l1.min(l2);
    (lighter + 0.05) / (darker + 0.05)
}
```

---

## Part 9 — CSS Output Format

### 9.1 Injected Style Block (example output)

```css
/* Injected by Rust at startup — do not edit manually */
/* Theme: "Default Light" | Generated: 2025-04-11T08:23:01Z */
:root {
  /* Backgrounds */
  --color-background: #ffffff;           /* bg lum: 1.000 */
  --color-surface-1:  #f9fafb;           /* mix 4%  */
  --color-surface-2:  #f3f4f6;           /* mix 8%  */
  --color-surface-3:  #e5e7eb;           /* mix 14% */
  --color-surface-4:  #d1d5db;           /* mix 22% */
  --color-border:        #e5e7eb;        /* mix 12% */
  --color-border-strong: #d1d5db;        /* mix 24% */

  /* Text */
  --color-text-primary:   #111827;       /* cr=15.4 ✓ AAA */
  --color-text-secondary: #4b5563;       /* cr=7.1  ✓ AA  */
  --color-text-tertiary:  #9ca3af;       /* cr=3.0  ✓ UI  */

  /* Primary */
  --color-primary:            #2563eb;
  --color-primary-hover:      #1d4ed8;
  --color-primary-muted:      #eff6ff;
  --color-primary-foreground: #ffffff;   /* auto: white wins (cr=5.2) */

  /* Shadows (light mode) */
  --shadow-sm:    0 1px 3px rgba(0, 0, 0, 0.08);
  --shadow-md:    0 4px 12px rgba(0, 0, 0, 0.12);
  --shadow-lg:    0 8px 32px rgba(0, 0, 0, 0.16);
  --shadow-scrim: rgba(0, 0, 0, 0.50);

  /* Semantic */
  --color-danger:            #dc2626;
  --color-danger-hover:      #b91c1c;
  --color-danger-muted:      #fef2f2;    /* mix(bg, danger, 10%) */
  --color-danger-foreground: #ffffff;
  --color-danger-text:       #b91c1c;

  /* Focus */
  --color-focus-ring: #2563eb;           /* = primary, cr/bg=5.2 ✓ */
}
```

### 9.2 Debug Mode Output

Development builds include extra comments:

```css
/* [DEBUG] Contrast checks:
   text.primary/bg:      15.40 ✓ (min 7.0)
   text.secondary/bg:     7.10 ✓ (min 4.5)
   text.tertiary/bg:      3.02 ✓ (min 3.0)
   primary-fg/primary:    5.22 ✓ (min 4.5)
   focus-ring/bg:         5.22 ✓ (min 3.0)
   focus-ring/surface-1:  5.12 ✓ (min 3.0 warn)
   surface differentiation: all passes
*/
```

---

## Part 10 — Edge Cases & Known Limitations

### 10.1 Very Bright Primary Colors (e.g. Yellow)

```
Problem:
  yellow-foreground: white_or_black(#FFFF00) → black wins (cr=19.0)

  primary.50 (tint toward white bg):
    oklch_mix(white, yellow, 0.08) → nearly invisible on white bg
    → badge background indistinguishable from background

Solution: if contrast(primary-muted, background) < 1.15:
    render Badge with border stroke instead of background fill
    → component reads --color-primary-muted-style: 'border' | 'fill'
```

### 10.2 Low-contrast Background/Foreground Pair

```
User sets background = #808080, foreground = #707070
Contrast = 1.15:1 — FAR below minimum.

Pipeline response:
  Step 3 validation: BLOCK
  Error: "Background and foreground are too similar.
          Current contrast: 1.15:1. Minimum required: 7.0:1."

  Suggestion: "Try foreground = #1A1A1A (contrast 5.3:1 with #808080)"
```

### 10.3 Custom Theme + Vibrancy Interaction

Vibrancy does not require separate validation. When vibrancy is on, the sidebar background is `rgba(bg, 0.7)` on top of the wallpaper. The worst case (lightest wallpaper for dark themes, darkest for light themes) is already covered by existing contrast checks against `background` and `surface-1`.

---

## Summary — The Most Important Numbers

| Value | Number | Reason |
| --- | --- | --- |
| Body text contrast | ≥ 7.0:1 | AAA, safe in all rendering conditions |
| Secondary text | ≥ 4.5:1 | AA Normal, standard minimum |
| Tertiary text | ≥ 3.0:1 | UI minimum per WCAG 1.4.11 |
| Primary foreground | ≥ 4.5:1 | AA Normal for interactive text |
| Focus ring | ≥ 3.0:1 | WCAG 1.4.11 UI component |
| Surface ratios | 4/8/14/22% | Perceptually even steps |
| Text wash | 30% / 55% | Perceptually calibrated tiers |
| Hue lerp | Shortest arc | Avoids going through 180° |
| Shadow threshold | L > 0.18 | Light/dark shadow switch point |
| Color space | OKLCH | Perceptually uniform interpolation |

---

*This document is the implementation-layer spec. See Color Token Framework for the architecture overview, tokens.css for the CSS output reference.*