//! The rendered terminal on screen (`cellview::rendered_terminal`): the
//! terminal's and the hacking game's menus shown the way the PC game shows
//! them by default (`[RenderedTerminal] bUseRenderedTerminals`), on the
//! screen of the terminal model `TerminalInterface01.NIF` in front of the
//! player, not flat over the window.
//!
//! - The menus' pictures are drawn into a picture of 1280 × 960 menu
//!   units, one pixel each (`FORenderedMenu::Draw`, `007fba00`; the
//!   picture's size in pixels isn't traced), which the Pip-Boy's screen
//!   effect (`pipboy::ScreenMaterial`, `ISIFSCANBLEND`, `007fbee0`) turns
//!   into the screen's texture with the terminal's values: blur
//!   `fDefaultBlurRadius` 0.3 and `fDefaultBlurIntensity` 0.2
//!   (`007fc150`), scanlines `PipboyScanlines.dds` ×
//!   `fRenderedTerminalScanlineScale` (`007f9050`), the terminal colour
//!   (system colour 3, `007feeb0`) as its tint, the pulse and the passing
//!   band (`FORenderedMenu::OnIdle`; no burst, roll or shudder: only the
//!   Pip-Boy's menus start those).
//! - The model's pieces are drawn by a camera of their own with the
//!   game's frustum and the model's three point lights, into a picture the
//!   size of the window that is laid on the HUD's picture under the menus'
//!   pictures and the cursor, faded by the model's fade (in over
//!   `fFadeInTime`, out over `fFadeOutTime` after the player leaves).
//! - The pointer goes through the screen: what's under it is found where
//!   the ray from the camera meets the screen (`007fb790`); a click on the
//!   power button leaves at once (`007ffba0`, `007ffd50`).
//!
//! Differences from the game: the model is drawn after the image space
//! pass (the game draws it with the world, before, so it's graded and
//! bloomed with it, and only the screen after); the fade is laid over the
//! finished model rather than each piece's own alpha. The menus on the
//! screen fade in and out with every menu's fade (`ui::fade`; 0.75 s when
//! the player leaves, `menufade` set at `00757ea0`, `00766aa0`), drawn on
//! the screen while they fade. The game's own cursor picture is drawn
//! where the system pointer is.

use std::collections::HashMap;
use std::sync::Arc;

use bevy::core_pipeline::tonemapping::{DebandDither, Tonemapping};
use bevy::prelude::*;
use bevy::render::camera::{Exposure, RenderTarget};
use bevy::render::render_resource::WgpuFeatures;
use bevy::render::renderer::RenderDevice;
use bevy::render::view::RenderLayers;
use bevy::window::PrimaryWindow;
use cellview::rendered_terminal::{self, Fade, Hit, Settings, TerminalScene};
use cellview::space;
use ui::draw::{DrawItem, DrawKind};
use ui::pipboy::screen::{ScreenEffects, ScreenSettings};

use crate::hud::{HudLayer, Quad, TileMaterial};
use crate::lighting::GameLitMaterial;
use crate::pipboy::{ScreenMaterial, ScreenUniform};
use bevy::asset::{load_internal_asset, weak_handle};
use bevy::render::render_resource::{AsBindGroup, ShaderRef, ShaderType};
use bevy::sprite::{AlphaMode2d, Material2d, Material2dPlugin};
use ui::pipboy::screen::TvEffects;
use crate::GameFiles;

/// The render layers of the menus' picture, the screen effect and the
/// model.
const PICTURE_LAYER: usize = 33;
const EFFECT_LAYER: usize = 34;
const SCENE_LAYER: usize = 35;

/// The menus' picture in pixels: the game's 1280 × 960 menu units.
const PICTURE: UVec2 = UVec2::new(1280, 960);

/// Where the pointer is when it's off the screen: no tile is there.
pub const OFF_SCREEN: (f32, f32) = (-1.0e4, -1.0e4);

/// How long the terminal stays up with neither of its menus open before
/// it fades out by itself (the hacking menu hands over to the terminal's
/// a frame later; the game's own leaving fades it at once). The viewer's.
const ABSENT_SECONDS: f32 = 0.25;

const TV_SHADER: Handle<Shader> = weak_handle!("0a7e3d52-91c4-4b6f-8d20-6f3b1c5e9a47");

