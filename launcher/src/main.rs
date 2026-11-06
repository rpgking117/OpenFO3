//! OpenFO3 Launcher: finds your Fallout 3, sets up the load order and the
//! display, and starts the game. Its pages (Data Files, Display, Settings,
//! Import, About) are tabs under a banner, with Play at the bottom.
//!
//! OpenFO3 by RPGKING The Mad Modder. It never changes the Fallout 3
//! installation or the original game's settings and saves: its own settings
//! live in `%APPDATA%\OpenFO3`, its saves and log in
//! `Documents\My Games\OpenFO3`.

#![cfg_attr(windows, windows_subsystem = "windows")]

mod config;
mod detect;
mod import;
mod plugins;
mod ui;

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use config::{Config, Start, WindowMode};
use eframe::egui::{self, Align, Color32, CornerRadius, Layout, Margin, RichText, Sense, Vec2};
use ui::{ACCENT, ACCENT_LIGHT, BAD, GOOD, MOD_BLUE, MUTED, TEXT};

const VERSION: &str = env!("CARGO_PKG_VERSION");
const CREDIT: &str = "by RPGKING The Mad Modder";

/// The common screen sizes offered on the Display page.
const RESOLUTIONS: [(u32, u32); 8] = [
    (1280, 720),
    (1366, 768),
    (1600, 900),
    (1920, 1080),
    (1920, 1200),
    (2560, 1080),
    (2560, 1440),
    (3840, 2160),
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    DataFiles,
    Display,
    Settings,
    Import,
    About,
}

impl Page {
    const ALL: [Page; 5] = [
        Page::DataFiles,
        Page::Display,
        Page::Settings,
        Page::Import,
        Page::About,
    ];

    fn label(self) -> &'static str {
        match self {
            Page::DataFiles => "Data Files",
            Page::Display => "Display",
            Page::Settings => "Settings",
            Page::Import => "Import",
            Page::About => "About",
        }
    }

    fn icon(self) -> char {
        match self {
            Page::DataFiles => ui::FOLDER,
            Page::Display => ui::MONITOR,
            Page::Settings => ui::GEAR,
            Page::Import => ui::IMPORT,
            Page::About => ui::INFO,
        }
    }
}

/// The official files' names as the game's menus give them.
fn friendly(name: &str) -> Option<&'static str> {
    Some(match name.to_ascii_lowercase().as_str() {
        "fallout3.esm" => "Fallout 3",
        "anchorage.esm" => "Operation: Anchorage",
        "thepitt.esm" => "The Pitt",
        "brokensteel.esm" => "Broken Steel",
        "pointlookout.esm" => "Point Lookout",
        "zeta.esm" => "Mothership Zeta",
        _ => return None,
    })
}

fn is_base(name: &str) -> bool {
    name.eq_ignore_ascii_case("Fallout3.esm")
}

fn window_label(mode: WindowMode) -> &'static str {
    match mode {
        WindowMode::Windowed => "windowed",
        WindowMode::Borderless => "borderless",
        WindowMode::Fullscreen => "fullscreen",
    }
}

struct Launcher {
    config: Config,
    page: Page,
    /// Every Fallout 3 found on this computer.
    found: Vec<PathBuf>,
    selected_plugin: Option<usize>,
    status: String,
    status_bad: bool,
    /// What the original game's settings say (the Import page).
    imported: Option<import::Imported>,
    game: Option<Child>,
}

impl Launcher {
    fn new(cc: &eframe::CreationContext<'_>) -> Launcher {
        ui::style(&cc.egui_ctx);
        let mut config = Config::load();
        let found = detect::data_dirs();
        let first_run = config.data_dir.is_none();
        if config.data_dir.as_deref().is_none_or(|d| !config::is_data_dir(d)) {
            config.data_dir = found.first().cloned();
        }
        let mut l = Launcher {
            config,
            page: Page::DataFiles,
            found,
            selected_plugin: None,
            status: String::new(),
            status_bad: false,
            imported: None,
            game: None,
        };
        l.rescan();
        match (&l.config.data_dir, first_run) {
            (Some(d), true) => l.say(format!("Found Fallout 3 at {}", d.display())),
            (Some(_), false) => l.say("Ready to play."),
            (None, _) => l.warn("Fallout 3 wasn't found: choose its Data folder."),
        }
        l
    }

