//! Colour that means something.
//!
//! A tile's hue says what kind of data it is ([`Category`]); every hue sits
//! at the same muted saturation and lightness, so no block stands out by
//! accident, and deeper tiles lift slightly so nesting reads without borders.
//! Reclaimable space is a hatch, not a colour, so "what is it" and "can it go"
//! are read independently.
//!
//! One strong colour is kept apart: the highlight, the theme's warning amber.
//! It marks the selection, the main action, reclaimable totals and the free
//! space after a removal, and nothing else, so the eye goes straight to it.
//!
//! Lightness, saturation and the surface every fill is pulled toward come
//! from the active Omarchy theme, so the mosaic sits inside it, light or dark.

use disktree_core::classify::Category;
use gpui_kit::base::ThemeAppearance;
use gpui_kit::{Hsla, Rgba};
use gpui_omarchy::Theme;

/// The hue a category is drawn in, and how much colour it carries. The
/// neutral kinds (documents, unknown) carry almost none.
const fn hue(category: Category) -> (f32, f32) {
    match category {
        Category::Code => (0.605, 1.0),
        Category::AgentScratch => (0.065, 1.0),
        Category::Toolchain => (0.415, 1.0),
        Category::Synced => (0.535, 1.0),
        Category::Git => (0.955, 1.0),
        Category::Media => (0.745, 1.0),
        Category::Cache => (0.125, 0.95),
        Category::Documents => (0.6, 0.18),
        Category::Other => (0.6, 0.08),
    }
}

const fn dark(theme: &Theme) -> bool {
    matches!(theme.appearance, ThemeAppearance::Dark)
}

/// The fill for a tile of `category`, `depth` levels into the view.
pub fn category_fill(theme: &Theme, category: Category, depth: u32) -> Hsla {
    let (h, chroma) = hue(category);
    let step = depth.min(4) as f32;
    let (s, l) = if dark(theme) {
        (0.26 * chroma, step.mul_add(0.028, 0.215))
    } else {
        (0.30 * chroma, step.mul_add(-0.03, 0.84))
    };
    // Pulled a little toward the theme surface, so each theme tints it.
    mix(Hsla { h, s, l, a: 1.0 }, theme.inset, 0.12)
}

/// The saturated version of a category's hue: the strip over a top-level
/// directory and the legend swatch.
pub fn category_accent(theme: &Theme, category: Category) -> Hsla {
    let (h, chroma) = hue(category);
    let (s, l) = if dark(theme) {
        (0.42 * chroma, 0.52)
    } else {
        (0.45 * chroma, 0.46)
    };
    Hsla { h, s, l, a: 1.0 }
}

/// A configured file-type colour as a tile fill, at `depth` levels into the
/// view.
///
/// Lifted a little with depth like a category fill, so nesting still reads
/// once colour no longer says it. The colour is otherwise left alone: it was
/// chosen on purpose, so it is not washed toward the theme surface.
pub fn type_fill(theme: &Theme, color: Hsla, depth: u32) -> Hsla {
    let step = depth.min(4) as f32;
    let delta = if dark(theme) {
        step * 0.028
    } else {
        -step * 0.03
    };
    Hsla {
        s: (color.s * 1.2).min(1.0),
        l: (color.l + delta).clamp(0.0, 1.0),
        ..color
    }
}

/// The saturated version of a file-type colour: the strip over a top-level
/// directory, matching [`category_accent`].
pub fn type_accent(theme: &Theme, color: Hsla) -> Hsla {
    Hsla {
        s: (color.s * 1.25).min(1.0),
        l: if dark(theme) { 0.52 } else { 0.46 },
        ..color
    }
}

fn white() -> Hsla {
    Hsla::from(Rgba {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        a: 1.0,
    })
}

fn black() -> Hsla {
    Hsla::from(Rgba {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    })
}

/// The eight ways a raised tile can be painted.
///
/// The CSS shapes they stand for use radial gradients, blurs and box shadows,
/// none of which gpui can draw: a tile is built from a solid or two-stop
/// linear body plus borders, which is what [`block_look`] hands the painter.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BlockStyle {
    /// The original lit block: a diagonal gradient with a bevelled edge.
    #[default]
    Classic,
    /// A glossy top-down sheen with a hard bottom shadow.
    Gloss,
    /// A dark ground with a bright edge of the tile's own hue.
    Neon,
    /// Brushed metal: a steep gradient and a bright band.
    Anodized,
    /// A solid body pressed in, with a dark inner ring.
    Embossed,
    /// A soft gradient with a light grain edge and a drop shadow.
    Grain,
    /// A solid body with four thick, differently lit cut edges.
    Chiseled,
    /// A hard fold down the middle, with hairline inner edges.
    Prism,
}

