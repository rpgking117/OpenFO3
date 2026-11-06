//! The face menu (`menus\chargen\race_sex_menu.xml`, class `RaceSexMenu`
//! 1036, vtable `00e072e4` in `Fallout3.exe` 1.7.0.3). Read from the code,
//! not the New Vegas notes in `docs/RACE_SEX_MENU.md`: Fallout 3 has no Age
//! slider and its item numbers differ from Hair on.
//!
//! * Twenty pages (`0066ffc0`), each a header and a list of items made from
//!   two templates, kept in order from `_top_bound` down (`0066c330`). The
//!   menu's `user0` is the page on show, which the templates compare with
//!   their own `user0` to decide what's visible.
//! * Pages 0 to 3 run Sex → Race → Face → Hair by Back and Next (`0066b050`);
//!   a link's target goes back to the page linking to it (`0066c7c0`). On the
//!   last page (3, or 2 for the surgeon) Next is Done and asks first.
//! * A click (`00670e40`): an item by its kind, else Back (id 4), Next (5), a
//!   slider's part (ids 100 to 105, `00670a30`), or the list's scroll arrows.
//! * The pointer dragging the face (`RSM_Face_Grab`, id 1) turns it a full
//!   circle across the tile's width, and a slider's marker moves it
//!   (`0066b8f0`, `00670830`). The wheel over the face zooms in eighths toward
//!   a target it accelerates to (`006711e0`); anywhere else it scrolls.
//!
//! What's on the game's side goes out as [`Request`]s and comes back as a
//! [`Snapshot`]: `viewer::game_menus::race_sex` applies them with
//! `world::chargen::race_sex`.
//!
//! Not here: the controller's and keyboard's list navigation (`0066b690`,
//! `0066c1e0`, the face turned by the stick) and the turning and zooming
//! sounds (`+0xf0` to the 4 sound handles of `006711e0`).

use crate::menu::{self, MenuCode};
use crate::names::t;
use crate::tile::{TileId, Ui};

/// The menu's file.
pub const FILE: &str = "menus\\chargen\\race_sex_menu.xml";
/// Its class number.
pub const CLASS: i32 = 1036;
/// "No page" (`0066c6d0`).
pub const NO_PAGE: usize = 20;

/// The menu's tile ids and a slider's parts.
pub mod id {
    pub const BACKGROUND: i32 = 0;
    pub const FACE_GRAB: i32 = 1;
    pub const SCROLL_UP: i32 = 2;
    pub const SCROLL_DOWN: i32 = 3;
    pub const BACK: i32 = 4;
    pub const NEXT: i32 = 5;
    pub const ARROW_LEFT: i32 = 100;
    pub const BAR: i32 = 101;
    pub const BAR_LEFT: i32 = 102;
    pub const BAR_RIGHT: i32 = 103;
    pub const ARROW_RIGHT: i32 = 104;
    pub const MARKER: i32 = 105;
}

/// The pages by number.
pub mod page {
    pub const SEX: usize = 0;
    pub const RACE: usize = 1;
    pub const FACE: usize = 2;
    pub const HAIR: usize = 3;
    pub const CUSTOMIZE: usize = 4;
    pub const HAIR_STYLE: usize = 5;
    pub const HAIR_COLOR: usize = 6;
    pub const FACIAL_HAIR: usize = 7;
    pub const EYE_COLOR: usize = 8;
    pub const SHAPE: usize = 9;
    pub const GENERAL: usize = 10;
    pub const FOREHEAD: usize = 11;
    pub const BROW: usize = 12;
    pub const EYES: usize = 13;
    pub const NOSE: usize = 14;
    pub const MOUTH: usize = 15;
    pub const CHEEKS: usize = 16;
    pub const JAW: usize = 17;
    pub const CHIN: usize = 18;
    pub const TONE: usize = 19;
}

/// What an item does (`+4`); below 20 it opens that page.
pub mod kind {
    pub const MALE: i32 = 0x15;
    pub const FEMALE: i32 = 0x16;
    pub const RACE: i32 = 0x17;
    pub const PRESET: i32 = 0x18;
    pub const RANDOMIZE: i32 = 0x19;
    pub const HAIR: i32 = 0x1a;
    pub const HEAD_PART: i32 = 0x1b;
    pub const HAIR_PRESET: i32 = 0x1c;
    pub const RED: i32 = 0x1d;
    pub const GREEN: i32 = 0x1e;
    pub const BLUE: i32 = 0x1f;
    pub const EYES: i32 = 0x20;
    pub const SHAPE: i32 = 0x21;
    pub const TONE: i32 = 0x22;
}

/// A shape slider's range (`0066e7b0`).
#[derive(Debug, Clone, Copy, PartialEq)]
enum Range {
    /// `fRSMFaceSliderDefaultMin` to `Max`.
    Default,
    /// -2 to 2.
    Narrow,
    /// The default minimum to 3.
    ToThree,
}

/// The shape sliders (`0066e7b0`, its table on the stack, read from the
/// instructions by `tools/rsm_table.py`): control, page, label. Other
/// controls have none.
const SHAPE_SLIDERS: [(u32, usize, &str, Range); 31] = [
    (0, page::BROW, "sRSMShapeOption01", Range::Default),
    (1, page::BROW, "sRSMShapeOption02", Range::Default),
    (2, page::BROW, "sRSMShapeOption03", Range::Default),
    (3, page::CHEEKS, "sRSMShapeOption04", Range::Default),
    (4, page::CHEEKS, "sRSMShapeOption05", Range::Default),
    (6, page::CHEEKS, "sRSMShapeOption07", Range::Default),
    (8, page::CHIN, "sRSMShapeOption09", Range::Default),
    (9, page::CHIN, "sRSMShapeOption10", Range::Default),
    (11, page::CHIN, "sRSMShapeOption12", Range::Default),
    (13, page::CHIN, "sRSMShapeOption14", Range::Default),
    (15, page::EYES, "sRSMShapeOption16", Range::Default),
    (16, page::EYES, "sRSMShapeOption13", Range::Default),
    (18, page::EYES, "sRSMShapeOption19", Range::Default),
    (21, page::GENERAL, "sRSMShapeOption22", Range::Narrow),
    (23, page::GENERAL, "sRSMShapeOption06", Range::Narrow),
    (24, page::FOREHEAD, "sRSMShapeOption13", Range::Default),
    (25, page::FOREHEAD, "sRSMShapeOption14", Range::Default),
    (26, page::FOREHEAD, "sRSMShapeOption27", Range::Default),
    (27, page::JAW, "sRSMShapeOption11", Range::Default),
    (28, page::JAW, "sRSMShapeOption15", Range::Default),
    (29, page::JAW, "sRSMShapeOption30", Range::Default),
    (30, page::JAW, "sRSMShapeOption07", Range::Default),
    (35, page::MOUTH, "sRSMShapeOption36", Range::Default),
    (39, page::MOUTH, "sRSMShapeOption40", Range::Default),
    (41, page::NOSE, "sRSMShapeOption42", Range::Default),
    (43, page::NOSE, "sRSMShapeOption16", Range::Default),
    (44, page::NOSE, "sRSMShapeOption45", Range::Default),
    (45, page::NOSE, "sRSMShapeOption46", Range::Default),
    (47, page::NOSE, "sRSMShapeOption48", Range::Default),
    (49, page::NOSE, "sRSMShapeOption50", Range::ToThree),
    (53, page::NOSE, "sRSMShapeOption54", Range::ToThree),
];