    fn say(&mut self, s: impl Into<String>) {
        self.status = s.into();
        self.status_bad = false;
    }

    fn warn(&mut self, s: impl Into<String>) {
        self.status = s.into();
        self.status_bad = true;
    }

    fn ready(&self) -> bool {
        self.config
            .data_dir
            .as_deref()
            .is_some_and(config::is_data_dir)
    }

    /// The plugins in the Data folder, merged into the kept order.
    fn rescan(&mut self) {
        if let Some(d) = &self.config.data_dir {
            let found = plugins::scan(d);
            self.config.plugins = plugins::merge(&self.config.plugins, &found);
        } else {
            self.config.plugins.clear();
        }
        self.selected_plugin = None;
    }

    fn use_data_dir(&mut self, dir: PathBuf) {
        self.say(format!("Using {}", dir.display()));
        self.config.data_dir = Some(dir);
        self.rescan();
        self.save();
    }

    fn save(&mut self) {
        if let Err(e) = self.config.save() {
            self.warn(format!("Couldn't save the settings: {e}"));
        }
    }

    fn play(&mut self, ctx: &egui::Context) {
        self.save();
        let Some(data) = self.config.data_dir.clone() else {
            self.warn("Choose your Fallout 3 Data folder first.");
            self.page = Page::DataFiles;
            return;
        };
        let plugins = match self.config.write_plugins() {
            Ok(p) => p,
            Err(e) => return self.warn(format!("Couldn't write the plugin list: {e}")),
        };
        let Some(exe) = game_exe(&self.config) else {
            self.page = Page::Settings;
            return self.warn("OpenFO3.exe wasn't found beside the launcher: choose it under Settings.");
        };
        let saves = config::saves_dir();
        if let Err(e) = std::fs::create_dir_all(&saves) {
            return self.warn(format!("Couldn't make the saves folder: {e}"));
        }
        let log = std::fs::File::create(config::log_path());
        let mut cmd = Command::new(&exe);
        cmd.args(game_args(&self.config, &data, &plugins))
            .current_dir(&saves);
        match log.and_then(|f| Ok((f.try_clone()?, f))) {
            Ok((out, err)) => {
                cmd.stdout(Stdio::from(out)).stderr(Stdio::from(err));
            }
            Err(_) => {
                cmd.stdout(Stdio::null()).stderr(Stdio::null());
            }
        }
        detect::hide_console(&mut cmd);
        match cmd.spawn() {
            Ok(child) => {
                if self.config.keep_open {
                    self.say("OpenFO3 is running.");
                    self.game = Some(child);
                } else {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
            Err(e) => self.warn(format!("Couldn't start {}: {e}", exe.display())),
        }
    }
}

/// The game's program: the one chosen, else `OpenFO3.exe` beside the
/// launcher (or the development build's `nv-viewer.exe`).
fn game_exe(config: &Config) -> Option<PathBuf> {
    if let Some(p) = config.game_exe.as_ref().filter(|p| p.is_file()) {
        return Some(p.clone());
    }
    let here = std::env::current_exe().ok()?.parent()?.to_path_buf();
    ["OpenFO3.exe", "nv-viewer.exe"]
        .iter()
        .map(|n| here.join(n))
        .find(|p| p.is_file())
}

/// The game's command line for these settings.
fn game_args(config: &Config, data: &Path, plugins: &Path) -> Vec<String> {
    let mut a = vec![
        data.display().to_string(),
        "--plugins".into(),
        plugins.display().to_string(),
        "--window".into(),
        config.window.arg().into(),
        "--resolution".into(),
        format!("{},{}", config.resolution.0, config.resolution.1),
        "--vsync".into(),
        if config.vsync { "on" } else { "off" }.into(),
    ];
    if (config.brightness - 1.0).abs() > 0.001 {
        a.push("--brightness".into());
        a.push(format!("{:.2}", config.brightness));
    }
    if config.subtitles {
        a.push("--subtitles".into());
    }
    if config.skip_movies {
        a.push("--no-movies".into());
    }
    if config.start == Start::NewGame {
        a.push("--new-game".into());
    }
    a
}

/// "Game of the Year (all five add-ons)", or how many add-ons there are.
fn edition(plugins: &[config::Plugin]) -> String {
    let addons = plugins
        .iter()
        .filter(|p| plugins::is_official(&p.name) && !is_base(&p.name))
        .count();
    match addons {
        5 => "Game of the Year: all five add-ons".into(),
        0 => "No add-ons".into(),
        1 => "1 add-on".into(),
        n => format!("{n} add-ons"),
    }
}

fn open_folder(path: &Path) {
    let _ = std::fs::create_dir_all(path);
    let _ = Command::new("explorer").arg(path).spawn();
}

impl eframe::App for Launcher {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // The game ended (with the launcher kept open).
        if let Some(child) = &mut self.game {
            if let Ok(Some(status)) = child.try_wait() {
                self.game = None;
                if status.success() {
                    self.say("OpenFO3 closed.");
                } else {
                    self.warn(format!(
                        "OpenFO3 stopped with an error: see {}",
                        config::log_path().display()
                    ));
                }
            }
            ctx.request_repaint_after(std::time::Duration::from_millis(500));
        }

        egui::TopBottomPanel::top("banner")
            .frame(egui::Frame::new().fill(ui::BG))
            .show_separator_line(false)
            .show(ctx, |ui| ui::banner(ui, CREDIT, VERSION));

        egui::TopBottomPanel::top("tabs")
            .frame(
                egui::Frame::new()
                    .fill(ui::PANEL)
                    .inner_margin(Margin::symmetric(16, 0)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    for page in Page::ALL {
                        if ui::tab(ui, page.icon(), page.label(), self.page == page) {
                            self.page = page;
                            if page == Page::Import {
                                self.imported = Some(import::read());
                            }
                        }
                    }
                });
            });