impl BlockStyle {
    /// Every style, in the dropdown's order.
    pub const ALL: [Self; 8] = [
        Self::Classic,
        Self::Gloss,
        Self::Neon,
        Self::Anodized,
        Self::Embossed,
        Self::Grain,
        Self::Chiseled,
        Self::Prism,
    ];

    /// The stable value the picker stores and hands back.
    pub const fn value(self) -> &'static str {
        match self {
            Self::Classic => "classic",
            Self::Gloss => "gloss",
            Self::Neon => "neon",
            Self::Anodized => "anodized",
            Self::Embossed => "embossed",
            Self::Grain => "grain",
            Self::Chiseled => "chiseled",
            Self::Prism => "prism",
        }
    }

    /// The picker entry's text, which is also its i18n key.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Classic => "Classic",
            Self::Gloss => "Gloss",
            Self::Neon => "Neon",
            Self::Anodized => "Anodized",
            Self::Embossed => "Embossed",
            Self::Grain => "Grain",
            Self::Chiseled => "Chiseled",
            Self::Prism => "Prism",
        }
    }

    /// The style a picker value names, the default one when it is unknown.
    pub fn from_value(value: &str) -> Self {
        Self::ALL
            .into_iter()
            .find(|style| style.value() == value)
            .unwrap_or_default()
    }

    /// The inner ring's thickness, in bevel units, a label has to clear.
    /// Zero for a style without a ring. The layout reserves this much on top
    /// of a header band so the inset text still fits inside it.
    pub const fn ring_bevels(self) -> f32 {
        match self {
            Self::Neon => 1.25,
            Self::Anodized | Self::Prism => 1.0,
            Self::Embossed => 3.0,
            Self::Chiseled => 1.5,
            Self::Classic | Self::Gloss | Self::Grain => 0.0,
        }
    }
}

/// One side of a tile's edge: how thick, in bevel units, and in what colour.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Edge {
    /// Multiples of the tile's bevel, which is a fraction of a rem.
    pub width: f32,
    pub color: Hsla,
}

impl Edge {
    pub const fn new(width: f32, color: Hsla) -> Self {
        Self { width, color }
    }
}

/// Everything one raised tile needs, worked out from its base colour.
#[derive(Clone, Copy, Debug)]
pub struct BlockLook {
    /// The body's gradient, as angle and its two ends; a solid fill when
    /// `None`.
    pub gradient: Option<(f32, Hsla, Hsla)>,
    pub body: Hsla,
    pub top: Edge,
    pub left: Edge,
    pub right: Edge,
    pub bottom: Edge,
    /// A uniform ring drawn inside the body.
    pub ring: Option<Edge>,
    /// Whether the tile casts a drop shadow behind it.
    pub shadow: bool,
}

