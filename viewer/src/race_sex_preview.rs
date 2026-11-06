//! The head on the face menu's screen (`FORenderedMenuRaceSex`, `0069bde0`,
//! `0069b430`). The game takes the player's own third-person model out of the
//! world into a scene of its own with `PlayerFaceLights01.NIF`, and draws that
//! under the menu. Here it's a body of its own built the same way as the
//! third-person one (`player_body`), on a layer of its own, rebuilt when what
//! the menu made changes.
//!
//! * The camera is at the origin looking along +y, +z up, a square view
//!   (`rendered_terminal::put_up`).
//! * The body's place (`0066b3b0`): between the mode's two places by the zoom,
//!   the height a share of how tall the player's model stands (`006f5de0`). Its
//!   turn starts 0.5236 from facing the camera and follows the pointer
//!   (`0066b230`).
//! * Worn on the head and in the hands goes while the menu's open
//!   (`00670be0`: body slots 0, 1, 5, 7, 8, 9, 10, 11, 14, 15).
//!
//! * Its lights reach three times the model's bounding radius (`0069bde0`).
//!   Here that's the radius of its pieces at rest, the game's being of the
//!   posed model.
//!
//! Differences from the game: the body's width and height aren't scaled apart
//! (`0066b230`, `00555180`), and it's rebuilt whole rather than morphed in
//! place (`0066d910`).

use bevy::prelude::*;
use world::chargen::race_sex::{self as rules, PlayerFace};
use world::dialogue::PLAYER_REF;

use crate::actors::ActorRig;
use crate::game_menus::{GameMenus, OpenMenu};
use crate::player_body::placement;
use crate::rendered_terminal::{RenderedTerminal, HEAD_LAYER};
use crate::{GameFiles, Spawner};

/// The body slots taken off while the menu's open (`00670be0`).
const TAKEN_OFF: [u32; 10] = [0, 1, 5, 7, 8, 9, 10, 11, 14, 15];
/// How tall to take the model with no bounds in its record.
const DEFAULT_HEIGHT: f32 = 128.0;

#[derive(Clone, PartialEq)]
struct Built {
    face: PlayerFace,
    female: bool,
    worn: Vec<esm::FormId>,
}

/// The head as it stands.
#[derive(Resource, Default)]
pub struct RaceSexPreview {
    built: Option<Built>,
    holder: Option<Entity>,
    height: f32,
    scale: f32,
}

pub struct RaceSexPreviewPlugin;

impl Plugin for RaceSexPreviewPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RaceSexPreview>().add_systems(
            Update,
            update_preview
                .after(crate::game_menus::draw_menus)
                .in_set(crate::frame_order::IdleSet),
        );
    }
}

/// What the player wears that stays on (`00670be0`).
fn worn_on(order: &esm::LoadOrder, worn: &[esm::FormId]) -> Vec<esm::FormId> {
    let off: u32 = TAKEN_OFF.iter().map(|slot| 1u32 << slot).sum();
    worn.iter()
        .copied()
        .filter(|&item| {
            let Some(rr) = order.get(item) else {
                return false;
            };
            if rr.entry.header.kind.as_bytes() != b"ARMO" {
                return false;
            }
            let Ok(record) = rr.record() else {
                return false;
            };
            let slots = record
                .get(esm::FourCC::new(b"BMDT"))
                .and_then(|s| s.data.get(..4).map(|d| u32::from_le_bytes(d.try_into().unwrap())))
                .unwrap_or(0);
            slots & off == 0
        })
        .collect()
}

/// How far the model's pieces reach from their middle.
fn bound_radius(scene: &cellview::ViewerScene) -> f32 {
    let mut low = [f32::MAX; 3];
    let mut high = [f32::MIN; 3];
    for draw in &scene.draws {
        for p in &scene.meshes[draw.mesh].positions {
            for k in 0..3 {
                low[k] = low[k].min(p[k]);
                high[k] = high[k].max(p[k]);
            }
        }
    }
    let half: f32 = (0..3).map(|k| ((high[k] - low[k]) / 2.0).powi(2)).sum();
    half.sqrt()
}

/// Builds the head when what it shows changes, and places it.
#[allow(clippy::too_many_arguments)]
fn update_preview(
    mut commands: Commands,
    game: Res<GameFiles>,
    state: Res<crate::dialogue::DialogueState>,
    mut preview: ResMut<RaceSexPreview>,
    terminal: Res<RenderedTerminal>,
    mut menus: ResMut<GameMenus>,
    mut spawner: Spawner,
    settings: Res<crate::Settings>,
    mut placed: Query<&mut Transform>,
) {
    let view = menus.screen().and_then(|s| {
        s.open.iter().find_map(|m| match m {
            OpenMenu::RaceSex(r) => Some((r.menu.heading, r.menu.zoom, r.menu.mode)),
            _ => None,
        })
    });
    let Some((heading, zoom, mode)) = view.filter(|_| terminal.head_settings().is_some()) else {
        // The menu's gone: so is the head.
        if let Some(holder) = preview.holder.take() {
            if let Ok(mut e) = commands.get_entity(holder) {
                e.despawn();
            }
        }
        preview.built = None;
        return;
    };
    let order = &game.0.order;
    let st = &state.0;
    let worn = st
        .equipped
        .get(&PLAYER_REF)
        .map(|list| worn_on(order, list.iter().copied().collect::<Vec<_>>().as_slice()))
        .unwrap_or_default();
    let wanted = PlayerFace::of_player(order, st.player_face.as_ref()).map(|face| Built {
        face,
        female: st.player_female.unwrap_or(false),
        worn,
    });
    if preview.built != wanted {
        if let Some(holder) = preview.holder.take() {
            if let Ok(mut e) = commands.get_entity(holder) {
                e.despawn();
            }
        }
        preview.built = wanted.clone();
        let Some(b) = wanted else {
            return;
        };
        let Some(look) =
            world::actor::player_look(order, b.female, &b.worn, None, Some(&b.face))
        else {
            return;
        };
        preview.height = rules::height_of_bounds(order).unwrap_or(DEFAULT_HEIGHT);
        preview.scale = look.scale;
        let scene = game.0.actor_scene(&look);
        let holder = commands
            .spawn((Transform::IDENTITY, Visibility::Visible))
            .id();
        preview.holder = Some(holder);
        let (lights, ambient) = cellview::rendered_terminal::face_lights(
            &game.0.assets,
            bound_radius(&scene) * cellview::rendered_terminal::LIGHT_RADIUS_MULT,
        );
        let mut lighting = crate::lockpick::menu_lighting(&lights, settings.brightness);
        lighting.ambient = Vec4::new(ambient[0], ambient[1], ambient[2], 0.0);
        let Some((root, joints, pieces, _)) =
            spawner.spawn_lone_actor_lit(&scene, lighting, holder, HEAD_LAYER)
        else {
            return;
        };
        for piece in pieces {
            commands.entity(piece).insert(crate::shared_light::MenuLit);
        }
        let mut rig = ActorRig::new(scene.actors[0].skeleton.clone(), look.scale, 0.0);
        rig.joints = joints;
        rig.preview = true;
        commands.entity(root).insert(rig);
    }
    let Some(holder) = preview.holder else {
        return;
    };
    let m = ui::menus::race_sex::mode(mode);
    let lerp = |i: usize| (m.far[i] - m.near[i]) * zoom + m.near[i];
    let feet = [lerp(0), lerp(1), lerp(2) * preview.height];
    if let Ok(mut transform) = placed.get_mut(holder) {
        *transform = placement(feet, heading, preview.scale);
    }
}