        egui::TopBottomPanel::bottom("play")
            .frame(
                egui::Frame::new()
                    .fill(ui::PANEL)
                    .inner_margin(Margin::symmetric(24, 14)),
            )
            .show(ctx, |ui| self.play_bar(ui, ctx));

        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(ui::BG).inner_margin(Margin::symmetric(0, 18)))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        // A centred column, not wider than reads well.
                        let avail = ui.available_width();
                        let width = (avail - 56.0).min(900.0);
                        let pad = ((avail - width) / 2.0).max(0.0);
                        ui.horizontal_top(|ui| {
                            ui.add_space(pad - ui.spacing().item_spacing.x);
                            ui.vertical(|ui| {
                                ui.set_width(width);
                                match self.page {
                                    Page::DataFiles => self.data_files(ui),
                                    Page::Display => self.display(ui),
                                    Page::Settings => self.settings(ui),
                                    Page::Import => self.import(ui),
                                    Page::About => about(ui),
                                }
                            });
                        });
                    });
            });
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        let _ = self.config.save();
    }
}

impl Launcher {
    fn play_bar(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let ready = self.ready();
        ui.horizontal(|ui| {
            ui::dot(ui, if self.status_bad || !ready { BAD } else { GOOD });
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                let summary = if ready {
                    let active = self.config.plugins.iter().filter(|p| p.enabled).count();
                    let (w, h) = self.config.resolution;
                    format!(
                        "{active} plugin{} · {w} × {h} {}",
                        if active == 1 { "" } else { "s" },
                        window_label(self.config.window)
                    )
                } else {
                    String::from("Not ready to play")
                };
                ui.label(RichText::new(summary).font(ui::semibold(15.0)));
                ui.add(
                    egui::Label::new(RichText::new(&self.status).small().color(MUTED)).truncate(),
                );
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui::play_button(ui, ready && self.game.is_none()) {
                    self.play(ctx);
                }
            });
        });
    }

    fn data_files(&mut self, ui: &mut egui::Ui) {
        let ready = self.ready();
        let mut chosen: Option<PathBuf> = None;
        ui::card(ui, "Fallout 3 installation", |ui| {
            ui.horizontal(|ui| {
                if ready {
                    ui::tag(ui, "FOUND", GOOD);
                    ui.label(edition(&self.config.plugins));
                } else {
                    ui::tag(ui, "NOT FOUND", BAD);
                    ui.label("Choose the Data folder of your Fallout 3 (the one with Fallout3.esm).");
                }
            });
            ui.add_space(2.0);
            ui::path_field(ui, self.config.data_dir.as_deref(), "No folder chosen");
            ui.add_space(2.0);
            ui.horizontal(|ui| {
                if ui.add(ui::icon_button(ui::FOLDER_OPEN, "Browse…")).clicked() {
                    if let Some(dir) = rfd::FileDialog::new()
                        .set_title("Choose Fallout 3's Data folder (the one with Fallout3.esm)")
                        .pick_folder()
                    {
                        let dir = if config::is_data_dir(&dir) { dir } else { dir.join("Data") };
                        if config::is_data_dir(&dir) {
                            chosen = Some(dir);
                        } else {
                            self.warn("That folder has no Fallout3.esm.");
                        }
                    }
                }
                if ui.add(ui::icon_button(ui::SEARCH, "Find automatically")).clicked() {
                    self.found = detect::data_dirs();
                    match self.found.first() {
                        Some(d) => chosen = Some(d.clone()),
                        None => self.warn("No Fallout 3 found: use Browse."),
                    }
                }
                if let Some(d) = self.config.data_dir.as_ref().filter(|_| ready) {
                    if ui.add(ui::icon_button(ui::FOLDER, "Open folder")).clicked() {
                        open_folder(d);
                    }
                }
            });
            let others: Vec<PathBuf> = self
                .found
                .iter()
                .filter(|d| Some(*d) != self.config.data_dir.as_ref())
                .cloned()
                .collect();
            if !others.is_empty() {
                ui.add_space(4.0);
                ui::note(ui, "Other copies found on this computer:");
                for d in others {
                    if ui.link(d.display().to_string()).clicked() {
                        chosen = Some(d);
                    }
                }
            }
        });
        if let Some(d) = chosen {
            self.use_data_dir(d);
        }

        let mut changed = false;
        ui::card(ui, "Content", |ui| {
            let n = self.config.plugins.len();
            let active = self.config.plugins.iter().filter(|p| p.enabled).count();
            let sel = self.selected_plugin.filter(|&i| i < n);
            let movable = |i: usize, j: usize, list: &[config::Plugin]| {
                let master = |k: usize| list[k].name.to_ascii_lowercase().ends_with(".esm");
                !is_base(&list[i].name) && !is_base(&list[j].name) && master(i) == master(j)
            };
            ui.horizontal(|ui| {
                let up = sel.filter(|&i| i > 0 && movable(i, i - 1, &self.config.plugins));
                if ui.add_enabled(up.is_some(), ui::icon_button(ui::UP, "Up")).clicked() {
                    let i = up.unwrap_or(1);
                    self.config.plugins.swap(i, i - 1);
                    self.selected_plugin = Some(i - 1);
                    changed = true;
                }
                let down = sel.filter(|&i| i + 1 < n && movable(i, i + 1, &self.config.plugins));
                if ui.add_enabled(down.is_some(), ui::icon_button(ui::DOWN, "Down")).clicked() {
                    let i = down.unwrap_or(0);
                    self.config.plugins.swap(i, i + 1);
                    self.selected_plugin = Some(i + 1);
                    changed = true;
                }
                ui.add_space(8.0);
                if ui.button("Official only").clicked() {
                    for p in &mut self.config.plugins {
                        p.enabled = plugins::is_official(&p.name);
                    }
                    changed = true;
                }
                if ui.add(ui::icon_button(ui::REFRESH, "Rescan")).clicked() {
                    self.rescan();
                    changed = true;
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(RichText::new(format!("{active} of {n} active")).color(MUTED));
                });
            });
            ui.add_space(4.0);
            egui::Frame::new()
                .fill(ui::FIELD)
                .stroke(egui::Stroke::new(1.0, ui::CARD_EDGE))
                .corner_radius(CornerRadius::same(6))
                .inner_margin(Margin::same(4))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.spacing_mut().item_spacing.y = 0.0;
                    if self.config.plugins.is_empty() {
                        ui.add_space(10.0);
                        ui.vertical_centered(|ui| {
                            ui.label(RichText::new("No plugins yet: choose the Data folder above.").color(MUTED));
                        });
                        ui.add_space(10.0);
                    }
                    let mut index = 0u32;
                    for i in 0..self.config.plugins.len() {
                        let (rect, row) =
                            ui.allocate_exact_size(Vec2::new(ui.available_width(), 36.0), Sense::click());
                        let selected = self.selected_plugin == Some(i);
                        let fill = if selected {
                            ACCENT.gamma_multiply(0.20)
                        } else if row.hovered() {
                            ui::CARD
                        } else if i % 2 == 1 {
                            ui::ROW_ALT
                        } else {
                            Color32::TRANSPARENT
                        };
                        ui.painter().rect_filled(rect, CornerRadius::same(4), fill);
                        if selected {
                            let bar = egui::Rect::from_min_size(rect.min, Vec2::new(3.0, rect.height()));
                            ui.painter().rect_filled(bar, CornerRadius::same(1), ACCENT);
                        }
                        if row.clicked() {
                            self.selected_plugin = Some(i);
                        }
                        let mut row_ui = ui.new_child(
                            egui::UiBuilder::new()
                                .max_rect(rect.shrink2(Vec2::new(12.0, 0.0)))
                                .layout(Layout::left_to_right(Align::Center)),
                        );
                        row_ui.spacing_mut().item_spacing.x = 10.0;
                        let p = &mut self.config.plugins[i];
                        // The load index the game gives it (enabled ones only).
                        let slot = if p.enabled {
                            index += 1;
                            format!("{:02X}", index - 1)
                        } else {
                            String::from("--")
                        };
                        row_ui.add_sized(
                            [22.0, 20.0],
                            egui::Label::new(RichText::new(slot).monospace().color(MUTED)),
                        );
                        let base = is_base(&p.name);
                        row_ui.add_enabled_ui(!base, |ui| {
                            if ui::toggle(ui, &mut p.enabled).changed() {
                                changed = true;
                            }
                        });
                        row_ui.label(
                            RichText::new(&p.name)
                                .font(ui::semibold(14.5))
                                .color(if p.enabled { TEXT } else { MUTED }),
                        );
                        if let Some(f) = friendly(&p.name) {
                            row_ui.label(RichText::new(f).color(MUTED));
                        }
                        let master = p.name.to_ascii_lowercase().ends_with(".esm");
                        let official = plugins::is_official(&p.name);
                        row_ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui::tag(ui, if master { "MASTER" } else { "PLUGIN" }, MUTED);
                            if base {
                                ui::tag(ui, "BASE GAME", ACCENT_LIGHT);
                            } else if official {
                                ui::tag(ui, "ADD-ON", ACCENT_LIGHT);
                            } else {
                                ui::tag(ui, "MOD", MOD_BLUE);
                            }
                        });
                    }
                });
            ui.add_space(6.0);
            ui::note(
                ui,
                "Plugins load top to bottom; the number is the load slot the game gives each. Fallout3.esm always loads. \
                 Mod support is early: script extender plugins don't run yet.",
            );
        });
        if changed {
            // Masters stay ahead of plugins, Fallout3.esm on.
            let found: Vec<String> = self.config.plugins.iter().map(|p| p.name.clone()).collect();
            self.config.plugins = plugins::merge(&self.config.plugins, &found);
            self.save();
        }
    }

    fn display(&mut self, ui: &mut egui::Ui) {
        let before = (
            self.config.window,
            self.config.resolution,
            self.config.vsync,
            self.config.brightness,
        );
        let screen = ui.ctx().input(|i| {
            let v = i.viewport();
            v.monitor_size
                .map(|s| s * v.native_pixels_per_point.unwrap_or(1.0))
                .map(|s| (s.x.round() as u32, s.y.round() as u32))
        });
        ui::card(ui, "Window", |ui| {
            let help = match self.config.window {
                WindowMode::Windowed => "A normal window you can move and resize.",
                WindowMode::Borderless => "Covers the screen without changing its mode: quick to Alt+Tab.",
                WindowMode::Fullscreen => "The screen is given over to the game.",
            };
            ui::setting(ui, "Mode", help, |ui| {
                ui::segmented(
                    ui,
                    &mut self.config.window,
                    &[
                        (WindowMode::Windowed, "Windowed"),
                        (WindowMode::Borderless, "Borderless"),
                        (WindowMode::Fullscreen, "Fullscreen"),
                    ],
                );
            });
            ui::setting(ui, "Resolution", "The size of the game's picture.", |ui| {
                ui.horizontal(|ui| {
                    let (w, h) = self.config.resolution;
                    egui::ComboBox::from_id_salt("resolution")
                        .selected_text(format!("{w} × {h}"))
                        .width(150.0)
                        .show_ui(ui, |ui| {
                            for r in RESOLUTIONS {
                                ui.selectable_value(
                                    &mut self.config.resolution,
                                    r,
                                    format!("{} × {}", r.0, r.1),
                                );
                            }
                        });
                    if let Some((sw, sh)) = screen {
                        if ui
                            .add_enabled(
                                self.config.resolution != (sw, sh),
                                egui::Button::new(format!("Match screen ({sw} × {sh})")),
                            )
                            .clicked()
                        {
                            self.config.resolution = (sw, sh);
                        }
                    }
                });
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Custom").color(MUTED));
                    ui.add(egui::DragValue::new(&mut self.config.resolution.0).range(640..=7680));
                    ui.label("×");
                    ui.add(egui::DragValue::new(&mut self.config.resolution.1).range(480..=4320));
                });
            });
        });
        ui::card(ui, "Picture", |ui| {
            ui::setting(
                ui,
                "Vertical sync",
                "On: each frame waits for the display. Off: the newest frame shows at each refresh.",
                |ui| {
                    ui.horizontal(|ui| {
                        ui::toggle(ui, &mut self.config.vsync);
                        ui.label(if self.config.vsync { "On" } else { "Off" });
                    });
                },
            );
            ui::setting(ui, "Brightness", "The game's gamma. 1.00 is as the game was made.", |ui| {
                ui.horizontal(|ui| {
                    ui.add(
                        egui::Slider::new(&mut self.config.brightness, 0.5..=2.0).fixed_decimals(2),
                    );
                    if ui
                        .add_enabled(
                            (self.config.brightness - 1.0).abs() > 0.001,
                            egui::Button::new("Reset"),
                        )
                        .clicked()
                    {
                        self.config.brightness = 1.0;
                    }
                });
            });
        });
        if before
            != (
                self.config.window,
                self.config.resolution,
                self.config.vsync,
                self.config.brightness,
            )
        {
            self.save();
        }
    }

    fn settings(&mut self, ui: &mut egui::Ui) {
        let mut changed = false;
        ui::card(ui, "Starting", |ui| {
            let help = match self.config.start {
                Start::MainMenu => "Continue, load a save or begin a new game from the menu.",
                Start::NewGame => "Skips the menu and begins a new game at once.",
            };
            ui::setting(ui, "Start at", help, |ui| {
                changed |= ui::segmented(
                    ui,
                    &mut self.config.start,
                    &[(Start::MainMenu, "Main menu"), (Start::NewGame, "New game")],
                );
            });
            ui::setting(ui, "Movies", "The intro and the add-ons' films.", |ui| {
                ui.horizontal(|ui| {
                    let mut play = !self.config.skip_movies;
                    if ui::toggle(ui, &mut play).changed() {
                        self.config.skip_movies = !play;
                        changed = true;
                    }
                    ui.label(if play { "Play them" } else { "Skip them" });
                });
            });
        });
        ui::card(ui, "Interface", |ui| {
            ui::setting(ui, "Subtitles", "Every line spoken, not only conversations.", |ui| {
                ui.horizontal(|ui| {
                    changed |= ui::toggle(ui, &mut self.config.subtitles).changed();
                    ui.label(if self.config.subtitles { "On" } else { "Off" });
                });
            });
        });
        ui::card(ui, "Launcher", |ui| {
            ui::setting(ui, "While playing", "Keep this window open to see when the game closes.", |ui| {
                ui.horizontal(|ui| {
                    changed |= ui::toggle(ui, &mut self.config.keep_open).changed();
                    ui.label(if self.config.keep_open { "Stay open" } else { "Close the launcher" });
                });
            });
            ui::setting(ui, "Game program", "OpenFO3.exe, found beside the launcher.", |ui| {
                let exe = game_exe(&self.config);
                ui::path_field(ui, exe.as_deref(), "Not found");
                ui.horizontal(|ui| {
                    if ui.add(ui::icon_button(ui::FOLDER_OPEN, "Choose…")).clicked() {
                        if let Some(p) = rfd::FileDialog::new()
                            .add_filter("Program", &["exe"])
                            .set_title("Choose OpenFO3.exe")
                            .pick_file()
                        {
                            self.config.game_exe = Some(p);
                            changed = true;
                        }
                    }
                    if self.config.game_exe.is_some() && ui.button("Use the one beside the launcher").clicked() {
                        self.config.game_exe = None;
                        changed = true;
                    }
                });
            });
        });
        ui::card(ui, "Folders", |ui| {
            ui.horizontal_wrapped(|ui| {
                if ui.add(ui::icon_button(ui::FOLDER, "Saves")).clicked() {
                    open_folder(&config::saves_dir());
                }
                if ui.add(ui::icon_button(ui::FOLDER, "Game log")).clicked() {
                    open_folder(&config::user_dir());
                }
                if ui.add(ui::icon_button(ui::FOLDER, "Launcher settings")).clicked() {
                    open_folder(&config::config_dir());
                }
            });
            ui.add_space(2.0);
            ui::note(
                ui,
                &format!(
                    "OpenFO3 keeps its own saves in {}. The original game's saves and settings are never touched.",
                    config::saves_dir().display()
                ),
            );
        });
        if changed {
            self.save();
        }
    }

    fn import(&mut self, ui: &mut egui::Ui) {
        let found = self.imported.get_or_insert_with(import::read).clone();
        ui::card(ui, "From the original Fallout 3", |ui| {
            ui.label("Bring over your display and subtitle settings and the plugins you had active. These files are only read, never changed.");
            ui.add_space(6.0);
            for (path, what) in [
                (import::prefs_path(), "Settings"),
                (import::plugins_txt_path(), "Active plugins"),
            ] {
                ui.horizontal(|ui| {
                    if path.is_file() {
                        ui::tag(ui, "FOUND", GOOD);
                    } else {
                        ui::tag(ui, "MISSING", MUTED);
                    }
                    ui.label(RichText::new(what).font(ui::semibold(14.5)));
                    ui.add(
                        egui::Label::new(
                            RichText::new(path.display().to_string()).monospace().color(MUTED),
                        )
                        .truncate(),
                    );
                });
            }
        });
        let mut take = false;
        ui::card(ui, "What will be imported", |ui| {
            let none = || RichText::new("not set").color(MUTED);
            egui::Grid::new("imported")
                .num_columns(2)
                .spacing([28.0, 10.0])
                .show(ui, |ui| {
                    ui.label(RichText::new("Resolution").font(ui::semibold(14.5)));
                    match found.resolution {
                        Some((w, h)) => ui.label(format!("{w} × {h}")),
                        None => ui.label(none()),
                    };
                    ui.end_row();
                    ui.label(RichText::new("Window").font(ui::semibold(14.5)));
                    match found.fullscreen {
                        Some(true) => ui.label("Fullscreen"),
                        Some(false) => ui.label("Windowed"),
                        None => ui.label(none()),
                    };
                    ui.end_row();
                    ui.label(RichText::new("Subtitles").font(ui::semibold(14.5)));
                    match found.subtitles {
                        Some(s) => ui.label(if s { "On" } else { "Off" }),
                        None => ui.label(none()),
                    };
                    ui.end_row();
                    ui.label(RichText::new("Plugins").font(ui::semibold(14.5)));
                    match &found.plugins {
                        Some(list) => ui.label(format!("{} active: {}", list.len(), list.join(", "))),
                        None => ui.label(none()),
                    };
                    ui.end_row();
                });
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                let anything = found.resolution.is_some()
                    || found.fullscreen.is_some()
                    || found.subtitles.is_some()
                    || found.plugins.is_some();
                let button = egui::Button::new(ui::icon_text(ui::IMPORT, "Import these", 14.5, Color32::WHITE))
                    .fill(ACCENT)
                    .min_size(Vec2::new(150.0, 32.0));
                take = ui.add_enabled(anything, button).clicked();
                if ui.add(ui::icon_button(ui::REFRESH, "Read again")).clicked() {
                    self.imported = Some(import::read());
                }
            });
        });
        if take {
            self.take_import(&found);
        }
    }

    fn take_import(&mut self, i: &import::Imported) {
        let mut took = Vec::new();
        if let Some(r) = i.resolution {
            self.config.resolution = r;
            took.push(format!("{} × {}", r.0, r.1));
        }
        if let Some(f) = i.fullscreen {
            self.config.window = if f { WindowMode::Fullscreen } else { WindowMode::Windowed };
            took.push(String::from(if f { "fullscreen" } else { "windowed" }));
        }
        if let Some(s) = i.subtitles {
            self.config.subtitles = s;
            took.push(format!("subtitles {}", if s { "on" } else { "off" }));
        }
        if let (Some(list), Some(data)) = (&i.plugins, &self.config.data_dir) {
            let found = plugins::scan(data);
            self.config.plugins = plugins::from_list(list, &found);
            took.push(format!("{} active plugins", list.len()));
        }
        if took.is_empty() {
            self.warn("Nothing found to import.");
        } else {
            self.say(format!("Imported {}.", took.join(", ")));
        }
        self.save();
    }
}