/// The look for `style`, over a tile whose fill is `base`.
///
/// Every colour is derived from `base`, so a tile keeps the hue that says
/// what it is whatever its kind; the styles differ in contrast, direction,
/// edge weight and the layers drawn over the body.
pub fn block_look(style: BlockStyle, base: Hsla) -> BlockLook {
    let light = |t: f32| mix(base, white(), t);
    let dark = |t: f32| mix(base, black(), t);
    // A bright, saturated version of the tile's own hue, for neon edges.
    let glow = Hsla {
        s: base.s.mul_add(1.8, 0.35).min(1.0),
        l: 0.62,
        ..base
    };
    let edge = Edge::new;
    match style {
        BlockStyle::Classic => BlockLook {
            // The flat look the app had before blocks: a plain fill, no
            // gradient, no edge, nothing laid over it.
            gradient: None,
            body: base,
            top: edge(0.0, base),
            left: edge(0.0, base),
            right: edge(0.0, base),
            bottom: edge(0.0, base),
            ring: None,
            shadow: false,
        },
        BlockStyle::Gloss => BlockLook {
            // A sheen that still shows the tile's own colour. The first
            // version lit the top-left almost to white, so every tile read
            // grey and no longer matched its legend swatch.
            gradient: Some((160.0, light(0.35), dark(0.45))),
            body: base,
            top: edge(1.5, light(0.55)),
            left: edge(0.5, light(0.3)),
            right: edge(0.5, dark(0.5)),
            bottom: edge(1.5, dark(0.65)),
            ring: None,
            shadow: true,
        },
        BlockStyle::Neon => BlockLook {
            // A lit edge around a body that still shows the tile's colour:
            // the first version was so dark the hue was lost, and its glow
            // was heavy enough to swallow the edge.
            gradient: Some((145.0, dark(0.35), dark(0.6))),
            body: dark(0.45),
            top: edge(0.75, glow),
            left: edge(0.75, glow),
            right: edge(0.75, glow),
            bottom: edge(0.75, glow),
            ring: Some(edge(style.ring_bevels(), glow.opacity(0.35))),
            shadow: false,
        },
        BlockStyle::Anodized => BlockLook {
            gradient: Some((115.0, light(0.55), dark(0.55))),
            body: base,
            top: edge(1.0, light(0.9)),
            left: edge(1.0, light(0.7)),
            right: edge(2.0, dark(0.55)),
            bottom: edge(2.0, dark(0.75)),
            ring: Some(edge(style.ring_bevels(), light(0.35))),
            shadow: false,
        },
        BlockStyle::Embossed => BlockLook {
            gradient: None,
            body: base,
            top: edge(1.5, light(0.45)),
            left: edge(1.0, light(0.3)),
            right: edge(1.0, dark(0.4)),
            bottom: edge(1.5, dark(0.55)),
            ring: Some(edge(style.ring_bevels(), dark(0.5))),
            shadow: false,
        },
        BlockStyle::Grain => BlockLook {
            gradient: Some((135.0, light(0.4), dark(0.4))),
            body: base,
            top: edge(1.0, light(0.5)),
            left: edge(1.0, light(0.5)),
            right: edge(1.0, dark(0.5)),
            bottom: edge(1.0, dark(0.5)),
            ring: None,
            shadow: true,
        },
        BlockStyle::Chiseled => BlockLook {
            gradient: None,
            body: base,
            // Hairline cuts: the first version's lit top was a bar.
            top: edge(1.25, light(0.55)),
            left: edge(1.0, light(0.4)),
            right: edge(1.0, dark(0.4)),
            bottom: edge(1.75, dark(0.65)),
            ring: Some(edge(style.ring_bevels(), black().opacity(0.25))),
            shadow: false,
        },
        BlockStyle::Prism => BlockLook {
            gradient: Some((135.0, light(0.65), dark(0.6))),
            body: base,
            top: edge(1.0, light(0.85)),
            left: edge(1.0, light(0.6)),
            right: edge(1.0, dark(0.7)),
            bottom: edge(1.0, dark(0.85)),
            ring: Some(edge(style.ring_bevels(), dark(0.25))),
            shadow: false,
        },
    }
}

impl BlockLook {
    /// Whether the tile is a plain fill: no gradient, no edge, no ring and no
    /// shadow. That is the "Classic" style, which is the look the app had
    /// before blocks existed.
    pub fn is_flat(&self) -> bool {
        self.gradient.is_none()
            && self.top.width == 0.0
            && self.left.width == 0.0
            && self.right.width == 0.0
            && self.bottom.width == 0.0
            && self.ring.is_none()
            && !self.shadow
    }

    /// The colour of the tile's top-left corner, which is where its label
    /// sits: the gradient's lit end, or the body when there is no gradient.
    pub fn top_left(&self) -> Hsla {
        self.gradient.map_or(self.body, |(_, from, _)| from)
    }
}

/// Text that reads on `top`, the colour the label is drawn over: dark on a
/// light corner, light on a dark one. Without this a near-white style (and
/// anodized is one) had the theme's light text on it and none of it read.
pub fn label_on(top: Hsla) -> Hsla {
    let rgb = top.to_rgb();
    let luminance =
        0.2126f32.mul_add(rgb.r, 0.7152f32.mul_add(rgb.g, 0.0722 * rgb.b));
    if luminance * 255.0 > 127.0 {
        Hsla::from(Rgba {
            r: 0.06,
            g: 0.07,
            b: 0.09,
            a: 1.0,
        })
    } else {
        Hsla::from(Rgba {
            r: 0.97,
            g: 0.97,
            b: 0.98,
            a: 1.0,
        })
    }
}

