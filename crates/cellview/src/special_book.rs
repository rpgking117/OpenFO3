//! Fallout 3's "You're SPECIAL!" book on screen (`SPECIALBookMenu`,
//! Fallout3.exe 1.7.0.3): its model placed in front of a camera of its own,
//! the pictures its pages swap in, which of its pieces show, and the button
//! a click finds. The book's rules (pages, points) are
//! `world::chargen::special_book`.
//!
//! * The model (`00676b70`): `Meshes\Terminals\Babybook02.NIF`, its root
//!   replaced by scale 0.05, a quarter turn about X (`00409b20`) and
//!   (0, 0, −60); lit by one point light at the origin, diffuse 0.85, its
//!   radius 20 × the placed model's bound radius.
//! * The camera (`00675660`): at (0, 0, 1) looking at the origin, +Y up,
//!   half as wide as `tan(fDefaultFOV × 0.15) × 0.75`.
//! * The pictures (`00675030`): `BBNumber0`–`10`, `ENGLISH\BBMessage01`–`04`,
//!   and the PC buttons (the gamepad's are `XBOX\`).
//! * What shows (each page's refresh functions): a value as its number
//!   (`00675d90`); the message by the points left (`00675de0`); the points
//!   left as two digits (`00675e50`); minus from 2 (`00675f10`), plus below
//!   10 while points are left (`00675f80`); the arrows (`00676010`,
//!   `00676040`); "All done" grey until every point is spent (`00676070`).
//! * A click (`vf03` id 5) tries the page's buttons in the order they were
//!   made (`00676b70`), on pages 0 to 8 only.
//!
//! Not traced: `00478d70`, which the number, message and plus/minus paths go
//! through (it reads the gamepad flags); the files are used as named.

use std::sync::Arc;

use nif::math::Transform;

pub const MODEL: &str = "meshes\\terminals\\babybook02.nif";
/// The book's placement's number in its scene.
pub const REFERENCE: u32 = 1;
/// The attributes as the index page's pieces name them.
pub const ATTRIBUTES: [&str; 7] = [
    "Strength",
    "Perception",
    "Endurance",
    "Charisma",
    "Intelligence",
    "Agility",
    "Luck",
];
/// The model's placement (`00676b70`).
pub const SCALE: f32 = 0.05;
pub const DEPTH: f32 = -60.0;
/// The camera's share of `fDefaultFOV` (`00675660`).
pub const FOV_SHARE: f32 = 0.15;

/// A picture a piece is given (`00675030`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Picture {
    /// 0 to 10.
    Number(u8),
    /// 1 to 4: "Assign your points!", "Points to go!", "Point to go!",
    /// "All points assigned!".
    Message(u8),
    Right,
    Left,
    /// "All done": dark once every point is spent (`BBXOff`), grey before
    /// (`BBXOn`).
    Done { ready: bool },
}

impl Picture {
    pub fn path(self) -> String {
        match self {
            Picture::Number(n) => format!("textures\\terminals\\BBNumber{n}.dds"),
            Picture::Message(n) => format!("textures\\terminals\\ENGLISH\\BBMessage0{n}.dds"),
            Picture::Right => "textures\\terminals\\PC\\BBRTOff.dds".into(),
            Picture::Left => "textures\\terminals\\PC\\BBLTOff.dds".into(),
            Picture::Done { ready } => format!(
                "textures\\terminals\\PC\\BBX{}.dds",
                if ready { "Off" } else { "On" }
            ),
        }
    }

    /// Every picture the book swaps in.
    pub fn all() -> Vec<Picture> {
        (0..=10)
            .map(Picture::Number)
            .chain((1..=4).map(Picture::Message))
            .chain([
                Picture::Right,
                Picture::Left,
                Picture::Done { ready: true },
                Picture::Done { ready: false },
            ])
            .collect()
    }
}

/// What a button does when clicked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `00676a20`.
    Next,
    /// `00676a50`.
    Previous,
    /// `00676880`, on an attribute (0 Strength .. 6 Luck).
    Decrease(usize),
    /// `00676940`.
    Increase(usize),
    /// `00676360`.
    Done,
}