/// The tone sliders, in order (`0066e7b0`): texture control, label.
const TONE_SLIDERS: [(u32, &str); 12] = [
    (29, "sRSMToneOption24"),
    (28, "sRSMToneOption01"),
    (30, "sRSMToneOption25"),
    (31, "sRSMToneOption26"),
    (8, "sRSMEyeSockets"),
    (9, "sEyebrows"),
    (18, "sEyeliner"),
    (27, "sNose"),
    (22, "sLips"),
    (5, "sRSMToneOption06"),
    (6, "sCheeks"),
    (3, "sBeard"),
];

/// The hair colours' presets (`iHairColor00` to `15`, `R | G << 8 | B << 16`:
/// the exe's defaults, none is in `Fallout3.esm`).
pub const HAIR_COLORS: [u32; 16] = [
    8289918, 3618617, 2039585, 4289146, 4680837, 3557724, 3360589, 2830157, 2965094, 4287105,
    1000298, 1974080, 2958114, 2239537, 7237230, 5400188,
];

/// A setting's text as the exe holds it, for the ones `Fallout3.esm` doesn't
/// override: all of these (`sHairColorN` are in the data).
const DEFAULT_TEXT: [(&str, &str); 67] = [
    ("sBack", "Back"),
    ("sBeard", "Beard"),
    ("sBrow", "Brow"),
    ("sCheeks", "Cheeks"),
    ("sChin", "Chin"),
    ("sDone", "Done"),
    ("sEyebrows", "Eyebrows"),
    ("sEyeliner", "Eyeliner"),
    ("sEyes", "Eyes"),
    ("sFemale", "Female"),
    ("sForehead", "Forehead"),
    ("sJaw", "Jaw"),
    ("sLips", "Lips"),
    ("sMale", "Male"),
    ("sMouth", "Mouth"),
    ("sNext", "Next"),
    ("sNo", "No"),
    ("sNose", "Nose"),
    ("sShape", "Shape"),
    ("sTone", "Tone"),
    ("sYes", "Yes"),
    ("sRSMBlueAbbrev", "B:"),
    ("sRSMCharacter", "character"),
    ("sRSMConfirmDone", "Are you sure you want this to be your"),
    ("sRSMConfirmRandomize", "Randomize your head?"),
    ("sRSMCustom", "Custom"),
    ("sRSMCustomize", "Customize"),
    ("sRSMEyeColor", "Eye Color"),
    ("sRSMEyeSockets", "Eye Sockets"),
    ("sRSMFace", "Face"),
    ("sRSMFacialHair", "Facial Hair"),
    ("sRSMGeneral", "General"),
    ("sRSMGreenAbbrev", "G:"),
    ("sRSMHair", "Hair"),
    ("sRSMHairColor", "Hair Color"),
    ("sRSMHairStyle", "Hair Style"),
    ("sRSMPreset", "Preset:"),
    ("sRSMRace", "Race"),
    ("sRSMRandomize", "Randomize"),
    ("sRSMRedAbbrev", "R:"),
    ("sRSMSex", "Sex"),
    ("sRSMShapeOption01", "High/Low"),
    ("sRSMShapeOption02", "Up/Down (inner)"),
    ("sRSMShapeOption03", "Up/Down (outer)"),
    ("sRSMShapeOption04", "Low/High"),
    ("sRSMShapeOption05", "Shallow/Pronounced"),
    ("sRSMShapeOption06", "Thin/Wide"),
    ("sRSMShapeOption07", "Concave/Convex"),
    ("sRSMShapeOption09", "Forward/Backward"),
    ("sRSMShapeOption10", "Broad/Recessed"),
    ("sRSMShapeOption11", "Retracted/Jutting"),
    ("sRSMShapeOption12", "Shallow/Deep"),
    ("sRSMShapeOption13", "Small/Large"),
    ("sRSMShapeOption14", "Tall/Short"),
    ("sRSMShapeOption15", "Wide/Thin"),
    ("sRSMShapeOption16", "Down/Up"),
    ("sRSMShapeOption19", "Together/Apart"),
    ("sRSMShapeOption22", "Heavy/Light"),
    ("sRSMShapeOption27", "Tilt Forward/Back"),
    ("sRSMShapeOption30", "Slope High/Low"),
    ("sRSMShapeOption36", "Large/Small"),
    ("sRSMShapeOption40", "Underbite/Overbite"),
    ("sRSMShapeOption42", "Bridge Depth"),
    ("sRSMShapeOption45", "Flat/Pointed"),
    ("sRSMShapeOption46", "Nostril Tilt"),
    ("sRSMShapeOption48", "Nostrils Wide/Thin"),
    ("sRSMShapeOption50", "Sellion Height"),
];

/// The tone labels and the rest of [`DEFAULT_TEXT`].
const MORE_TEXT: [(&str, &str); 6] = [
    ("sRSMShapeOption54", "Short/Long"),
    ("sRSMToneOption01", "Flushed"),
    ("sRSMToneOption06", "Moustache"),
    ("sRSMToneOption24", "Shade"),
    ("sRSMToneOption25", "Blue Tint"),
    ("sRSMToneOption26", "Yellow Tint"),
];

/// A setting's text: the data's, else the exe's own default.
pub fn text(ui: &Ui, name: &str) -> String {
    ui.setting_text(name)
        .or_else(|| {
            DEFAULT_TEXT
                .iter()
                .chain(MORE_TEXT.iter())
                .find(|(n, _)| *n == name)
                .map(|(_, v)| v.to_string())
        })
        .unwrap_or_default()
}

/// How the menu is set up for each way of opening it (`0066ffc0`): the model,
/// the head's place with the zoom out and in (x, y, and z as a share of the
/// player's height), and the last page.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mode {
    pub model: &'static str,
    pub near: [f32; 3],
    pub far: [f32; 3],
    pub last_page: usize,
}

