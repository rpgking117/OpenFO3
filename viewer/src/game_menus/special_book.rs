//! Fallout 3's "You're SPECIAL!" book (`menus::Menu::Character(
//! CharacterMenu::SpecialBook)`, the nursery's `ShowSPECIALBookMenuParams
//! 40`): the menu's clicks and keys (`ui::menus::special_book`) carried out
//! on the book (`world::chargen::special_book`), and its model drawn by a
//! camera of its own into the HUD's picture (`cellview::special_book`).
//!
//! * A turn plays the page's sequence from its start (`00675a00` makes it
//!   start over); pages turn again only once it has played out
//!   (`00675470`).
//! * The model's pieces show and take pictures as the page's refresh
//!   functions decide; here every frame, from the values as they are.
//! * "All done" closes the book once every point is spent (`00676150`);
//!   otherwise `UIActivateNothing` plays.
//!
//! **Unsettled:** the menu activates `Forward` as it opens (`00676b70`);
//! whether the book shows that turn's start (closed) or plays it isn't
//! settled. Here the book opens on the cover, `Forward`'s first pose.

use std::sync::Arc;

use bevy::prelude::*;
use cellview::special_book::{Action, BookScene, Display};
use esm::FormId;
use ui::menus::special_book::{click, Input, SpecialBookMenu, FILE};
use world::chargen::special_book::{self as rules, Book};
use world::chargen::CharacterMenu;
use world::scripting::GameState;

use super::{OpenMenu, Screen};
use crate::menus::Menu;

pub struct BookScreen {
    pub menu: SpecialBookMenu,
    pub scene: Arc<BookScene>,
    pub size: UVec2,
    pub book: Book,
    /// The sequence posing the book and the seconds since it began; the
    /// time stands at 0 until a page is turned.
    pub sequence: Option<usize>,
    pub time: f32,
    pub playing: bool,
    /// The pointer in the window's pixels.
    pub pointer: Option<Vec2>,
}

/// Whether this module shows a request.
pub fn takes(menu: &Menu) -> bool {
    matches!(menu, Menu::Character(CharacterMenu::SpecialBook { .. }))
}

fn values(game: &cellview::Game, state: &GameState) -> [i32; 7] {
    std::array::from_fn(|i| rules::value(&game.order, state, i))
}

fn sound(game: &cellview::Game, name: &str, sounds: &mut Vec<FormId>) {
    if let Some(id) = game.order.form_by_editor_id(name) {
        sounds.push(id);
    }
}

impl BookScreen {
    pub fn display(&self, game: &cellview::Game, state: &GameState) -> Display {
        Display {
            page: self.book.page,
            focus: self.book.focus,
            gamepad: false,
            values: values(game, state),
            points: self.book.points,
        }
    }

    /// The sequence and the time in it that pose the book now.
    pub fn layer(&self) -> Option<(&nif::Sequence, f32)> {
        self.sequence.map(|i| {
            let s = &self.scene.sequences[i];
            (s, s.start + self.time.clamp(0.0, s.stop - s.start))
        })
    }

    fn play(&mut self, name: &str) {
        self.sequence = self.scene.sequences.iter().position(|s| s.name == name);
        self.time = 0.0;
        self.playing = true;
    }

    /// Whether the sequence has played out.
    fn played_out(&self) -> bool {
        self.layer().is_none_or(|(s, at)| at >= s.stop)
    }

    /// A click (`vf03`, `00676430`).
    // Translated from 00676430 (decompiled, Fallout3.exe 1.7.0.3)
    fn act(
        &mut self,
        id: i32,
        game: &cellview::Game,
        state: &mut GameState,
        sounds: &mut Vec<FormId>,
    ) {
        let order = &game.order;
        let points = self.book.points;
        match id {
            click::NEXT | click::PREVIOUS => {
                if !self.book.ready {
                    return;
                }
                if let Some(sequence) = self.book.turn(id == click::NEXT) {
                    sound(game, rules::PAGE_SOUND, sounds);
                    self.play(sequence);
                    self.book.ready = self.book.page < 1;
                }
            }
            click::INCREASE | click::DECREASE => {
                let Some(a) = self.book.attribute() else {
                    return;
                };
                self.change(a, id == click::INCREASE, game, state, sounds);
            }
            click::DONE => {
                if rules::may_finish(order, state, points) {
                    self.menu.closed = true;
                    println!("SPECIAL book: done.");
                } else {
                    sound(game, rules::NOT_DONE_SOUND, sounds);
                }
            }
            click::PICK => self.pick(game, state, sounds),
            click::FOCUS_UP | click::FOCUS_DOWN => {
                if self.book.move_focus(id == click::FOCUS_UP) {
                    sound(game, rules::FOCUS_SOUND, sounds);
                }
            }
            _ => {}
        }
    }