/// What the pieces show from: the page, the index's focus, whether the
/// gamepad is in use (`00477b00`), the seven values and the points.
#[derive(Debug, Clone, PartialEq)]
pub struct Display {
    pub page: u8,
    pub focus: i32,
    pub gamepad: bool,
    pub values: [i32; 7],
    pub points: i32,
}

/// A piece's name without its `:0`.
fn stem(name: &str) -> &str {
    name.split(':').next().unwrap_or(name)
}

/// An attribute page's piece: (page 1–7, the rest of the name).
fn page_piece(name: &str) -> Option<(usize, &str)> {
    let rest = name.strip_prefix('P')?;
    let (n, part) = rest.split_once('_')?;
    let n: usize = n.parse().ok()?;
    (1..=7).contains(&n).then_some((n, part))
}

/// An index piece naming an attribute: (attribute, the rest).
fn index_piece(name: &str) -> Option<(usize, &str)> {
    let rest = name.strip_prefix("Index_")?;
    ATTRIBUTES
        .iter()
        .enumerate()
        .find_map(|(i, a)| rest.strip_prefix(a).map(|part| (i, part)))
}

impl Display {
    fn total(&self) -> i32 {
        self.values.iter().sum()
    }

    fn left(&self) -> i32 {
        self.points - self.total()
    }

    /// The number picture for a value; none outside 0–10 (the game would
    /// read past its eleven).
    fn number(value: i32) -> Option<Picture> {
        u8::try_from(value)
            .ok()
            .filter(|v| *v <= 10)
            .map(Picture::Number)
    }

    /// Whether minus (`00675f10`) or plus (`00675f80`) on an attribute
    /// shows. With the gamepad the index shows only the focused row's.
    fn button_shown(&self, attribute: usize, increase: bool) -> bool {
        let value = self.values[attribute];
        let shown = if increase {
            value < 10 && self.total() < self.points
        } else {
            value >= 2
        };
        let unfocused = self.gamepad && self.page == 8 && attribute as i32 + 1 != self.focus;
        shown && !unfocused
    }

    /// The message picture (`00675de0`); none keeps the one shown.
    fn message(&self) -> Option<Picture> {
        let left = self.left();
        match left {
            l if l > 1 => Some(Picture::Message(2)),
            1 => Some(Picture::Message(3)),
            0 => Some(Picture::Message(4)),
            _ => None,
        }
    }

    /// (shown, the picture to give it) for a piece by its name. Pieces the
    /// menu doesn't touch show as the file has them.
    pub fn appearance(&self, name: &str) -> (bool, Option<Picture>) {
        let name = stem(name);
        let left = self.left();
        let part = page_piece(name)
            .map(|(_, part)| part)
            .or_else(|| name.strip_prefix("Index_"));
        match part {
            Some("RT_Btn") => return (true, Some(Picture::Right)),
            Some("LT_Btn") => return (true, Some(Picture::Left)),
            Some("Message") => return (true, self.message()),
            // `00675e50`: none at 0; the tens only from 10; a negative
            // number would read past the pictures, so the last stays.
            Some("PointsRemainDigit1") => {
                return if left < 10 {
                    (false, None)
                } else {
                    (true, Self::number((left / 10).min(10)))
                }
            }
            Some("PointsRemainDigit2") => {
                return if left == 0 {
                    (false, None)
                } else if left < 0 {
                    (true, None)
                } else {
                    (true, Self::number(left % 10))
                }
            }
            _ => {}
        }
        match name {
            "LookInside_Btn" => return (true, Some(Picture::Right)),
            "AllDone_Btn" => {
                return (
                    true,
                    Some(Picture::Done {
                        ready: self.total() >= self.points,
                    }),
                )
            }
            _ => {}
        }
        let attribute = page_piece(name)
            .map(|(n, part)| (n - 1, part))
            .or_else(|| index_piece(name));
        if let Some((a, part)) = attribute {
            match part {
                "PointVal" => return (true, Self::number(self.values[a])),
                "Decrease_Btn" => return (self.button_shown(a, false), None),
                "Increase_Btn" => return (self.button_shown(a, true), None),
                _ => {}
            }
        }
        (true, None)
    }
}