/// `ISTV`'s constants (`0069b270`, `00b3b220`).
#[derive(Clone, Copy, Debug, ShaderType)]
pub struct TvUniform {
    offsets: Vec4,
    effect: Vec4,
    distort: Vec4,
    fake: Vec4,
    tint: Vec4,
}

/// The face menu's screen effect (`tv.wgsl`).
#[derive(Asset, TypePath, AsBindGroup, Clone)]
pub struct TvMaterial {
    #[uniform(0)]
    u: TvUniform,
    #[texture(1)]
    #[sampler(2)]
    picture: Handle<Image>,
    #[texture(3)]
    #[sampler(4)]
    band: Handle<Image>,
    #[texture(5)]
    #[sampler(6)]
    scanlines: Handle<Image>,
}

impl Material2d for TvMaterial {
    fn fragment_shader() -> ShaderRef {
        TV_SHADER.into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Opaque
    }
}

pub struct RenderedTerminalPlugin;

impl Plugin for RenderedTerminalPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(app, TV_SHADER, "tv.wgsl", Shader::from_wgsl);
        app.add_plugins(Material2dPlugin::<TvMaterial>::default());
        app.init_resource::<RenderedTerminal>().add_systems(
            Update,
            show_terminal
                // Kept: drawn after the game's menus, before the viewer's,
                // inside the interface idle's set (`crate::frame_order`).
                .after(crate::game_menus::draw_menus)
                .before(crate::menus::run_menus)
                .in_set(crate::frame_order::IdleSet),
        );
    }
}

/// The render layer of the face menu's head (`race_sex_preview`).
pub const HEAD_LAYER: usize = 36;
/// The head's picture in pixels: the camera's view is square (`0069bde0`), and
/// the picture is stretched over the menus' as the game's is.
const HEAD_PICTURE: UVec2 = UVec2::new(960, 960);

/// Which rendered menu is up: the terminal's model, or the face menu's with
/// its way of opening (`ui::menus::race_sex::mode`).
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum Kind {
    #[default]
    Terminal,
    RaceSex(u8),
}

/// The rendered terminal's state.
#[derive(Resource, Default)]
pub struct RenderedTerminal {
    /// Read once: the settings and the model.
    settings: Option<Settings>,
    model: Option<Option<Arc<TerminalScene>>>,
    /// The face menu's, read when it first opens.
    race_settings: Option<Settings>,
    race_model: Option<(u8, Option<Arc<TerminalScene>>)>,
    /// Which one this is.
    pub kind: Kind,
    /// The face menu's page, for its lamps.
    pub race_page: usize,
    /// In use: the setting on, the model read and the HUD drawn.
    on: bool,
    /// The terminal and hacking menus' pictures this frame (in menu
    /// units), and their fonts' pictures.
    pub items: Vec<DrawItem>,
    pub font_paths: HashMap<usize, Vec<String>>,
    /// A terminal or hacking menu is open.
    pub menu_open: bool,
    /// The power button was clicked: gone at once.
    pub power_off: bool,
    fade: Option<Fade>,
    absent: f32,
    shown: Option<Shown>,
}

impl RenderedTerminal {
    /// Whether terminals are drawn this way: reads the settings and the
    /// model the first time.
    pub fn prepare(&mut self, game: &cellview::Game, hud: bool) -> bool {
        let settings = *self
            .settings
            .get_or_insert_with(|| Settings::from_ini(&game.settings));
        if !settings.on || !hud {
            self.on = false;
            return false;
        }
        if let Kind::RaceSex(mode) = self.kind {
            self.race_settings
                .get_or_insert_with(|| Settings::race_sex_from_ini(&game.settings));
            let fresh = self.race_model.as_ref().is_none_or(|(m, _)| *m != mode);
            if fresh {
                let file = ui::menus::race_sex::mode(mode).model;
                let model = rendered_terminal::race_sex_scene(&game.assets, file).map(Arc::new);
                if model.is_none() {
                    println!("The face menu's model ({file}) can't be read: it's drawn flat.");
                }
                self.race_model = Some((mode, model));
            }
            self.on = self.race_model.as_ref().is_some_and(|(_, m)| m.is_some());
            return self.on;
        }
        let model = self.model.get_or_insert_with(|| {
            let m = game.terminal_scene();
            if m.is_none() {
                println!(
                    "The rendered terminal's model ({}) can't be read: terminals are drawn flat.",
                    rendered_terminal::MODEL
                );
            }
            m
        });
        self.on = model.is_some();
        self.on
    }