/// A way of opening it: 0 from the console, 1 from a script (starting on
/// Race rather than Sex), 2 the barber, 3 the plastic surgeon.
pub fn mode(number: u8) -> Mode {
    match number {
        2 => Mode {
            model: "meshes\\terminals\\barberinterface01.nif",
            near: [-7.4, 80.0, -0.84],
            far: [-12.4, 135.0, -0.8],
            last_page: page::HAIR,
        },
        3 => Mode {
            model: "meshes\\terminals\\plasticsurgeryinterface01.nif",
            near: [-5.2, 55.0, -0.84],
            far: [-12.4, 135.0, -0.77],
            last_page: page::FACE,
        },
        _ => Mode {
            model: "meshes\\terminals\\racesexinterface01.nif",
            near: [-4.7, 55.0, -0.84],
            far: [-11.4, 135.0, -0.8],
            last_page: page::HAIR,
        },
    }
}

/// The page a mode starts on (`0066ffc0`).
fn first_page(number: u8) -> usize {
    match number {
        1 => page::RACE,
        2 => page::HAIR,
        3 => page::FACE,
        _ => page::SEX,
    }
}

/// A race, hairstyle, eye colour or head part.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Choice {
    pub form: u32,
    pub name: String,
}

/// The game's side of the menu, as it stands.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Snapshot {
    pub female: bool,
    pub race: u32,
    pub races: Vec<Choice>,
    pub hair: Option<u32>,
    pub hairs: Vec<Choice>,
    pub eyes: Option<u32>,
    pub eye_colors: Vec<Choice>,
    pub head_part: Option<u32>,
    pub head_parts: Vec<Choice>,
    pub hair_color: [u8; 3],
    /// How many preset faces there are for the race and sex.
    pub presets: i32,
    /// Symmetric controls' values, shape and texture, by control.
    pub shape: Vec<f32>,
    pub tone: Vec<f32>,
    /// `fRSMFaceSliderDefaultMin` and `Max`.
    pub slider_min: f32,
    pub slider_max: f32,
}

/// What the menu asks of the game.
#[derive(Debug, Clone, PartialEq)]
pub enum Request {
    /// Male or female (`0044f750`).
    Sex { female: bool },
    Race(u32),
    /// A preset face by its number from 0 (`005579a0`).
    Preset(i32),
    /// Ask "Randomize your head?", then [`Request::Randomize`].
    ConfirmRandomize { text: String, no: String, yes: String },
    Randomize,
    Hair(u32),
    /// A head part, or none when the one chosen is clicked again.
    HeadPart(Option<u32>),
    Eyes(u32),
    HairColor([u8; 3]),
    /// A slider's new value (`0066d560`).
    Control { tone: bool, control: u32, value: f32 },
    /// Done: ask "Are you sure...", then [`RaceSexMenu::finish`].
    ConfirmDone { text: String, no: String, yes: String },
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Item {
    tile: TileId,
    kind: i32,
    /// The form, or the control (`+8`).
    value: u32,
    /// A slider whose display isn't its number (`user5` holds text).
    text_display: bool,
}

#[derive(Debug, Clone, Default)]
struct Page {
    items: Vec<Item>,
    /// `+0xc`, `+0x10`.
    back: usize,
    next: usize,
    /// The tallest item, what one scroll moves (`+0x18`).
    step: f32,
}

/// The face menu.
#[derive(Debug)]
pub struct RaceSexMenu {
    pub menu: TileId,
    tiles: [Option<TileId>; 6],
    pages: Vec<Page>,
    pub mode: u8,
    last_page: usize,
    current: usize,
    /// Hair and eye lists need filling when next opened (`+0xec` bits 0, 1).
    refill_hair: bool,
    refill_eyes: bool,
    /// Randomized since the last change, so it doesn't ask again (bit 7).
    randomized: bool,
    snapshot: Snapshot,
    /// The slider part and slider being worked (`+0xc0`, `+0xc4`).
    held: Option<(TileId, usize)>,
    /// A drag's start: the pointer and the slider's value (`+0xb8`, `+0xbc`).
    drag_from: f32,
    drag_value: i32,
    /// What the head shows: its turn in radians, zoom now and wanted, the
    /// zoom's speed (`+0xd4`'s rotation, `+0xd0`, `+0xc8`, `+0xcc`).
    pub heading: f32,
    pub zoom: f32,
    zoom_target: f32,
    zoom_speed: f32,
    pub requests: Vec<Request>,
    pub closed: bool,
    pub sounds: Vec<String>,
}

fn custom(ui: &mut Ui, name: &str) -> i32 {
    ui.names.lookup_or_add(name).unwrap_or(0)
}

const USER1: i32 = t::USER0 + 1;
const USER2: i32 = t::USER0 + 2;
const USER3: i32 = t::USER0 + 3;
const USER4: i32 = t::USER0 + 4;
const USER5: i32 = t::USER0 + 5;
const USER6: i32 = t::USER0 + 6;

/// The opening's turn from straight on (`0069bde0`: `0066b230(0.5236, 1)`,
/// from the model turned by pi to face the camera).
const OPENING_TURN: f32 = 0.5236;
/// How the zoom speeds up, per millisecond squared (`006711e0`).
const ZOOM_ACCELERATION: f32 = 3.125e-6;
/// A wheel notch's change of the wanted zoom (`006711e0`).
const WHEEL_ZOOM: f32 = 0.125;

impl RaceSexMenu {
    /// Before its file is read.
    pub fn new(menu: TileId, mode: u8) -> RaceSexMenu {
        RaceSexMenu {
            menu,
            tiles: [None; 6],
            pages: Vec::new(),
            mode,
            last_page: self::mode(mode).last_page,
            current: first_page(mode),
            refill_hair: true,
            refill_eyes: true,
            randomized: false,
            snapshot: Snapshot::default(),
            held: None,
            drag_from: 0.0,
            drag_value: 0,
            heading: std::f32::consts::PI - OPENING_TURN,
            zoom: 0.0,
            zoom_target: 0.0,
            zoom_speed: 0.0,
            requests: Vec::new(),
            closed: false,
            sounds: Vec::new(),
        }
    }

    /// The page on show.
    pub fn page(&self) -> usize {
        self.current
    }

    /// Whether the menu has its tiles (`RSM_*`, ids 0 to 5).
    fn has_tiles(&self) -> bool {
        self.tiles.iter().all(Option::is_some)
    }

    fn sound(&mut self, name: &str) {
        self.sounds.push(name.to_string());
    }

    /// An item added to a page (`0066c330`).
    fn add(
        &mut self,
        ui: &mut Ui,
        at: usize,
        label: &str,
        template: &str,
        kind: i32,
    ) -> Option<usize> {
        let parent = self.tiles[id::BACKGROUND as usize]?;
        let tile = ui.instantiate(self.menu, parent, template)?;
        ui.set_text(tile, t::STRING, label);
        ui.set_number(tile, t::USER0, at as f32);
        ui.set_number(tile, t::TARGET, 1.0);
        let top = custom(ui, "_top_bound");
        let y = match self.pages[at].items.last() {
            None => ui.number(parent, top),
            Some(last) => ui.number(last.tile, t::Y) + ui.number(last.tile, t::HEIGHT),
        };
        ui.set_number(tile, t::Y, y);
        let height = ui.number(tile, t::HEIGHT);
        let p = &mut self.pages[at];
        p.step = p.step.max(height);
        p.items.push(Item {
            tile,
            kind,
            value: 0,
            text_display: false,
        });
        Some(p.items.len() - 1)
    }