/// A page's buttons in the order a click tries them (`00676b70`); none on
/// the back cover (`vf03` id 5 stops past page 8).
pub fn buttons(page: u8) -> Vec<String> {
    match page {
        0 => vec!["LookInside_Btn:0".into()],
        1..=7 => ["Decrease_Btn", "Increase_Btn", "RT_Btn", "LT_Btn"]
            .iter()
            .map(|b| format!("P{page}_{b}:0"))
            .collect(),
        8 => ATTRIBUTES
            .iter()
            .flat_map(|a| [format!("Index_{a}Decrease_Btn:0"), format!("Index_{a}Increase_Btn:0")])
            .chain(["AllDone_Btn:0".into(), "Index_LT_Btn:0".into()])
            .collect(),
        _ => Vec::new(),
    }
}

/// What a button does.
pub fn action(name: &str) -> Option<Action> {
    let name = stem(name);
    if name == "LookInside_Btn" {
        return Some(Action::Next);
    }
    if name == "AllDone_Btn" {
        return Some(Action::Done);
    }
    if name == "Index_LT_Btn" {
        return Some(Action::Previous);
    }
    let attribute = page_piece(name)
        .map(|(n, part)| (n - 1, part))
        .or_else(|| index_piece(name));
    match attribute? {
        (_, "RT_Btn") => Some(Action::Next),
        (_, "LT_Btn") => Some(Action::Previous),
        (a, "Decrease_Btn") => Some(Action::Decrease(a)),
        (a, "Increase_Btn") => Some(Action::Increase(a)),
        _ => None,
    }
}

/// The model's root (`00676b70`): the file's replaced by scale 0.05, a
/// quarter turn about X as `00409b20` makes it, and (0, 0, −60).
pub fn model_transform() -> Transform {
    let (s, c) = std::f32::consts::FRAC_PI_2.sin_cos();
    Transform {
        rotation: [[1.0, 0.0, 0.0], [0.0, c, s], [0.0, -s, c]],
        translation: [0.0, 0.0, DEPTH],
        scale: SCALE,
    }
}

/// The book's camera (`00675660`).
pub fn camera(settings: &assets::IniSettings, width: u32, height: u32) -> crate::lockpick::MenuCamera {
    crate::lockpick::MenuCamera::with_share(settings, width, height, FOV_SHARE)
}

/// The model ready to draw: its pieces with their own transforms (the
/// file's root removed; [`model_transform`] goes over every one), its
/// sequences, the light and the pictures.
pub struct BookScene {
    pub scene: crate::ViewerScene,
    pub sequences: Arc<Vec<nif::Sequence>>,
    pub lights: Vec<crate::LightData>,
    pub pictures: Vec<(Picture, usize)>,
    /// The buttons' own vertices, for picking.
    buttons: Vec<nif::Mesh>,
}