    fn active_settings(&self) -> Option<Settings> {
        match self.kind {
            Kind::Terminal => self.settings,
            Kind::RaceSex(_) => self.race_settings,
        }
    }

    fn active_model(&self) -> Option<Arc<TerminalScene>> {
        match self.kind {
            Kind::Terminal => self.model.clone().flatten(),
            Kind::RaceSex(_) => self.race_model.as_ref().and_then(|(_, m)| m.clone()),
        }
    }

    /// The face menu's settings, while it's up.
    pub fn head_settings(&self) -> Option<Settings> {
        matches!(self.kind, Kind::RaceSex(_)).then_some(self.race_settings).flatten()
    }

    /// What's under the pointer at `at` (window pixels) in a window of
    /// `size`; `None` when terminals aren't drawn this way.
    pub fn hit(&self, game: &cellview::Game, at: Vec2, size: UVec2, click: bool) -> Option<Hit> {
        if !self.on {
            return None;
        }
        let model = self.active_model()?;
        let settings = self.active_settings()?;
        let size = size.max(UVec2::ONE);
        let camera = rendered_terminal::camera(&game.settings, &settings, size.x, size.y);
        let root = rendered_terminal::root_transform(&settings, size.x, size.y);
        let ndc = (
            2.0 * at.x / size.x as f32 - 1.0,
            1.0 - 2.0 * at.y / size.y as f32,
        );
        Some(model.pick.pick(&root, &camera, ndc, click))
    }

    /// The pointer in menu units for what's under it: on the screen, the
    /// point of the menus' picture; elsewhere, off every tile.
    pub fn menu_pointer(&self, hit: Hit) -> (f32, f32) {
        match (hit, self.kind) {
            (Hit::Screen(uv), Kind::RaceSex(_)) => rendered_terminal::race_sex_menu_point(uv),
            (Hit::Screen(uv), Kind::Terminal) => rendered_terminal::menu_point(uv),
            _ => OFF_SCREEN,
        }
    }
}

/// What's on screen of it.
struct Shown {
    kind: Kind,
    /// The face menu's lamps by page, and the page they show.
    glows: [Vec<Entity>; 4],
    lit_page: Option<usize>,
    size: UVec2,
    entities: Vec<Entity>,
    meshes: Vec<Handle<Mesh>>,
    lit: Vec<Handle<GameLitMaterial>>,
    images: Vec<Handle<Image>>,
    head_material: Option<Handle<TileMaterial>>,
    /// The face menu's effect, in place of the terminal's.
    tv: Option<(Handle<TvMaterial>, TvEffects)>,
    effect: Handle<ScreenMaterial>,
    composite: Handle<TileMaterial>,
    effects: ScreenEffects,
    tint: Vec4,
    white: Handle<Image>,
    last: Vec<DrawItem>,
    drawn: Vec<(Entity, Handle<Mesh>, Handle<TileMaterial>)>,
    item_images: HashMap<(String, bool), Option<Handle<Image>>>,
    font_images: HashMap<(usize, u32), Option<Handle<Image>>>,
}

/// The screen effect's settings for a terminal: the INI's interface
/// effects (`[InterfaceFX]`: the pulse, the band) with the terminal's blur
/// (`fDefaultBlurRadius`, `fDefaultBlurIntensity`, `007fc150`) and
/// scanlines.
fn effect_settings(game: &cellview::Game, settings: &Settings) -> ScreenSettings {
    let ini = |s: &str, k: &str| game.settings.get(s, k).map(str::to_string);
    let mut s = ScreenSettings::from_ini(&ini);
    let float =
        |key: &str, default: f32| game.settings.float("InterfaceFX", key).unwrap_or(default);
    s.blur_radius = float("fDefaultBlurRadius", 0.3);
    s.blur_intensity = float("fDefaultBlurIntensity", 0.2);
    s.scanline_frequency = settings.scanline_scale;
    s
}