    /// A link to another page (`0066c7c0`): "label >".
    fn link(&mut self, ui: &mut Ui, at: usize, to: usize, label: &str) {
        self.pages[to].back = at;
        self.add(
            ui,
            at,
            &format!("{label} >"),
            "RSM_list_item_template",
            to as i32,
        );
    }

    /// A slider (`0066c8c0`): its range, value and jump.
    #[allow(clippy::too_many_arguments)]
    fn slider(
        &mut self,
        ui: &mut Ui,
        at: usize,
        label: &str,
        kind: i32,
        low: i32,
        high: i32,
        value: i32,
        jump: f32,
    ) -> Option<usize> {
        let i = self.add(ui, at, label, "RSM_slider_option_template", kind)?;
        let tile = self.pages[at].items[i].tile;
        ui.set_number(tile, USER2, low as f32);
        ui.set_number(tile, USER3, high as f32);
        ui.set_number(tile, USER1, low as f32 - 1.0);
        self.set_value(ui, at, i, value);
        ui.set_number(tile, USER4, (((high - low + 1) as f32) * jump).round());
        Some(i)
    }

    /// Sets a slider's value within its range (`0066b4c0`); true if it moved.
    fn set_value(&mut self, ui: &mut Ui, at: usize, i: usize, value: i32) -> bool {
        let item = self.pages[at].items[i];
        let low = ui.number(item.tile, USER2) as i32;
        let high = ui.number(item.tile, USER3) as i32;
        let value = value.max(low).min(high);
        let now = ui.number(item.tile, USER1).round() as i32;
        if now == value {
            return false;
        }
        ui.set_number(item.tile, USER1, value as f32);
        if !item.text_display {
            ui.set_number(item.tile, USER5, value as f32);
        }
        true
    }

    fn value(&self, ui: &mut Ui, at: usize, i: usize) -> i32 {
        ui.number(self.pages[at].items[i].tile, USER1).round() as i32
    }

    fn find_kind(&self, at: usize, kind: i32) -> Option<usize> {
        self.pages[at].items.iter().position(|i| i.kind == kind)
    }

    /// Everything but the lists filled when opened (`0066ffc0`, `0066e7b0`);
    /// false without the menu's tiles.
    pub fn open(&mut self, ui: &mut Ui, snapshot: Snapshot) -> bool {
        if !self.has_tiles() {
            return false;
        }
        self.pages = vec![
            Page {
                back: NO_PAGE,
                next: NO_PAGE,
                ..Page::default()
            };
            NO_PAGE
        ];
        ui.set_string(self.menu, USER1, "<");
        ui.set_string(self.menu, USER2, ">");
        // The back button's label stays; the next's follows the page.
        let label = text(ui, "sBack").to_uppercase();
        if let Some(back) = self.tiles[id::BACK as usize] {
            ui.set_string(back, t::STRING, &label);
        }

        // The top pages' headers are numbered in the opening.
        let names = [
            "sRSMSex",
            "sRSMRace",
            "sRSMFace",
            "sRSMHair",
            "sRSMCustomize",
            "sRSMHairStyle",
            "sRSMHairColor",
            "sRSMFacialHair",
            "sRSMEyeColor",
            "sShape",
            "sRSMGeneral",
            "sForehead",
            "sBrow",
            "sEyes",
            "sNose",
            "sMouth",
            "sCheeks",
            "sJaw",
            "sChin",
            "sTone",
        ];
        let parent = self.tiles[id::BACKGROUND as usize];
        for (index, name) in names.iter().enumerate() {
            let mut label = text(ui, name);
            if index < 4 && !(2..=3).contains(&self.mode) {
                label = format!("{}. {label}", index + 1);
            }
            // The header (`0066c6d0`): not clickable, shown whatever its
            // place in the list.
            if let Some(header) =
                parent.and_then(|p| ui.instantiate(self.menu, p, "RSM_list_item_template"))
            {
                ui.set_number(header, t::USER0, index as f32);
                ui.set_text(header, t::STRING, &label);
                ui.set_number(header, t::TARGET, 0.0);
                let is_header = custom(ui, "_is_header");
                ui.set_number(header, is_header, 1.0);
            }
        }

        // Sex, Race, Face, Hair.
        self.add(
            ui,
            page::SEX,
            &text(ui, "sMale"),
            "RSM_list_item_template",
            kind::MALE,
        );
        self.add(
            ui,
            page::SEX,
            &text(ui, "sFemale"),
            "RSM_list_item_template",
            kind::FEMALE,
        );
        for race in &snapshot.races {
            if let Some(i) = self.add(
                ui,
                page::RACE,
                &race.name,
                "RSM_list_item_template",
                kind::RACE,
            ) {
                self.pages[page::RACE].items[i].value = race.form;
            }
        }
        // The value starts out of range, which clamps to the lowest
        // (`0x80000001`).
        self.slider(
            ui,
            page::FACE,
            &text(ui, "sRSMPreset"),
            kind::PRESET,
            1,
            20,
            i32::MIN + 1,
            0.25,
        );
        self.link(ui, page::FACE, page::CUSTOMIZE, &text(ui, "sRSMCustomize"));
        self.add(
            ui,
            page::FACE,
            &text(ui, "sRSMRandomize"),
            "RSM_list_item_template",
            kind::RANDOMIZE,
        );
        self.link(ui, page::HAIR, page::HAIR_STYLE, &text(ui, "sRSMHairStyle"));
        self.link(ui, page::HAIR, page::HAIR_COLOR, &text(ui, "sRSMHairColor"));
        // The male's only: `0066cb10` keeps this right when the page opens.
        self.pages[page::FACIAL_HAIR].back = page::HAIR;
        if !snapshot.female {
            self.link(
                ui,
                page::HAIR,
                page::FACIAL_HAIR,
                &text(ui, "sRSMFacialHair"),
            );
        }
        self.link(ui, page::CUSTOMIZE, page::SHAPE, &text(ui, "sShape"));
        self.link(ui, page::CUSTOMIZE, page::TONE, &text(ui, "sTone"));
        self.link(ui, page::CUSTOMIZE, page::EYE_COLOR, &text(ui, "sRSMEyeColor"));

        // The hair colour page.
        let [red, green, blue] = snapshot.hair_color;
        let custom_text = text(ui, "sRSMCustom");
        if let Some(i) = self.slider(
            ui,
            page::HAIR_COLOR,
            &text(ui, "sRSMPreset"),
            kind::HAIR_PRESET,
            0,
            15,
            i32::MIN + 1,
            0.25,
        ) {
            let item = &mut self.pages[page::HAIR_COLOR].items[i];
            item.text_display = true;
            let tile = item.tile;
            ui.set_string(tile, USER5, &custom_text);
        }
        for (name, kind, value) in [
            ("sRSMRedAbbrev", kind::RED, red),
            ("sRSMGreenAbbrev", kind::GREEN, green),
            ("sRSMBlueAbbrev", kind::BLUE, blue),
        ] {
            self.slider(
                ui,
                page::HAIR_COLOR,
                &text(ui, name),
                kind,
                0,
                255,
                i32::from(value),
                0.125,
            );
        }

        // Facial hair, and the shape page's links.
        for part in &snapshot.head_parts {
            if let Some(i) = self.add(
                ui,
                page::FACIAL_HAIR,
                &part.name,
                "RSM_list_item_template",
                kind::HEAD_PART,
            ) {
                self.pages[page::FACIAL_HAIR].items[i].value = part.form;
            }
        }
        for (to, name) in [
            (page::GENERAL, "sRSMGeneral"),
            (page::FOREHEAD, "sForehead"),
            (page::BROW, "sBrow"),
            (page::EYES, "sEyes"),
            (page::NOSE, "sNose"),
            (page::MOUTH, "sMouth"),
            (page::CHEEKS, "sCheeks"),
            (page::JAW, "sJaw"),
            (page::CHIN, "sChin"),
        ] {
            self.link(ui, page::SHAPE, to, &text(ui, name));
        }

        // The sliders (`0066e7b0`): shown as ten times the control's value,
        // with no number beside them.
        let (min, max) = (snapshot.slider_min, snapshot.slider_max);
        let tens = |v: f32| (v * 10.0).round() as i32;
        for (control, at, label, range) in SHAPE_SLIDERS {
            let (low, high) = match range {
                Range::Default => (min, max),
                Range::Narrow => (-2.0, 2.0),
                Range::ToThree => (min, 3.0),
            };
            let value = snapshot.shape.get(control as usize).copied().unwrap_or(0.0);
            let label = text(ui, label);
            self.face_slider(ui, at, &label, kind::SHAPE, control, tens(low), tens(high), tens(value));
        }
        for (control, label) in TONE_SLIDERS {
            let value = snapshot.tone.get(control as usize).copied().unwrap_or(0.0);
            let label = text(ui, label);
            self.face_slider(
                ui,
                page::TONE,
                &label,
                kind::TONE,
                control,
                tens(min),
                tens(max),
                tens(value),
            );
        }

        // The top pages lead on to each other (`0066b050`).
        if !(2..=3).contains(&self.mode) {
            for (from, to) in [
                (page::SEX, page::RACE),
                (page::RACE, page::FACE),
                (page::FACE, page::HAIR),
            ] {
                self.pages[from].next = to;
                self.pages[to].back = from;
            }
        }

        self.snapshot = snapshot.clone();
        self.set_presets(ui, snapshot.presets);
        self.select(ui);
        ui.set_number(self.menu, menu::LEAVE_STACK, 0.0);
        ui.set_number(self.menu, t::VISIBLE, 1.0);
        self.open_page(ui, self.current);
        ui.refresh();
        true
    }