    /// Plus or minus on an attribute (`00676940`, `00676880`).
    fn change(
        &mut self,
        attribute: usize,
        increase: bool,
        game: &cellview::Game,
        state: &mut GameState,
        sounds: &mut Vec<FormId>,
    ) {
        let order = &game.order;
        let changed = if increase {
            rules::increase(order, state, attribute, self.book.points)
        } else {
            rules::decrease(order, state, attribute)
        };
        if changed {
            sound(game, rules::NUMBER_SOUND, sounds);
        }
    }

    /// A click anywhere (`vf03` id 5): the page's buttons under the
    /// pointer, through the book's camera.
    fn pick(&mut self, game: &cellview::Game, state: &mut GameState, sounds: &mut Vec<FormId>) {
        if self.book.page > rules::INDEX_PAGE {
            return;
        }
        let Some(pointer) = self.pointer else {
            return;
        };
        let camera = cellview::special_book::camera(&game.settings, self.size.x, self.size.y);
        let ray = Vec3::new(
            (2.0 * pointer.x / self.size.x as f32 - 1.0) * camera.tan_half_width,
            (1.0 - 2.0 * pointer.y / self.size.y as f32) * camera.tan_half_height,
            -1.0,
        )
        .normalize();
        let display = self.display(game, state);
        let picked = self
            .scene
            .pick_button(
                self.book.page,
                &display,
                [0.0, 0.0, 1.0],
                ray.to_array(),
                self.layer(),
            )
            .map(str::to_string);
        let Some(name) = picked else {
            return;
        };
        match cellview::special_book::action(&name) {
            Some(Action::Next) => self.act(click::NEXT, game, state, sounds),
            Some(Action::Previous) => self.act(click::PREVIOUS, game, state, sounds),
            Some(Action::Increase(a)) => self.change(a, true, game, state, sounds),
            Some(Action::Decrease(a)) => self.change(a, false, game, state, sounds),
            Some(Action::Done) => self.act(click::DONE, game, state, sounds),
            None => {}
        }
    }

    /// What the player did, carried out in order. The sounds to play.
    pub fn inputs(&mut self, game: &cellview::Game, state: &mut GameState) -> Vec<FormId> {
        let mut sounds = Vec::new();
        for input in self.menu.take_inputs() {
            if self.menu.closed {
                break;
            }
            match input {
                Input::Click(id) => self.act(id, game, state, &mut sounds),
                // `00676260`: plus only when it would go up, minus only when
                // it would go down.
                Input::Increase => {
                    let order = &game.order;
                    let wanted = self.book.attribute().is_some_and(|a| {
                        rules::total(order, state) < self.book.points
                            && rules::value(order, state, a) < rules::MOST
                    });
                    if wanted {
                        self.act(click::INCREASE, game, state, &mut sounds);
                    }
                }
                Input::Decrease => {
                    let wanted = self
                        .book
                        .attribute()
                        .is_some_and(|a| rules::value(&game.order, state, a) > rules::LEAST);
                    if wanted {
                        self.act(click::DECREASE, game, state, &mut sounds);
                    }
                }
            }
        }
        sounds
    }
}