fn about(ui: &mut egui::Ui) {
    ui::card(ui, "About", |ui| {
        ui.label(RichText::new("OpenFO3").font(ui::semibold(30.0)).color(Color32::WHITE));
        ui.horizontal(|ui| {
            ui.label(RichText::new(CREDIT).font(ui::semibold(15.0)).color(ACCENT_LIGHT));
            ui.label(RichText::new(format!("· version {VERSION}")).color(MUTED));
        });
        ui.add_space(8.0);
        ui.label("An open-source engine that plays Fallout 3 from your own copy of the game. Every rule it follows is read from the original game's data and program, so the Capital Wasteland plays as it was made.");
        ui.add_space(6.0);
        ui.label(
            RichText::new("Version 1.0 is not finished: bugs will be common. That's the nature of a first version, and every report helps.")
                .color(ACCENT_LIGHT),
        );
    });
    ui::card(ui, "Credits", |ui| {
        ui.label(RichText::new("RPGKING The Mad Modder").font(ui::semibold(15.0)));
        ui::note(ui, "OpenFO3: the engine, the Fallout 3 work and this launcher.");
        ui.add_space(6.0);
        ui.label(RichText::new("The nv-rs contributors").font(ui::semibold(15.0)));
        ui::note(ui, "OpenFO3 is built on their open-source Fallout: New Vegas engine.");
        ui.add_space(6.0);
        ui.label(RichText::new("OpenMW").font(ui::semibold(15.0)));
        ui::note(ui, "The inspiration for an open engine that plays the original game, and for this launcher.");
    });
    ui::card(ui, "Licence", |ui| {
        ui.label("GNU General Public License, version 3 or later. No game files are included: you need your own legally obtained Fallout 3.");
        ui.add_space(4.0);
        ui::note(ui, "OpenFO3 is not affiliated with or endorsed by Bethesda Softworks, ZeniMax Media or Microsoft. \"Fallout\" is their trademark, used only to say which game this works with.");
    });
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("OpenFO3 Launcher")
            .with_icon(ui::app_icon())
            .with_inner_size([1000.0, 660.0])
            .with_min_inner_size([860.0, 560.0]),
        centered: true,
        ..Default::default()
    };
    eframe::run_native(
        "OpenFO3 Launcher",
        options,
        Box::new(|cc| Ok(Box::new(Launcher::new(cc)))),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The game's command line follows the settings.
    #[test]
    fn the_game_is_started_as_set() {
        let mut c = Config::default();
        let a = game_args(&c, Path::new("D:/FO3/Data"), Path::new("C:/p.txt"));
        assert_eq!(
            a,
            [
                "D:/FO3/Data",
                "--plugins",
                "C:/p.txt",
                "--window",
                "windowed",
                "--resolution",
                "1920,1080",
                "--vsync",
                "off"
            ]
        );
        c.window = WindowMode::Borderless;
        c.vsync = true;
        c.subtitles = true;
        c.skip_movies = true;
        c.start = Start::NewGame;
        c.brightness = 1.25;
        let a = game_args(&c, Path::new("D"), Path::new("P"));
        assert!(a.windows(2).any(|w| w == ["--window", "borderless"]));
        assert!(a.windows(2).any(|w| w == ["--vsync", "on"]));
        assert!(a.windows(2).any(|w| w == ["--brightness", "1.25"]));
        for flag in ["--subtitles", "--no-movies", "--new-game"] {
            assert!(a.iter().any(|x| x == flag), "{flag}");
        }
    }

    #[test]
    fn the_edition_is_named_from_the_add_ons() {
        let p = |n: &str| config::Plugin {
            name: n.into(),
            enabled: true,
        };
        let goty: Vec<_> = plugins::OFFICIAL.iter().map(|n| p(n)).collect();
        assert_eq!(edition(&goty), "Game of the Year: all five add-ons");
        assert_eq!(edition(&[p("Fallout3.esm"), p("Mod.esp")]), "No add-ons");
        assert_eq!(edition(&[p("Fallout3.esm"), p("Zeta.esm")]), "1 add-on");
    }
}