    /// A slider of the face (`0066e7b0`): nothing beside it, " ".
    #[allow(clippy::too_many_arguments)]
    fn face_slider(
        &mut self,
        ui: &mut Ui,
        at: usize,
        label: &str,
        kind: i32,
        control: u32,
        low: i32,
        high: i32,
        value: i32,
    ) {
        if let Some(i) = self.slider(ui, at, label, kind, low, high, value, 0.25) {
            let item = &mut self.pages[at].items[i];
            item.value = control;
            item.text_display = true;
            let tile = item.tile;
            ui.set_string(tile, USER5, " ");
        }
    }

    /// The zoom to start from (`fRSMStartingZoom`).
    pub fn set_starting_zoom(&mut self, zoom: f32) {
        self.zoom = zoom.clamp(0.0, 1.0);
        self.zoom_target = self.zoom;
    }

    /// The preset slider's range, off without any (`0066b9b0`).
    fn set_presets(&mut self, ui: &mut Ui, count: i32) {
        let Some(i) = self.find_kind(page::FACE, kind::PRESET) else {
            return;
        };
        let tile = self.pages[page::FACE].items[i].tile;
        let enabled = custom(ui, "_enabled");
        ui.set_number(tile, enabled, f32::from(count != 0));
        if count != 0 {
            ui.set_number(tile, USER2, 1.0);
            ui.set_number(tile, USER3, count as f32);
        }
    }

    /// `_selected` on the pages that have one chosen (`0066b600`).
    fn select(&mut self, ui: &mut Ui) {
        let selected = custom(ui, "_selected");
        let s = &self.snapshot;
        for (at, p) in self.pages.iter().enumerate() {
            for item in &p.items {
                let on = match at {
                    page::SEX => match item.kind {
                        kind::MALE => !s.female,
                        kind::FEMALE => s.female,
                        _ => continue,
                    },
                    page::RACE => item.value == s.race,
                    page::HAIR_STYLE => Some(item.value) == s.hair,
                    page::FACIAL_HAIR => Some(item.value) == s.head_part,
                    page::EYE_COLOR => Some(item.value) == s.eyes,
                    _ => continue,
                };
                ui.set_number(item.tile, selected, if on { 2.0 } else { 1.0 });
            }
        }
    }

    /// Opens a page (`0066ce10`); false past the last.
    pub fn open_page(&mut self, ui: &mut Ui, to: usize) -> bool {
        if to >= NO_PAGE {
            return false;
        }
        self.current = to;
        ui.set_number(self.menu, t::USER0, to as f32);
        match to {
            page::HAIR_STYLE if self.refill_hair => self.fill_hair(ui),
            page::EYE_COLOR if self.refill_eyes => self.fill_eyes(ui),
            page::HAIR => self.facial_hair_link(ui),
            _ => {}
        }
        self.arrows(ui);
        let p = &self.pages[to];
        let (back, next) = (p.back, p.next);
        if let Some(tile) = self.tiles[id::BACK as usize] {
            ui.set_number(tile, t::VISIBLE, f32::from(back < NO_PAGE));
        }
        if let Some(tile) = self.tiles[id::NEXT as usize] {
            ui.set_number(
                tile,
                t::VISIBLE,
                f32::from(next < NO_PAGE || to == self.last_page),
            );
            let name = if to == self.last_page { "sDone" } else { "sNext" };
            let label = text(ui, name).to_uppercase();
            ui.set_string(tile, t::STRING, &label);
        }
        true
    }

    /// Empties a page's list (`0066bc70`).
    fn empty(&mut self, ui: &mut Ui, at: usize) {
        for item in std::mem::take(&mut self.pages[at].items) {
            ui.remove(item.tile);
        }
    }