/// Opens the book (`006794c0`).
pub fn open(screen: &mut Screen, game: &cellview::Game, state: &mut GameState, request: Menu) {
    let Menu::Character(CharacterMenu::SpecialBook { points }) = request else {
        return;
    };
    let scene = match BookScene::load(&game.assets) {
        Ok(scene) => Arc::new(scene),
        Err(e) => {
            println!("The SPECIAL book can't be shown: {e}");
            return;
        }
    };
    let mut menu = SpecialBookMenu::new(0);
    let tile = match screen.load(game, FILE, &mut menu) {
        Ok(tile) => tile,
        Err(e) => {
            println!("The SPECIAL book can't be shown: {e}");
            return;
        }
    };
    menu.menu = tile;
    let mut opened = BookScreen {
        menu,
        scene,
        size: screen.size,
        book: Book::new(points),
        sequence: None,
        time: 0.0,
        playing: false,
        pointer: None,
    };
    // `00676b70` activates `Forward`; its first pose is the closed book.
    opened.play("Forward");
    opened.playing = false;
    if std::env::var_os("OF3_BOOK_DEBUG").is_some() {
        debug_poses(&opened.scene);
    }
    println!(
        "SPECIAL book: {points} points, {} pieces, {} sequences.",
        opened.scene.scene.draws.len(),
        opened.scene.sequences.len()
    );
    let _ = state;
    screen.open.push(OpenMenu::SpecialBook(Box::new(opened)));
}

/// `OF3_BOOK_DEBUG`: the cover's and first page's poses at the start and
/// end of the first turns.
fn debug_poses(scene: &BookScene) {
    for s in scene.sequences.iter() {
        for node in ["cover", "S", "L01"] {
            if let Some(t) = s.tracks.iter().find(|t| t.node == node) {
                println!(
                    "  book {} {node}: start {:?} end {:?}",
                    s.name,
                    t.sample(s.start),
                    t.sample(s.stop)
                );
            }
        }
    }
}

/// The time passing for the book's turn; pages may turn once it's played
/// out (`00675470`).
pub fn update(screen: &mut Screen, dt: f32) {
    for opened in &mut screen.open {
        let OpenMenu::SpecialBook(book) = opened else {
            continue;
        };
        if book.playing {
            book.time += dt;
        }
        let done = book.played_out() || !book.playing;
        book.book.update_ready(done);
    }
}

use crate::lighting::GameLitMaterial;
use bevy::core_pipeline::tonemapping::{DebandDither, Tonemapping};
use bevy::render::camera::{CameraOutputMode, Exposure, RenderTarget};
use bevy::render::render_resource::{BlendState, WgpuFeatures};
use bevy::render::renderer::RenderDevice;
use bevy::render::view::RenderLayers;
use cellview::space;

const SCENE_LAYER: usize = 28;

#[derive(Component)]
pub struct BookPiece {
    mesh: usize,
    center: Vec3,
}

#[derive(Default)]
pub struct Shown {
    entities: Vec<Entity>,
    meshes: Vec<Handle<Mesh>>,
    materials: Vec<Handle<GameLitMaterial>>,
    textures: Vec<Option<Handle<Image>>>,
    layer: Option<Handle<Image>>,
    active: bool,
}

#[derive(bevy::ecs::system::SystemParam)]
pub struct SceneAssets<'w> {
    images: ResMut<'w, Assets<Image>>,
    meshes: ResMut<'w, Assets<Mesh>>,
    materials: ResMut<'w, Assets<GameLitMaterial>>,
    device: Option<Res<'w, RenderDevice>>,
}

fn scene_camera(target: RenderTarget) -> Camera {
    Camera {
        target,
        order: -5,
        hdr: true,
        clear_color: ClearColorConfig::Custom(Color::NONE),
        output_mode: CameraOutputMode::Write {
            blend_state: Some(BlendState::REPLACE),
            clear_color: ClearColorConfig::Custom(Color::NONE),
        },
        ..default()
    }
}