/// The age ramp, newest first: this week, this month, this half-year, this
/// year, older.
pub const AGE_BUCKETS: [(i64, &str); 5] = [
    (7, "This week"),
    (30, "This month"),
    (182, "Six months"),
    (365, "This year"),
    (i64::MAX, "Older"),
];

/// Which [`AGE_BUCKETS`] entry an age in days falls in.
pub fn age_bucket(days: i64) -> usize {
    AGE_BUCKETS
        .iter()
        .position(|(limit, _)| days <= *limit)
        .unwrap_or(AGE_BUCKETS.len() - 1)
}

/// The fill for age mode: recent writes carry the theme accent, and colour
/// drains out of a tile as it goes untouched.
pub fn age_fill(theme: &Theme, bucket: usize, depth: u32) -> Hsla {
    let fade = bucket.min(AGE_BUCKETS.len() - 1) as f32 / 4.0;
    let step = depth.min(4) as f32;
    let (s, l) = if dark(theme) {
        (
            (1.0 - fade).mul_add(0.34, 0.03),
            step.mul_add(0.028, 0.29 - fade * 0.09),
        )
    } else {
        (
            (1.0 - fade).mul_add(0.36, 0.04),
            step.mul_add(-0.03, 0.74 + fade * 0.1),
        )
    };
    mix(
        Hsla {
            h: theme.accent.h,
            s,
            l,
            a: 1.0,
        },
        theme.inset,
        0.1,
    )
}

/// The age swatch for the legend.
pub fn age_accent(theme: &Theme, bucket: usize) -> Hsla {
    let fade = bucket.min(AGE_BUCKETS.len() - 1) as f32 / 4.0;
    let l = if dark(theme) {
        0.55 - fade * 0.25
    } else {
        0.45 + fade * 0.25
    };
    Hsla {
        h: theme.accent.h,
        s: (1.0 - fade).mul_add(0.45, 0.04),
        l,
        a: 1.0,
    }
}

/// The one strong colour: selection, the main action, what can be had back.
pub const fn highlight(theme: &Theme) -> Hsla {
    theme.warning
}

/// Text on a filled highlight.
pub const fn on_highlight(theme: &Theme) -> Hsla {
    if dark(theme) {
        theme.background
    } else {
        theme.bright
    }
}

/// The diagonal hatch over reclaimable space: quiet enough to leave the hue
/// readable, visible on every fill.
pub fn hatch(theme: &Theme) -> Hsla {
    if dark(theme) {
        theme.bright.opacity(0.16)
    } else {
        theme.foreground.opacity(0.18)
    }
}

/// Linear interpolation between two colours, in RGB: interpolating hue
/// would drag a colour around the wheel on its way to a grey.
pub fn mix(from: Hsla, to: Hsla, t: f32) -> Hsla {
    let t = t.clamp(0.0, 1.0);
    let lerp = |a: f32, b: f32| (b - a).mul_add(t, a);
    let (a, b) = (from.to_rgb(), to.to_rgb());
    Hsla::from(Rgba {
        r: lerp(a.r, b.r),
        g: lerp(a.g, b.g),
        b: lerp(a.b, b.b),
        a: lerp(a.a, b.a),
    })
}