    /// The hair styles for the race and sex (`0066c9d0`), brought into view.
    fn fill_hair(&mut self, ui: &mut Ui) {
        self.empty(ui, page::HAIR_STYLE);
        for hair in self.snapshot.hairs.clone() {
            if let Some(i) = self.add(
                ui,
                page::HAIR_STYLE,
                &hair.name,
                "RSM_list_item_template",
                kind::HAIR,
            ) {
                self.pages[page::HAIR_STYLE].items[i].value = hair.form;
            }
        }
        self.select(ui);
        self.refill_hair = false;
        let chosen = self.pages[page::HAIR_STYLE]
            .items
            .iter()
            .find(|i| Some(i.value) == self.snapshot.hair)
            .map(|i| i.tile);
        if let Some(chosen) = chosen {
            let position = custom(ui, "_position");
            for _ in 0..100 {
                if ui.number(chosen, position) as i32 != 1 {
                    break;
                }
                self.scroll(ui, page::HAIR_STYLE, -1.0);
            }
        }
    }

    /// The eye colours for the race and sex (`0066ca90`).
    fn fill_eyes(&mut self, ui: &mut Ui) {
        self.empty(ui, page::EYE_COLOR);
        for eyes in self.snapshot.eye_colors.clone() {
            if let Some(i) = self.add(
                ui,
                page::EYE_COLOR,
                &eyes.name,
                "RSM_list_item_template",
                kind::EYES,
            ) {
                self.pages[page::EYE_COLOR].items[i].value = eyes.form;
            }
        }
        self.select(ui);
        self.refill_eyes = false;
    }

    /// Facial Hair is on the Hair page only for a male (`0066cb10`).
    fn facial_hair_link(&mut self, ui: &mut Ui) {
        let at = self.find_kind(page::HAIR, page::FACIAL_HAIR as i32);
        match (at, self.snapshot.female) {
            (None, false) => {
                let label = text(ui, "sRSMFacialHair");
                self.link(ui, page::HAIR, page::FACIAL_HAIR, &label);
            }
            (Some(i), true) => {
                let item = self.pages[page::HAIR].items.remove(i);
                ui.remove(item.tile);
            }
            _ => {}
        }
    }

    /// Moves a page's list one item (`0066b7b0`).
    fn scroll(&mut self, ui: &mut Ui, at: usize, direction: f32) {
        let p = &self.pages[at];
        if p.step <= 0.0 {
            return;
        }
        let step = p.step;
        for item in p.items.clone() {
            let y = ui.number(item.tile, t::Y);
            ui.set_number(item.tile, t::Y, y + direction * step);
        }
        ui.refresh();
    }

    /// The scroll arrows show when something's past that end (`0066b140`).
    fn arrows(&mut self, ui: &mut Ui) {
        let position = custom(ui, "_position");
        let (mut above, mut below) = (false, false);
        for item in self.pages[self.current].items.clone() {
            match ui.number(item.tile, position) as i32 {
                -1 => above = true,
                1 => below = true,
                _ => {}
            }
        }
        if let Some(tile) = self.tiles[id::SCROLL_UP as usize] {
            ui.set_number(tile, t::VISIBLE, f32::from(above));
        }
        if let Some(tile) = self.tiles[id::SCROLL_DOWN as usize] {
            ui.set_number(tile, t::VISIBLE, f32::from(below));
        }
    }

    /// The game's side, after a change: choices marked, sliders moved (all
    /// but the one in hand, `0066d6c0`, which only shows values within its
    /// range) and lists filled again when next opened.
    pub fn sync(&mut self, ui: &mut Ui, snapshot: Snapshot) {
        self.refill_hair |= snapshot.hairs != self.snapshot.hairs;
        self.refill_eyes |= snapshot.eye_colors != self.snapshot.eye_colors;
        // Another race or sex starts on the first preset (`0066ff30`).
        let other = snapshot.race != self.snapshot.race || snapshot.female != self.snapshot.female;
        if snapshot.presets != self.snapshot.presets || other {
            self.set_presets(ui, snapshot.presets);
        }
        if other {
            if let Some(i) = self.find_kind(page::FACE, kind::PRESET) {
                self.set_value(ui, page::FACE, i, 1);
            }
        }
        self.snapshot = snapshot;
        self.select(ui);
        let held = self.held.map(|(_, item)| item);
        for at in 0..self.pages.len() {
            for i in 0..self.pages[at].items.len() {
                let item = self.pages[at].items[i];
                let values = match item.kind {
                    kind::SHAPE => &self.snapshot.shape,
                    kind::TONE => &self.snapshot.tone,
                    _ => continue,
                };
                if at == self.current && held == Some(i) {
                    continue;
                }
                let value = values.get(item.value as usize).copied().unwrap_or(0.0);
                let value = (value * 10.0).round() as i32;
                self.set_value(ui, at, i, value);
            }
        }
        let rgb = self.snapshot.hair_color;
        for (kind_of, value) in [
            (kind::RED, rgb[0]),
            (kind::GREEN, rgb[1]),
            (kind::BLUE, rgb[2]),
        ] {
            if let Some(i) = self.find_kind(page::HAIR_COLOR, kind_of) {
                if self.current != page::HAIR_COLOR || held != Some(i) {
                    self.set_value(ui, page::HAIR_COLOR, i, i32::from(value));
                }
            }
        }
        ui.refresh();
    }

    /// A slider's new value, carried out (`0066fce0`).
    fn apply(&mut self, ui: &mut Ui, i: usize) {
        let at = self.current;
        let item = self.pages[at].items[i];
        let value = self.value(ui, at, i);
        if at == page::HAIR_COLOR {
            self.apply_hair_color(ui, i);
            self.randomized = false;
            return;
        }
        match item.kind {
            kind::SHAPE | kind::TONE => {
                self.requests.push(Request::Control {
                    tone: item.kind == kind::TONE,
                    control: item.value,
                    value: value as f32 / 10.0,
                });
                self.randomized = false;
            }
            kind::PRESET => {
                self.requests.push(Request::Preset(value - 1));
                self.refill_hair = true;
                self.randomized = false;
            }
            _ => {}
        }
    }