/// The book drawn into the HUD's picture before its tiles and the cursor
/// (as the menus with a 3D scene are, `super::compose_hud_over_scene`).
#[allow(clippy::too_many_arguments)]
pub fn draw(
    mut commands: Commands,
    menus: Res<super::GameMenus>,
    game: Res<crate::GameFiles>,
    state: Res<crate::dialogue::DialogueState>,
    settings: Res<crate::Settings>,
    hud: Option<Res<crate::hud::HudLayer>>,
    mut assets: SceneAssets,
    mut shown: Local<Shown>,
    mut pieces: Query<(
        &BookPiece,
        &mut Transform,
        &mut Visibility,
        &MeshMaterial3d<GameLitMaterial>,
    )>,
    world_camera: Query<&Projection, With<crate::FlyCamera>>,
) {
    let open = menus.screen.as_ref().and_then(|s| {
        s.open.iter().find_map(|m| match m {
            OpenMenu::SpecialBook(b) => Some(&**b),
            _ => None,
        })
    });
    let Some(open) = open else {
        if shown.active {
            for entity in shown.entities.drain(..) {
                commands.entity(entity).despawn();
            }
            for mesh in shown.meshes.drain(..) {
                assets.meshes.remove(&mesh);
            }
            for material in shown.materials.drain(..) {
                assets.materials.remove(&material);
            }
            for image in shown.textures.drain(..).flatten() {
                assets.images.remove(&image);
            }
            shown.active = false;
        }
        return;
    };
    if !shown.active {
        let target = crate::lockpick::menu_layer(
            &mut commands,
            hud.as_deref(),
            &mut shown.layer,
            &mut assets.images,
            open.size,
        );
        let compressed = assets
            .device
            .as_ref()
            .is_none_or(|d| d.features().contains(WgpuFeatures::TEXTURE_COMPRESSION_BC));
        shown.textures = open
            .scene
            .scene
            .textures
            .iter()
            .map(|t| crate::upload_texture(&mut assets.images, t, compressed, settings.anisotropy))
            .collect();
        let frustum = cellview::special_book::camera(&game.0.settings, open.size.x, open.size.y);
        let far = world_camera
            .iter()
            .find_map(|p| match p {
                Projection::Perspective(p) => Some(p.far),
                _ => None,
            })
            .unwrap_or(1000.0);
        // `00675660`: at (0, 0, 1) looking at the origin, +Y up.
        shown.entities.push(
            commands
                .spawn((
                    Camera3d::default(),
                    scene_camera(RenderTarget::from(target)),
                    Projection::from(PerspectiveProjection {
                        fov: frustum.vertical_fov(),
                        near: frustum.near * space::METERS_PER_UNIT,
                        far,
                        ..default()
                    }),
                    Transform::from_translation(Vec3::from(space::point([0.0, 0.0, 1.0])))
                        .looking_to(Vec3::NEG_Y, Vec3::NEG_Z),
                    Tonemapping::None,
                    DebandDither::Disabled,
                    bevy::pbr::ClusterConfig::None,
                    Exposure {
                        ev100: crate::START_EV100,
                    },
                    RenderLayers::layer(SCENE_LAYER),
                ))
                .id(),
        );
        let lighting = crate::lockpick::menu_lighting(&open.scene.lights, settings.brightness);
        for draw in &open.scene.scene.draws {
            let data = &open.scene.scene.meshes[draw.mesh];
            let center = crate::sort_center(data).unwrap_or([0.0; 3]);
            let mesh = assets.meshes.add(crate::game_mesh_around(data, center));
            let material = assets
                .materials
                .add(crate::lit_material(data, &shown.textures, lighting));
            shown.meshes.push(mesh.clone());
            shown.materials.push(material.clone());
            shown.entities.push(
                commands
                    .spawn((
                        Mesh3d(mesh),
                        MeshMaterial3d(material),
                        Transform::IDENTITY,
                        RenderLayers::layer(SCENE_LAYER),
                        crate::shared_light::MenuLit,
                        BookPiece {
                            mesh: draw.mesh,
                            center: Vec3::from(center),
                        },
                    ))
                    .id(),
            );
        }
        shown.active = true;
    }
    let display = open.display(&game.0, &state.0);
    for (piece, mut transform, mut visible, handle) in &mut pieces {
        let data = &open.scene.scene.meshes[piece.mesh];
        let matrix = BookScene::piece_matrix(data.motion.as_deref(), open.layer());
        *transform = Transform::from_matrix(
            Mat4::from_cols_array(&space::matrix(&matrix)) * Mat4::from_translation(piece.center),
        );
        let (show, picture) = display.appearance(&data.shape_name);
        *visible = if show {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if let Some(picture) = picture {
            if let Some((_, index)) = open.scene.pictures.iter().find(|(p, _)| *p == picture) {
                if let Some(material) = assets.materials.get_mut(handle.id()) {
                    let texture = shown.textures[*index].clone();
                    if material.base.base_color_texture != texture {
                        material.base.base_color_texture = texture;
                    }
                }
            }
        }
    }
}