impl BookScene {
    pub fn load(assets: &assets::Assets) -> Result<Self, crate::Error> {
        let bytes = assets
            .read(MODEL)
            .map_err(|e| crate::Error(e.to_string()))?
            .ok_or_else(|| crate::Error(format!("Missing the book's model: {MODEL}")))?;
        let nif = nif::Nif::parse(bytes).map_err(|e| crate::Error(format!("{MODEL}: {e}")))?;
        let buttons = nif
            .placed_scene()
            .map_err(|e| crate::Error(e.to_string()))?
            .meshes
            .into_iter()
            .filter(|m| m.name.ends_with("_Btn:0"))
            .collect();
        // The placed model's bound (`00676b70` reads it after the update
        // that follows the placement).
        let root = model_transform();
        let radius = nif
            .roots()
            .first()
            .and_then(|&r| crate::lockpick::bound(&nif, r, &Transform::IDENTITY, Some(&root)))
            .map_or(0.0, |(_, radius)| radius);
        let lights = vec![crate::LightData {
            position: [0.0; 3],
            color: [0.85; 3],
            radius: 20.0 * radius,
            fade: 1.0,
        }];
        let sequences = Arc::new(nif.sequences().map_err(|e| crate::Error(e.to_string()))?);
        let mut cell = world::LoadedCell::actors_only(Vec::new());
        cell.objects.push(world::Placement {
            form_id: esm::FormId(REFERENCE),
            record_type: esm::FourCC::new(b"REFR"),
            editor_id: None,
            base: esm::FormId(0),
            base_type: esm::FourCC::new(b"STAT"),
            base_editor_id: None,
            position: [0.0; 3],
            rotation: [0.0; 3],
            scale: 1.0,
            model: Some(MODEL.into()),
            parts: Vec::new(),
            light: None,
            radius: None,
            teleport: None,
            emittance: None,
            flags: 0,
            enable_parent: None,
            plugin: String::new(),
            actor: None,
            primitive: None,
            open_by_default: false,
        });
        let built = preview::cell::build_scene(assets, cell);
        let mut cache = crate::TextureCache::new(assets);
        let mut scene = crate::convert(&built, &mut cache);
        let pictures = Picture::all()
            .into_iter()
            .map(|picture| {
                let path = picture.path();
                cache
                    .get(&path)
                    .map(|index| (picture, index))
                    .ok_or_else(|| crate::Error(format!("Missing the book's picture: {path}")))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let (textures, notes) = cache.finish();
        scene.textures = textures;
        scene.notes.extend(notes);
        if scene.draws.is_empty() {
            return Err(crate::Error("The book's model has no pieces to draw".into()));
        }
        Ok(Self {
            scene,
            sequences,
            lights,
            pictures,
            buttons,
        })
    }

    /// The button on `page` the ray from `origin` along `direction` (the
    /// camera's space) meets first, among those showing.
    pub fn pick_button(
        &self,
        page: u8,
        display: &Display,
        origin: [f32; 3],
        direction: [f32; 3],
        layer: Option<(&nif::Sequence, f32)>,
    ) -> Option<&str> {
        for name in buttons(page) {
            if !display.appearance(&name).0 {
                continue;
            }
            let Some(mesh) = self.buttons.iter().find(|m| m.name == name) else {
                continue;
            };
            let layers: Vec<_> = layer.into_iter().collect();
            let inverse = model_transform()
                .then_child(&mesh.posed_transform(&layers))
                .inverse();
            let local_origin = inverse.apply_point(origin);
            let local_direction = inverse
                .apply_direction(direction)
                .map(|v| v * inverse.scale);
            let hit = mesh.triangles.iter().any(|t| {
                let corner = |i: u16| mesh.positions.get(usize::from(i)).copied();
                match (corner(t[0]), corner(t[1]), corner(t[2])) {
                    (Some(a), Some(b), Some(c)) => {
                        crate::vigor::pick_triangle(local_origin, local_direction, a, b, c)
                    }
                    _ => false,
                }
            });
            if hit {
                return Some(&mesh.name);
            }
        }
        None
    }

    /// A piece's transform as the sequence poses it, under the model's
    /// root. No sequence leaves the file's pose.
    pub fn piece_matrix(
        motion: Option<&crate::PieceMotion>,
        layer: Option<(&nif::Sequence, f32)>,
    ) -> [f32; 16] {
        let root = model_transform();
        let transform = match (motion, layer) {
            (Some(motion), Some(layer)) => root.then_child(&motion.motion.transform(&[layer])),
            _ => root,
        };
        crate::column_major(&transform)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn display(values: [i32; 7], points: i32) -> Display {
        Display {
            page: 1,
            focus: 1,
            gamepad: false,
            values,
            points,
        }
    }

    /// Five points left: "Points to go!", the ones digit alone, plus and
    /// minus shown, "All done" grey.
    #[test]
    fn points_left_show_as_the_book_shows_them() {
        let d = display([5; 7], 40);
        assert_eq!(d.appearance("P1_Message:0"), (true, Some(Picture::Message(2))));
        assert_eq!(d.appearance("P3_PointsRemainDigit1:0"), (false, None));
        assert_eq!(
            d.appearance("Index_PointsRemainDigit2:0"),
            (true, Some(Picture::Number(5)))
        );
        assert_eq!(d.appearance("P2_PointVal:0"), (true, Some(Picture::Number(5))));
        assert_eq!(
            d.appearance("Index_LuckPointVal:0"),
            (true, Some(Picture::Number(5)))
        );
        assert!(d.appearance("P1_Increase_Btn:0").0);
        assert!(d.appearance("Index_AgilityDecrease_Btn:0").0);
        assert_eq!(
            d.appearance("AllDone_Btn:0"),
            (true, Some(Picture::Done { ready: false }))
        );
        assert_eq!(d.appearance("P4_RT_Btn:0"), (true, Some(Picture::Right)));
        assert_eq!(d.appearance("Index_LT_Btn:0"), (true, Some(Picture::Left)));
        // One left, then none.
        let one = display([5, 5, 5, 5, 5, 5, 9], 40);
        assert_eq!(one.appearance("P1_Message:0").1, Some(Picture::Message(3)));
        let none = display([10, 5, 5, 5, 5, 5, 5], 40);
        assert_eq!(none.appearance("P1_Message:0").1, Some(Picture::Message(4)));
        assert_eq!(none.appearance("P1_PointsRemainDigit2:0"), (false, None));
        assert!(!none.appearance("P2_Increase_Btn:0").0);
        assert!(!none.appearance("P1_Increase_Btn:0").0);
        assert_eq!(
            none.appearance("AllDone_Btn:0").1,
            Some(Picture::Done { ready: true })
        );
        // Twelve left: both digits.
        let twelve = display([4, 4, 4, 4, 4, 4, 4], 40);
        assert_eq!(
            twelve.appearance("P1_PointsRemainDigit1:0"),
            (true, Some(Picture::Number(1)))
        );
        assert_eq!(
            twelve.appearance("P1_PointsRemainDigit2:0"),
            (true, Some(Picture::Number(2)))
        );
        // Minus hides at 1.
        let low = display([1, 5, 5, 5, 5, 5, 5], 40);
        assert!(!low.appearance("P1_Decrease_Btn:0").0);
        assert!(!low.appearance("Index_StrengthDecrease_Btn:0").0);
    }

    /// With the gamepad the index shows only the focused row's buttons.
    #[test]
    fn the_gamepad_shows_the_focused_row_only() {
        let mut d = display([5; 7], 40);
        d.gamepad = true;
        d.page = 8;
        d.focus = 2;
        assert!(!d.appearance("Index_StrengthIncrease_Btn:0").0);
        assert!(d.appearance("Index_PerceptionIncrease_Btn:0").0);
    }

    #[test]
    fn buttons_act_as_the_game_wires_them() {
        assert_eq!(action("LookInside_Btn:0"), Some(Action::Next));
        assert_eq!(action("P3_RT_Btn:0"), Some(Action::Next));
        assert_eq!(action("P3_LT_Btn:0"), Some(Action::Previous));
        assert_eq!(action("P7_Increase_Btn:0"), Some(Action::Increase(6)));
        assert_eq!(action("Index_CharismaDecrease_Btn:0"), Some(Action::Decrease(3)));
        assert_eq!(action("AllDone_Btn:0"), Some(Action::Done));
        assert_eq!(action("Back_LT_Btn:0"), None);
        assert_eq!(buttons(0), vec!["LookInside_Btn:0".to_string()]);
        assert_eq!(buttons(5)[1], "P5_Increase_Btn:0");
        assert_eq!(buttons(8).len(), 16);
        assert!(buttons(9).is_empty());
    }

    /// `00409b20` for π/2 about X: y takes z, z takes −y.
    #[test]
    fn the_book_lies_as_the_game_turns_it() {
        let t = model_transform();
        let p = t.apply_point([20.0, -40.0, 60.0]);
        assert!((p[0] - 1.0).abs() < 1e-5);
        assert!((p[1] - 3.0).abs() < 1e-4);
        assert!((p[2] - (2.0 - 60.0)).abs() < 1e-4);
    }
}