    /// The hair colour page (`0066e610`): a preset gives its name and colour
    /// (`iHairColorNN`, `sHairColorN`); otherwise the preset shows Custom and
    /// the colour is the sliders' (`0066e480`).
    fn apply_hair_color(&mut self, ui: &mut Ui, i: usize) {
        let at = page::HAIR_COLOR;
        let item = self.pages[at].items[i];
        if item.kind == kind::HAIR_PRESET {
            let n = self.value(ui, at, i);
            let name = ui.setting_text(&format!("sHairColor{n}"));
            if let Some(name) = name {
                ui.set_string(item.tile, USER5, &name);
            }
            let color = HAIR_COLORS.get(n as usize).copied().unwrap_or(0);
            for (kind_of, shift) in [(kind::RED, 0), (kind::GREEN, 8), (kind::BLUE, 16)] {
                if let Some(s) = self.find_kind(at, kind_of) {
                    self.set_value(ui, at, s, ((color >> shift) & 0xff) as i32);
                }
            }
        } else if let Some(s) = self.find_kind(at, kind::HAIR_PRESET) {
            let label = text(ui, "sRSMCustom");
            let tile = self.pages[at].items[s].tile;
            ui.set_string(tile, USER5, &label);
        }
        let mut rgb = [255u8; 3];
        for (n, kind_of) in [kind::RED, kind::GREEN, kind::BLUE].into_iter().enumerate() {
            if let Some(s) = self.find_kind(at, kind_of) {
                rgb[n] = self.value(ui, at, s).clamp(0, 255) as u8;
            }
        }
        self.requests.push(Request::HairColor(rgb));
    }

    /// A slider's arrows and bar (`00670a30`).
    fn slider_click(&mut self, ui: &mut Ui, id_of_part: i32, i: usize) {
        let at = self.current;
        let item = self.pages[at].items[i];
        // The hair colour preset's arrows from Custom show the preset it
        // stands on without moving (`00670a30`).
        let custom_text = text(ui, "sRSMCustom");
        let from_custom = item.kind == kind::HAIR_PRESET
            && matches!(id_of_part, id::ARROW_LEFT | id::ARROW_RIGHT)
            && ui.string(item.tile, USER5).as_deref() == Some(custom_text.as_str());
        if from_custom {
            self.apply(ui, i);
            self.sound("UIMenuPrevNext");
            return;
        }
        let now = self.value(ui, at, i);
        let increment = ui.number(item.tile, USER6);
        let increment = if increment <= 1.0 {
            1
        } else {
            increment.round() as i32
        };
        let jump = ui.number(item.tile, USER4) as i32;
        let moved = match id_of_part {
            id::ARROW_LEFT => self.set_value(ui, at, i, now - increment),
            id::ARROW_RIGHT => self.set_value(ui, at, i, now + increment),
            id::BAR_LEFT => self.set_value(ui, at, i, now - jump),
            id::BAR_RIGHT => self.set_value(ui, at, i, now + jump),
            _ => false,
        };
        if moved {
            self.apply(ui, i);
        }
    }

    /// The item with this tile on the page on show.
    fn item_of(&self, tile: TileId) -> Option<usize> {
        self.pages[self.current]
            .items
            .iter()
            .position(|i| i.tile == tile)
    }

    /// The pointer pressed (`0066b8f0`): on the face, or a slider's marker.
    pub fn mouse_down(&mut self, ui: &mut Ui, over: Option<TileId>, x: f32) {
        self.drag_from = x;
        self.held = None;
        let Some(tile) = over else {
            return;
        };
        if ui.number(tile, t::ID) as i32 != id::MARKER {
            return;
        }
        let Some(i) = ui.tiles[tile].parent.and_then(|p| self.item_of(p)) else {
            return;
        };
        self.held = Some((tile, i));
        self.drag_value = self.value(ui, self.current, i);
    }

    /// The pointer held and moved (`00670830`).
    pub fn mouse_drag(&mut self, ui: &mut Ui, over: Option<TileId>, x: f32) {
        let moved = x - self.drag_from;
        if moved == 0.0 {
            return;
        }
        if let Some((_, i)) = self.held {
            let at = self.current;
            let item = self.pages[at].items[i];
            let low = ui.number(item.tile, USER2);
            let high = ui.number(item.tile, USER3);
            let bar = ui
                .find_below(item.tile, "RSM_slider_bar")
                .map_or(1.0, |b| ui.number(b, t::WIDTH).max(1.0));
            let value = (high - low) * (moved / bar) + self.drag_value as f32;
            if self.set_value(ui, at, i, value.round() as i32) {
                self.apply(ui, i);
            }
            return;
        }
        // The face: a whole turn across its tile's width.
        let grab = self.tiles[id::FACE_GRAB as usize];
        if over == grab {
            if let Some(grab) = grab {
                let width = ui.number(grab, t::WIDTH).max(1.0);
                self.turn(moved / width * std::f32::consts::TAU);
                self.drag_from = x;
            }
        }
    }

    /// The pointer let go.
    pub fn mouse_up(&mut self) {
        self.held = None;
    }

    /// Turns the head (`0066b230`): the angle taken off, kept in a circle.
    pub fn turn(&mut self, angle: f32) {
        self.heading = (self.heading - angle).rem_euclid(std::f32::consts::TAU);
    }

    /// The wheel (`006711e0`): over the face the wanted zoom changes, away
    /// from the player zooming in; anywhere else the list scrolls.
    pub fn wheel_at(&mut self, ui: &mut Ui, over: Option<TileId>, notches: i32, now: f64) {
        if notches == 0 {
            return;
        }
        if over.is_some() && over == self.tiles[id::FACE_GRAB as usize] {
            let change = if notches > 0 { -WHEEL_ZOOM } else { WHEEL_ZOOM };
            self.zoom_target = (self.zoom_target + change).clamp(0.0, 1.0);
        } else {
            let part = if notches > 0 {
                id::SCROLL_UP
            } else {
                id::SCROLL_DOWN
            };
            self.click(ui, part, None, now);
        }
    }

    /// Every frame (`006711e0`): the zoom toward what's wanted.
    pub fn frame(&mut self, dt_ms: f32) {
        if self.zoom_target > self.zoom {
            self.zoom_speed += dt_ms * ZOOM_ACCELERATION;
            let step = (self.zoom_speed * dt_ms).min(self.zoom_target - self.zoom);
            self.zoom = (self.zoom + step).clamp(0.0, 1.0);
        } else if self.zoom_target < self.zoom {
            self.zoom_speed -= dt_ms * ZOOM_ACCELERATION;
            let step = (self.zoom_speed * dt_ms).max(self.zoom_target - self.zoom);
            self.zoom = (self.zoom + step).clamp(0.0, 1.0);
        } else {
            self.zoom_speed = 0.0;
        }
    }

    /// Done, and answered yes (`0066c550`): the menu leaves.
    pub fn finish(&mut self, ui: &mut Ui) {
        if !self.closed {
            ui.set_number(self.menu, menu::LEAVE_STACK, 1.0);
            self.closed = true;
        }
    }

    /// "Randomize" answered yes, or asked for again (`0066e530`).
    pub fn randomized(&mut self) {
        self.randomized = true;
        self.refill_hair = true;
    }