/// The window's size in pixels.
fn window_size(windows: &Query<&Window, With<PrimaryWindow>>) -> UVec2 {
    windows
        .single()
        .map(|w| UVec2::new(w.physical_width(), w.physical_height()))
        .unwrap_or(UVec2::new(1920, 1080))
        .max(UVec2::ONE)
}

/// The assets the terminal makes.
#[derive(bevy::ecs::system::SystemParam)]
pub struct TerminalAssets<'w> {
    meshes: ResMut<'w, Assets<Mesh>>,
    lit: ResMut<'w, Assets<GameLitMaterial>>,
    images: ResMut<'w, Assets<Image>>,
    tiles: ResMut<'w, Assets<TileMaterial>>,
    screens: ResMut<'w, Assets<ScreenMaterial>>,
    tvs: ResMut<'w, Assets<TvMaterial>>,
    device: Option<Res<'w, RenderDevice>>,
}

/// Puts it up while a terminal or hacking menu is open, keeps its
/// pictures and fade going, and takes it away once it has faded out.
#[allow(clippy::too_many_arguments)]
fn show_terminal(
    mut commands: Commands,
    time: Res<Time>,
    game: Res<GameFiles>,
    settings: Res<crate::Settings>,
    hud: Option<Res<HudLayer>>,
    mut terminal: ResMut<RenderedTerminal>,
    mut menus: ResMut<crate::game_menus::GameMenus>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut assets: TerminalAssets,
) {
    let t = &mut *terminal;
    let dt = time.delta_secs();
    let now_ms = time.elapsed_secs() * 1000.0;
    let left = menus
        .screen()
        .is_some_and(|s| std::mem::take(&mut s.terminal_left));
    let power_off = std::mem::take(&mut t.power_off);
    let size = window_size(&windows);

    // Opening, leaving and closing.
    if t.on && t.menu_open {
        t.absent = 0.0;
        match t.fade.as_mut() {
            None => t.fade = Some(Fade::opening()),
            // Opened again while fading out.
            Some(f) if f.out && !left => f.out = false,
            _ => {}
        }
    } else if t.fade.is_some() {
        t.absent += dt;
    }
    let mut gone = power_off || !t.on;
    let active = t.active_settings();
    if let Some(f) = t.fade.as_mut() {
        if left || t.absent > ABSENT_SECONDS {
            f.out = true;
        }
        if let Some(s) = active {
            gone |= f.step(&s, dt);
        }
    }
    let stale = t
        .shown
        .as_ref()
        .is_some_and(|s| s.size != size || s.kind != t.kind);
    if gone || stale {
        if gone {
            t.fade = None;
        }
        if let Some(s) = t.shown.take() {
            take_away(&mut commands, s, &mut assets);
        }
    }
    let (Some(fade), Some(model), Some(st), Some(hud)) =
        (t.fade, t.active_model(), t.active_settings(), hud.as_deref())
    else {
        return;
    };
    let compressed = assets
        .device
        .as_ref()
        .is_none_or(|d| d.features().contains(WgpuFeatures::TEXTURE_COMPRESSION_BC));
    let game = &game.0;

    if t.shown.is_none() {
        t.shown = Some(put_up(
            &mut commands,
            game,
            &settings,
            &model,
            &st,
            hud,
            size,
            &mut assets,
            compressed,
            now_ms,
            t.kind,
        ));
    }
    let race_page = t.race_page;
    let Some(s) = t.shown.as_mut() else {
        return;
    };

    // The face menu's lamp for its page (`0069ba10`).
    if matches!(s.kind, Kind::RaceSex(_)) && s.lit_page != Some(race_page) {
        s.lit_page = Some(race_page);
        for (page, glows) in s.glows.iter().enumerate() {
            let shown = if page == race_page {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
            for &glow in glows {
                commands.entity(glow).insert(shown);
            }
        }
    }

    // The model's fade.
    if let Some(m) = assets.tiles.get_mut(&s.composite) {
        let want = Vec4::new(1.0, 1.0, 1.0, fade.alpha);
        if m.params.tint != want {
            m.params.tint = want;
        }
    }

    // The menus' picture, when it changed.
    if t.items != s.last {
        paint(&mut commands, game, t, &mut assets, compressed);
    }

    // The screen effect's values this frame (`007fbee0`).
    let Some(s) = t.shown.as_mut() else {
        return;
    };
    if let Some((tv, effects)) = s.tv.as_mut() {
        let p = effects.params(now_ms);
        let (vertical, horizontal) = (
            effects.settings.distort_vertical_scale,
            effects.settings.distort_horizontal_scale,
        );
        if let Some(m) = assets.tvs.get_mut(tv) {
            m.u.effect = Vec4::new(p.pulse, 0.0, 0.0, 0.0);
            m.u.distort = Vec4::new(vertical, p.jitter, horizontal, 0.0);
            m.u.fake = p.passing.map_or(Vec4::new(0.0, -1.0, 0.0, 0.0), |progress| {
                Vec4::new(vertical, progress, horizontal, 0.0)
            });
        }
        return;
    }
    let p = s.effects.params(now_ms);
    if let Some(m) = assets.screens.get_mut(&s.effect) {
        m.u = ScreenUniform {
            params: Vec4::new(p.blur_intensity, p.scroll, 1.0, p.scanline_frequency),
            distort: p.distort.map_or(Vec4::ZERO, |(v, progress, h)| {
                Vec4::new(v, progress, h, 0.0)
            }),
            tint: s.tint,
            offsets: Vec4::new(
                p.blur_radius / PICTURE.x as f32,
                p.blur_radius / PICTURE.y as f32,
                0.0,
                0.0,
            ),
        };
    }
}

/// The cameras, pictures and model (`007feeb0`, `007ff650`).
#[allow(clippy::too_many_arguments)]
fn put_up(
    commands: &mut Commands,
    game: &cellview::Game,
    settings: &crate::Settings,
    model: &TerminalScene,
    st: &Settings,
    hud: &HudLayer,
    size: UVec2,
    assets: &mut TerminalAssets,
    compressed: bool,
    now_ms: f32,
    kind: Kind,
) -> Shown {
    let picture = crate::pipboy::target_image(&mut assets.images, PICTURE);
    let screen = crate::pipboy::target_image(&mut assets.images, PICTURE);
    let scene = crate::pipboy::target_image(&mut assets.images, size);
    let mut entities = Vec::new();
    let camera_2d = |target: &Handle<Image>, order: isize, layer: usize| {
        (
            Camera2d,
            Camera {
                target: RenderTarget::from(target.clone()),
                order,
                hdr: true,
                // The rendered menu's background colour (`007faa20`: black;
                // its alpha of 0 is opaque here, as the game's screen is
                // drawn opaque whatever its texture's alpha).
                clear_color: ClearColorConfig::Custom(Color::BLACK),
                ..default()
            },
            Tonemapping::None,
            DebandDither::Disabled,
            Msaa::Off,
            RenderLayers::layer(layer),
        )
    };
    entities.push(commands.spawn(camera_2d(&picture, -12, PICTURE_LAYER)).id());
    entities.push(commands.spawn(camera_2d(&screen, -11, EFFECT_LAYER)).id());

    // The screen effect.
    let white = {
        use bevy::asset::RenderAssetUsages;
        use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
        let mut white = Image::new_fill(
            Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            &[255, 255, 255, 255],
            TextureFormat::Rgba8Unorm,
            RenderAssetUsages::RENDER_WORLD,
        );
        white.sampler = crate::hud::sampler(false, false);
        assets.images.add(white)
    };
    let scanline_file = game
        .settings
        .get("InterfaceFX", "sScanlineTexture")
        .map(|p| {
            let p = p.trim().to_ascii_lowercase().replace('/', "\\");
            p.strip_prefix("data\\").map(str::to_string).unwrap_or(p)
        })
        .unwrap_or_else(|| "textures\\pipboy3000\\pipboyscanlines.dds".to_string());
    let scanlines = crate::hud::upload_picture(
        &mut assets.images,
        game,
        &scanline_file,
        (true, true),
        compressed,
    )
    .unwrap_or_else(|| white.clone());
    let band = crate::hud::upload_picture(
        &mut assets.images,
        game,
        "textures\\pipboy3000\\pipboydistorteffectmap.dds",
        (false, false),
        compressed,
    )
    .unwrap_or_else(|| white.clone());
    let effect = assets.screens.add(ScreenMaterial {
        u: ScreenUniform {
            params: Vec4::new(0.0, 0.0, 1.0, 0.0),
            distort: Vec4::ZERO,
            tint: Vec4::ONE,
            offsets: Vec4::ZERO,
        },
        picture: picture.clone(),
        scanlines: scanlines.clone(),
        band: band.clone(),
    });
    let effect_mesh = assets
        .meshes
        .add(Rectangle::new(PICTURE.x as f32, PICTURE.y as f32));
    let tint = ui::SystemColors::new(None, None)
        .get(3)
        .map_or(Vec4::ONE, |[r, g, b]| Vec4::new(r, g, b, 1.0));
    let tv = if matches!(kind, Kind::RaceSex(_)) {
        let material = assets.tvs.add(TvMaterial {
            u: TvUniform {
                // A texel of the picture (`00b3b220`).
                offsets: Vec4::new(1.0 / PICTURE.x as f32, 1.0 / PICTURE.y as f32, 0.0, 0.0),
                effect: Vec4::ZERO,
                distort: Vec4::ZERO,
                fake: Vec4::new(0.0, -1.0, 0.0, 0.0),
                tint,
            },
            picture: picture.clone(),
            band: band.clone(),
            scanlines: scanlines.clone(),
        });
        entities.push(
            commands
                .spawn((
                    Mesh2d(effect_mesh.clone()),
                    MeshMaterial2d(material.clone()),
                    Transform::IDENTITY,
                    RenderLayers::layer(EFFECT_LAYER),
                ))
                .id(),
        );
        let jitter_ms = game
            .settings
            .float("RenderedMenus", "fRaceSexMenuDistortDuration")
            .unwrap_or(50.0);
        Some((
            material,
            TvEffects::new(effect_settings(game, st), jitter_ms, now_ms),
        ))
    } else {
        entities.push(
            commands
                .spawn((
                    Mesh2d(effect_mesh.clone()),
                    MeshMaterial2d(effect.clone()),
                    Transform::IDENTITY,
                    RenderLayers::layer(EFFECT_LAYER),
                ))
                .id(),
        );
        None
    };

    // The face menu's head (`0069bde0`, `0069b430`): drawn first, by a camera
    // of its own at the origin looking along +y with +z up (Bevy's own axes),
    // its square view stretched over the whole picture, the menus over it.
    let mut head_picture = None;
    let mut head_material = None;
    if matches!(kind, Kind::RaceSex(_)) {
        let head = crate::pipboy::target_image(&mut assets.images, HEAD_PICTURE);
        let tangent = rendered_terminal::head_view_tangent(&game.settings, st);
        let near = game
            .settings
            .float("Display", "fNearDistance")
            .unwrap_or(5.0);
        entities.push(
            commands
                .spawn((
                    Camera3d::default(),
                    Camera {
                        target: RenderTarget::from(head.clone()),
                        order: -14,
                        hdr: true,
                        clear_color: ClearColorConfig::Custom(Color::BLACK),
                        ..default()
                    },
                    Tonemapping::None,
                    DebandDither::Disabled,
                    Projection::from(PerspectiveProjection {
                        fov: 2.0 * tangent.atan(),
                        near: near * space::METERS_PER_UNIT,
                        far: 100.0,
                        ..default()
                    }),
                    Exposure {
                        ev100: crate::START_EV100,
                    },
                    Transform::IDENTITY,
                    RenderLayers::layer(HEAD_LAYER),
                ))
                .id(),
        );
        let mut plain = TileMaterial::plain(Vec4::ONE, head.clone(), white.clone());
        plain.params.mode.y = 1.0;
        let material = assets.tiles.add(plain);
        entities.push(
            commands
                .spawn((
                    Mesh2d(effect_mesh.clone()),
                    MeshMaterial2d(material.clone()),
                    Transform::from_xyz(0.0, 0.0, -1.0),
                    RenderLayers::layer(PICTURE_LAYER),
                ))
                .id(),
        );
        head_picture = Some(head);
        head_material = Some(material);
    }

    // The model's camera (`007feeb0`): at the origin looking along +x with
    // +y up; the scene's +x is Bevy's +x, its +y Bevy's −z (`space`).
    let camera = rendered_terminal::camera(&game.settings, st, size.x, size.y);
    entities.push(
        commands
            .spawn((
                Camera3d::default(),
                Camera {
                    target: RenderTarget::from(scene.clone()),
                    order: -10,
                    hdr: true,
                    clear_color: ClearColorConfig::Custom(Color::NONE),
                    ..default()
                },
                Tonemapping::None,
                DebandDither::Disabled,
                Projection::from(PerspectiveProjection {
                    fov: camera.vertical_fov(),
                    near: camera.near * space::METERS_PER_UNIT,
                    far: 100.0,
                    ..default()
                }),
                Exposure {
                    ev100: crate::START_EV100,
                },
                Transform::from_translation(Vec3::ZERO).looking_to(Vec3::X, Vec3::NEG_Z),
                RenderLayers::layer(SCENE_LAYER),
            ))
            .id(),
    );

    // The model's pieces, lit by its lights where its top node puts them;
    // the screen's piece shows the screen effect's picture.
    let root = rendered_terminal::root_transform(st, size.x, size.y);
    let root_matrix = Mat4::from_cols_array(&cellview::column_major(&root));
    let lights = rendered_terminal::lights_at(&model.lights, &root);
    let lighting = crate::lockpick::menu_lighting(&lights, settings.brightness);
    let textures: Vec<Option<Handle<Image>>> = model
        .scene
        .textures
        .iter()
        .map(|t| crate::upload_texture(&mut assets.images, t, compressed, settings.anisotropy))
        .collect();
    let mut images: Vec<Handle<Image>> = textures.iter().flatten().cloned().collect();
    images.extend([picture, screen.clone(), scene.clone(), white.clone()]);
    images.extend([scanlines, band]);
    images.extend(head_picture);
    let mut meshes = vec![effect_mesh];
    let mut lit = Vec::new();
    let mut glows: [Vec<Entity>; 4] = Default::default();
    for draw in &model.scene.draws {
        let data = &model.scene.meshes[draw.mesh];
        let center = crate::sort_center(data).unwrap_or([0.0; 3]);
        let mesh = assets.meshes.add(crate::game_mesh_around(data, center));
        let mut material = crate::lit_material(data, &textures, lighting);
        if model.screen_shape.as_deref() == Some(data.shape_name.as_str()) {
            material.base.base_color_texture = Some(screen.clone());
        }
        let material = assets.lit.add(material);
        let placed = root_matrix * Mat4::from_cols_array(&draw.transform);
        let transform = Transform::from_matrix(
            Mat4::from_cols_array(&space::matrix(&placed.to_cols_array()))
                * Mat4::from_translation(Vec3::from(center)),
        );
        meshes.push(mesh.clone());
        lit.push(material.clone());
        let piece = commands
            .spawn((
                Mesh3d(mesh),
                MeshMaterial3d(material),
                transform,
                RenderLayers::layer(SCENE_LAYER),
                crate::shared_light::MenuLit,
            ))
            .id();
        entities.push(piece);
        if let Some(page) = model
            .glows
            .iter()
            .position(|g| g.as_deref() == Some(data.shape_name.as_str()))
        {
            glows[page].push(piece);
        }
    }

    // The model's picture on the HUD's, under the menus' pictures.
    let composite = assets.tiles.add(TileMaterial::plain(
        Vec4::new(1.0, 1.0, 1.0, 0.0),
        scene,
        white.clone(),
    ));
    let quad = assets
        .meshes
        .add(Rectangle::new(size.x as f32, size.y as f32));
    meshes.push(quad.clone());
    let _ = hud;
    entities.push(
        commands
            .spawn((
                Mesh2d(quad),
                MeshMaterial2d(composite.clone()),
                Transform::from_xyz(0.0, 0.0, -1.0),
                RenderLayers::layer(crate::hud::HUD_LAYER),
            ))
            .id(),
    );

    Shown {
        kind,
        glows,
        lit_page: None,
        head_material,
        tv,
        size,
        entities,
        meshes,
        lit,
        images,
        effect,
        composite,
        effects: ScreenEffects::new(effect_settings(game, st), now_ms),
        tint,
        white,
        last: Vec::new(),
        drawn: Vec::new(),
        item_images: HashMap::new(),
        font_images: HashMap::new(),
    }
}

/// Everything taken away.
fn take_away(commands: &mut Commands, s: Shown, assets: &mut TerminalAssets) {
    for e in s.entities.into_iter().chain(s.drawn.iter().map(|d| d.0)) {
        if let Ok(mut e) = commands.get_entity(e) {
            e.despawn();
        }
    }
    for (_, mesh, material) in s.drawn {
        assets.meshes.remove(&mesh);
        assets.tiles.remove(&material);
    }
    for m in s.meshes {
        assets.meshes.remove(&m);
    }
    for m in s.lit {
        assets.lit.remove(&m);
    }
    assets.screens.remove(&s.effect);
    if let Some((tv, _)) = &s.tv {
        assets.tvs.remove(tv);
    }
    assets.tiles.remove(&s.composite);
    if let Some(head) = &s.head_material {
        assets.tiles.remove(head);
    }
    for i in s
        .images
        .into_iter()
        .chain(s.item_images.into_values().flatten())
        .chain(s.font_images.into_values().flatten())
    {
        assets.images.remove(&i);
    }
}

/// The menus' pictures into the picture, one menu unit a pixel (as the
/// Pip-Boy's, `pipboy::update_pipboy`).
fn paint(
    commands: &mut Commands,
    game: &cellview::Game,
    t: &mut RenderedTerminal,
    assets: &mut TerminalAssets,
    compressed: bool,
) {
    let Some(s) = t.shown.as_mut() else {
        return;
    };
    for (e, mesh, material) in s.drawn.drain(..) {
        if let Ok(mut e) = commands.get_entity(e) {
            e.despawn();
        }
        assets.meshes.remove(&mesh);
        assets.tiles.remove(&material);
    }
    for (i, item) in t.items.iter().enumerate() {
        let tint = Vec4::from_array(item.color);
        let mut pieces: Vec<(Handle<Image>, Vec<Quad>)> = Vec::new();
        match &item.kind {
            DrawKind::Image {
                texture,
                rect,
                uv,
                repeat_u,
                ..
            } => {
                let handle = s
                    .item_images
                    .entry((texture.clone(), *repeat_u))
                    .or_insert_with(|| {
                        crate::hud::upload_picture(
                            &mut assets.images,
                            game,
                            texture,
                            (*repeat_u, false),
                            compressed,
                        )
                    })
                    .clone();
                if let Some(handle) = handle {
                    let corners = [
                        [uv[0], uv[1]],
                        [uv[2], uv[1]],
                        [uv[0], uv[3]],
                        [uv[2], uv[3]],
                    ];
                    pieces.push((handle, vec![(*rect, corners)]));
                }
            }
            DrawKind::Text { font, glyphs } => {
                let Some(paths) = t.font_paths.get(font) else {
                    continue;
                };
                let mut by_picture: HashMap<u32, Vec<Quad>> = HashMap::new();
                for (rect, uv, picture) in glyphs {
                    by_picture.entry(*picture).or_default().push((*rect, *uv));
                }
                let mut pictures: Vec<_> = by_picture.into_iter().collect();
                pictures.sort_by_key(|(p, _)| *p);
                for (picture, quads) in pictures {
                    let Some(path) = paths.get(picture as usize) else {
                        continue;
                    };
                    let handle = s
                        .font_images
                        .entry((*font, picture))
                        .or_insert_with(|| {
                            crate::hud::upload_font_picture(&mut assets.images, game, path)
                        })
                        .clone();
                    if let Some(texture) = handle {
                        pieces.push((texture, quads));
                    }
                }
            }
            // A menu's `nif` tile (the start menu's background): the
            // terminal and hacking menus have none.
            DrawKind::Model { .. } => continue,
        }
        for (texture, quads) in pieces {
            let mesh = assets
                .meshes
                .add(crate::hud::quads_mesh(&quads, 1.0, PICTURE));
            let material = assets
                .tiles
                .add(TileMaterial::plain(tint, texture, s.white.clone()));
            let entity = commands
                .spawn((
                    Mesh2d(mesh.clone()),
                    MeshMaterial2d(material.clone()),
                    Transform::from_xyz(0.0, 0.0, i as f32 * 0.01),
                    RenderLayers::layer(PICTURE_LAYER),
                ))
                .id();
            s.drawn.push((entity, mesh, material));
        }
    }
    s.last = t.items.clone();
}