/// A tile's name on top of its fill.
pub fn label_color(theme: &Theme, depth: u32) -> Hsla {
    let base = if dark(theme) {
        theme.bright
    } else {
        theme.foreground
    };
    if depth == 0 { base } else { base.opacity(0.88) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn theme(appearance: ThemeAppearance) -> Theme {
        match appearance {
            ThemeAppearance::Dark => Theme::tokyo_night(),
            ThemeAppearance::Light => Theme::flexoki_light(),
        }
    }

    #[test]
    fn every_legend_category_has_its_own_hue() {
        let theme = theme(ThemeAppearance::Dark);
        let fills: Vec<Hsla> = Category::LEGEND
            .iter()
            .map(|&category| category_accent(&theme, category))
            .collect();
        for (index, left) in fills.iter().enumerate() {
            for right in &fills[index + 1..] {
                let apart = (left.h - right.h).abs() > 0.03
                    || (left.s - right.s).abs() > 0.1;
                assert!(apart, "{left:?} and {right:?} read as one colour");
            }
        }
    }

    #[test]
    fn colourful_categories_share_one_level() {
        let theme = theme(ThemeAppearance::Dark);
        let code = category_fill(&theme, Category::Code, 0);
        let git = category_fill(&theme, Category::Git, 0);
        assert!((code.l - git.l).abs() < 0.02);
        assert!((code.s - git.s).abs() < 0.03);
    }

    #[test]
    fn deeper_tiles_lift_away_from_the_background() {
        for appearance in [ThemeAppearance::Dark, ThemeAppearance::Light] {
            let theme = theme(appearance);
            let top = category_fill(&theme, Category::Code, 0);
            let deep = category_fill(&theme, Category::Code, 3);
            // Away from the background: lighter on dark, darker on light.
            let distance = |fill: Hsla| (fill.l - theme.background.l).abs();
            assert!(distance(deep) > distance(top) + 0.05, "{appearance:?}");
        }
    }

    #[test]
    fn the_highlight_is_not_a_category_colour() {
        let theme = theme(ThemeAppearance::Dark);
        let highlight = highlight(&theme);
        for category in Category::LEGEND {
            let fill = category_fill(&theme, category, 0);
            assert!(highlight.s - fill.s > 0.2, "{category:?} competes");
        }
    }

    #[test]
    fn age_buckets_cover_every_age_in_order() {
        assert_eq!(age_bucket(0), 0);
        assert_eq!(age_bucket(8), 1);
        assert_eq!(age_bucket(100), 2);
        assert_eq!(age_bucket(300), 3);
        assert_eq!(age_bucket(5000), 4);
        let theme = theme(ThemeAppearance::Dark);
        assert!(age_fill(&theme, 0, 0).s > age_fill(&theme, 4, 0).s);
    }

    #[test]
    fn mixing_toward_a_grey_keeps_the_hue() {
        let theme = theme(ThemeAppearance::Dark);
        let orange = category_accent(&theme, Category::AgentScratch);
        let mixed = mix(orange, theme.inset, 0.3);
        assert!((mixed.h - orange.h).abs() < 0.02, "{mixed:?}");
    }

    #[test]
    fn mix_clamps_its_parameter() {
        let theme = theme(ThemeAppearance::Dark);
        let clamped_low = mix(theme.background, theme.accent, -1.0).l;
        let clamped_high = mix(theme.background, theme.accent, 2.0).l;
        assert!((clamped_low - theme.background.l).abs() < 1e-3);
        assert!((clamped_high - theme.accent.l).abs() < 1e-3);
    }

    #[test]
    fn the_classic_block_is_the_flat_look() {
        let base = Hsla {
            h: 0.6,
            s: 0.3,
            l: 0.5,
            a: 1.0,
        };
        let look = block_look(BlockStyle::Classic, base);
        assert!(look.is_flat(), "classic is the plain fill");
        assert!(
            (look.body.l - base.l).abs() < 1e-6,
            "and it is the tile's own colour"
        );
        assert!((look.top_left().l - base.l).abs() < 1e-6);
    }

    #[test]
    fn a_label_is_dark_on_a_light_tile_and_light_on_a_dark_one() {
        let light = Hsla {
            h: 0.1,
            s: 0.2,
            l: 0.9,
            a: 1.0,
        };
        let dark = Hsla {
            h: 0.6,
            s: 0.5,
            l: 0.2,
            a: 1.0,
        };
        assert!(label_on(light).l < 0.3, "dark text on a light tile");
        assert!(label_on(dark).l > 0.7, "light text on a dark tile");
    }

    #[test]
    fn the_light_styles_take_dark_text() {
        let base = Hsla {
            h: 0.11,
            s: 0.7,
            l: 0.5,
            a: 1.0,
        };
        for style in [
            BlockStyle::Anodized,
            BlockStyle::Gloss,
            BlockStyle::Grain,
            BlockStyle::Prism,
        ] {
            let look = block_look(style, base);
            assert!(
                label_on(look.top_left()).l < 0.5,
                "{style:?} has a light corner and needs dark text"
            );
        }
    }

    /// The gloss sheen must not wash the tile out to grey: its lit corner
    /// keeps most of the tile's colour, so a tile still matches its legend
    /// swatch.
    #[test]
    fn gloss_keeps_the_tile_colour() {
        let base = Hsla {
            h: 0.6,
            s: 0.5,
            l: 0.5,
            a: 1.0,
        };
        let corner = block_look(BlockStyle::Gloss, base).top_left();
        assert!(
            corner.s > base.s * 0.55,
            "the sheen keeps the tile's saturation: {corner:?}"
        );
        assert!((corner.h - base.h).abs() < 0.05, "and its hue: {corner:?}");
    }

    #[test]
    fn every_block_style_derives_its_look_from_the_tile_colour() {
        let base = Hsla {
            h: 0.55,
            s: 0.55,
            l: 0.5,
            a: 1.0,
        };
        for style in BlockStyle::ALL {
            let look = block_look(style, base);
            assert!(look.body.a > 0.99, "{style:?} body is opaque");
            if look.is_flat() {
                assert!(
                    (look.body.l - base.l).abs() < 1e-6,
                    "{style:?} paints the tile's own colour"
                );
                continue;
            }
            assert!((look.body.h - base.h).abs() < 0.02, "{style:?} body hue");
            for edge in [look.top, look.left, look.right, look.bottom] {
                assert!(edge.width > 0.0, "{style:?} has a zero-width edge");
                assert!(
                    (edge.color.l - look.body.l).abs() > 0.05,
                    "{style:?} edge has no contrast with the body"
                );
            }
            if let Some((angle, from, to)) = look.gradient {
                assert!(angle.is_finite(), "{style:?} angle");
                assert!(
                    (from.l - to.l).abs() > 0.03,
                    "{style:?} gradient has no contrast"
                );
            }
        }
    }

    #[test]
    fn every_block_style_is_a_distinct_look() {
        let base = Hsla {
            h: 0.55,
            s: 0.55,
            l: 0.5,
            a: 1.0,
        };
        let signature = |style: BlockStyle| {
            let look = block_look(style, base);
            (
                look.gradient.map(|(angle, from, to)| (angle, from.l, to.l)),
                look.body.l,
                look.top.width,
                look.top.color.l,
                look.left.width,
                look.right.width,
                look.bottom.width,
                look.bottom.color.l,
                look.ring
                    .map(|edge| (edge.width, edge.color.l, edge.color.a)),
                look.shadow,
            )
        };
        let looks: Vec<_> =
            BlockStyle::ALL.into_iter().map(signature).collect();
        for (index, left) in looks.iter().enumerate() {
            for right in &looks[index + 1..] {
                assert_ne!(left, right, "two styles paint the same");
            }
        }
    }

    #[test]
    fn a_style_value_round_trips_and_defaults_to_classic() {
        for style in BlockStyle::ALL {
            assert_eq!(BlockStyle::from_value(style.value()), style);
            assert!(!style.label().is_empty());
        }
        assert_eq!(BlockStyle::from_value("nonsense"), BlockStyle::Classic);
        assert_eq!(BlockStyle::default(), BlockStyle::Classic);
    }

    #[test]
    fn a_file_type_colour_is_not_washed_out() {
        let theme = theme(ThemeAppearance::Dark);
        let color = Hsla {
            h: 0.02,
            s: 0.9,
            l: 0.55,
            a: 1.0,
        };
        let fill = type_fill(&theme, color, 0);
        // At the top level the colour is kept: its saturation is nudged up,
        // never pulled toward the theme surface.
        assert!(fill.s >= color.s, "saturation is not drained");
        assert!((fill.l - color.l).abs() < 1e-6, "lightness is untouched");
    }

    #[test]
    fn a_file_type_colour_still_lifts_with_depth() {
        for appearance in [ThemeAppearance::Dark, ThemeAppearance::Light] {
            let theme = theme(appearance);
            let color = Hsla {
                h: 0.1,
                s: 0.6,
                l: 0.5,
                a: 1.0,
            };
            let top = type_fill(&theme, color, 0);
            let deep = type_fill(&theme, color, 3);
            // Away from the background: lighter on dark, darker on light.
            let distance = |fill: Hsla| (fill.l - theme.background.l).abs();
            assert!(distance(deep) > distance(top) + 0.05, "{appearance:?}");
        }
    }
}