    /// `--menu-keys`, for automated runs: `next`, `back`, `itemN` clicks the
    /// page's Nth item, `rightN` / `leftN` / `jumprightN` / `jumpleftN` a
    /// slider's arrows or bar, `turn` and `zoomin` / `zoomout`.
    pub fn test_key(&mut self, ui: &mut Ui, key: &str, now: f64) -> bool {
        let key = key.to_ascii_lowercase();
        let number = |prefix: &str| -> Option<usize> { key.strip_prefix(prefix)?.parse().ok() };
        let item = |menu: &RaceSexMenu, n: usize| menu.pages[menu.current].items.get(n).copied();
        if key == "next" {
            self.click(ui, id::NEXT, None, now);
        } else if key == "back" {
            self.click(ui, id::BACK, None, now);
        } else if key == "turn" {
            self.turn(0.8);
        } else if key == "zoomin" {
            self.zoom_target = 0.0;
        } else if key == "zoomout" {
            self.zoom_target = 1.0;
        } else if let Some(i) = number("item").and_then(|n| item(self, n)) {
            let tile_id = ui.number(i.tile, t::ID) as i32;
            self.click(ui, tile_id, Some(i.tile), now);
        } else {
            for (prefix, part, id_of_part) in [
                ("jumpright", "RSM_slider_bar_right", id::BAR_RIGHT),
                ("jumpleft", "RSM_slider_bar_left", id::BAR_LEFT),
                ("right", "RSM_slider_arrow_right", id::ARROW_RIGHT),
                ("left", "RSM_slider_arrow_left", id::ARROW_LEFT),
            ] {
                if let Some(i) = number(prefix).and_then(|n| item(self, n)) {
                    if let Some(tile) = ui.find_below(i.tile, part) {
                        self.click(ui, id_of_part, Some(tile), now);
                        return true;
                    }
                }
            }
            return false;
        }
        true
    }

    /// The text and buttons Done asks with (`0066ffc0`: "%s %s?").
    fn confirm_done(&self, ui: &Ui) -> Request {
        let what = match self.mode {
            2 => "sRSMHair",
            3 => "sRSMFace",
            _ => "sRSMCharacter",
        };
        Request::ConfirmDone {
            text: format!("{} {}?", text(ui, "sRSMConfirmDone"), text(ui, what)),
            no: text(ui, "sNo"),
            yes: text(ui, "sYes"),
        }
    }
}

impl MenuCode for RaceSexMenu {
    fn class(&self) -> i32 {
        CLASS
    }

    /// `0066cdf0`.
    fn set_tile(&mut self, id: i32, tile: TileId) {
        if (0..6).contains(&id) {
            self.tiles[id as usize] = Some(tile);
        }
    }

    /// `00670e40`.
    fn click(&mut self, ui: &mut Ui, id: i32, tile: Option<TileId>, _now: f64) {
        let tile = tile.or_else(|| {
            usize::try_from(id)
                .ok()
                .and_then(|i| self.tiles.get(i).copied().flatten())
        });
        let at = self.current;
        if let Some(i) = tile.and_then(|tile| self.item_of(tile)) {
            let item = self.pages[at].items[i];
            let selected = {
                let name = custom(ui, "_selected");
                ui.number(item.tile, name) as i32 == 2
            };
            match item.kind {
                kind::MALE | kind::FEMALE | kind::RACE => {
                    if selected {
                        return;
                    }
                    self.requests.push(match item.kind {
                        kind::RACE => Request::Race(item.value),
                        _ => Request::Sex {
                            female: item.kind == kind::FEMALE,
                        },
                    });
                    self.refill_hair = true;
                    self.refill_eyes = true;
                }
                kind::RANDOMIZE => {
                    if self.randomized {
                        self.requests.push(Request::Randomize);
                    } else {
                        self.requests.push(Request::ConfirmRandomize {
                            text: text(ui, "sRSMConfirmRandomize"),
                            no: text(ui, "sNo"),
                            yes: text(ui, "sYes"),
                        });
                    }
                    return;
                }
                kind::HAIR => {
                    if selected {
                        return;
                    }
                    self.requests.push(Request::Hair(item.value));
                }
                kind::HEAD_PART => {
                    // The chosen one clicked again takes it off.
                    self.requests.push(Request::HeadPart(if selected {
                        None
                    } else {
                        Some(item.value)
                    }));
                }
                kind::EYES => {
                    if selected {
                        return;
                    }
                    self.requests.push(Request::Eyes(item.value));
                }
                to if to < NO_PAGE as i32 => {
                    self.open_page(ui, to as usize);
                    return;
                }
                _ => return,
            }
            self.randomized = false;
            return;
        }
        match id {
            id::BACK => {
                self.open_page(ui, self.pages[at].back);
                return;
            }
            id::NEXT => {
                if at == self.last_page {
                    let ask = self.confirm_done(ui);
                    self.requests.push(ask);
                } else {
                    self.open_page(ui, self.pages[at].next);
                }
                return;
            }
            _ => {}
        }
        let parent = tile.and_then(|t| ui.tiles[t].parent);
        match parent.and_then(|p| self.item_of(p)) {
            Some(i) => {
                self.slider_click(ui, id, i);
            }
            None => {
                let shown = tile.is_some_and(|t| ui.number(t, t::VISIBLE) != 0.0);
                if shown {
                    match id {
                        id::SCROLL_UP => self.scroll(ui, at, 1.0),
                        id::SCROLL_DOWN => self.scroll(ui, at, -1.0),
                        _ => return,
                    }
                    self.arrows(ui);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_label_the_tables_use_has_its_text() {
        let known = |name: &str| {
            DEFAULT_TEXT
                .iter()
                .chain(MORE_TEXT.iter())
                .any(|(n, _)| *n == name)
        };
        for (_, _, label, _) in SHAPE_SLIDERS {
            assert!(known(label), "{label}");
        }
        for (_, label) in TONE_SLIDERS {
            assert!(known(label), "{label}");
        }
    }

    #[test]
    fn the_modes_are_as_the_setup_has_them() {
        assert_eq!(first_page(0), page::SEX);
        assert_eq!(first_page(1), page::RACE);
        assert_eq!(first_page(2), page::HAIR);
        assert_eq!(first_page(3), page::FACE);
        assert_eq!(mode(1).last_page, page::HAIR);
        assert_eq!(mode(3).last_page, page::FACE);
        assert!(mode(1).model.ends_with("racesexinterface01.nif"));
    }

    #[test]
    fn the_zoom_speeds_up_and_stops_on_what_is_wanted() {
        let mut m = RaceSexMenu::new(0, 1);
        m.set_starting_zoom(0.5);
        m.zoom_target = 0.625;
        let mut last = m.zoom;
        for _ in 0..500 {
            m.frame(16.0);
            assert!(m.zoom >= last);
            last = m.zoom;
        }
        assert_eq!(m.zoom, 0.625);
        m.frame(16.0);
        assert_eq!(m.zoom_speed, 0.0);
    }

    #[test]
    fn turning_stays_in_the_circle() {
        let mut m = RaceSexMenu::new(0, 1);
        m.heading = 0.1;
        m.turn(0.5);
        assert!((m.heading - (std::f32::consts::TAU - 0.4)).abs() < 1.0e-5);
    }
}
