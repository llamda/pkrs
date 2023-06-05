use std::{
    cmp::max,
    collections::HashSet,
    error::Error,
    ops::RangeInclusive,
    path::PathBuf,
    sync::mpsc::{self, Receiver, Sender},
    thread,
};

use crate::{
    config::Config,
    db::Database,
    message::{FromGUI, FromWorker},
    post::Post,
    thumbnail,
    worker::Worker,
};
use eframe::egui::{self, Context, Key, Visuals};
use egui_extras::{Column, TableBuilder};
use poll_promise::Promise;

static THUMBNAIL_SIZE: f32 = thumbnail::THUMBNAIL_SIZE as f32;
static THUMBNAIL_VEC2: [f32; 2] = [THUMBNAIL_SIZE, THUMBNAIL_SIZE];

pub fn run(db: Database) -> Result<(), eframe::Error> {
    let (from_worker, to_gui) = mpsc::channel::<FromWorker>();
    let (from_gui, to_worker) = mpsc::channel::<FromGUI>();
    let config = db.config.clone();

    thread::spawn(move || Worker::create(from_worker, to_worker, db));
    App::create(from_gui, to_gui, config)
}

impl App {
    fn create(
        tx: Sender<FromGUI>,
        rx: Receiver<FromWorker>,
        config: Config,
    ) -> Result<(), eframe::Error> {
        let app = App {
            tx,
            rx,
            config,
            posts: vec![],
            progress: (0.0, 0.0),
            show_progress: false,
            progress_message: None,
            search: String::new(),
            selected: None,
            tag_editor: None,
            focus_search: false,
            focus_editor: false,
            settings: Default::default(),
        };

        let options = eframe::NativeOptions {
            drag_and_drop_support: true,
            centered: !app.settings.fullscreen,
            fullscreen: app.settings.fullscreen,
            initial_window_size: Some(egui::vec2(
                app.settings.window_size.0,
                app.settings.window_size.1,
            )),
            ..Default::default()
        };

        app.tx.send(FromGUI::RequestAllPosts).unwrap();
        eframe::run_native("window", options, Box::new(|_cc| Box::new(app)))
    }
}

pub struct App {
    tx: Sender<FromGUI>,
    rx: Receiver<FromWorker>,
    config: Config,
    posts: Vec<PostThumbnail>,
    progress: (f32, f32),
    show_progress: bool,
    progress_message: Option<String>,
    search: String,
    selected: Option<usize>,
    tag_editor: Option<String>,
    focus_search: bool,
    focus_editor: bool,
    settings: AppSettings,
}

struct AppSettings {
    window_size: (f32, f32),
    main_panel_width: f32,
    fullscreen: bool,
    dark_mode: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        let width = 1280.0;
        let height = 720.0;
        Self {
            window_size: (width, height),
            main_panel_width: width * 0.8,
            fullscreen: false,
            dark_mode: true,
        }
    }
}

impl App {
    fn read_channel(&mut self, ctx: &egui::Context) -> Result<(), Box<dyn Error>> {
        match self.rx.try_recv()? {
            FromWorker::RequestContext => self.tx.send(FromGUI::SendContext(ctx.clone()))?,
            FromWorker::SetPosts(posts) => {
                self.posts = posts;
                self.selected = None;
                self.tag_editor = None;
            }
            FromWorker::ShowProgress(b) => self.show_progress = b,
            FromWorker::SetProgress(current, total) => self.progress = (current, total),
            FromWorker::SetProgressMessage(message) => self.progress_message = message,
            FromWorker::SetSelected(selected) => {
                self.selected = selected;
                self.tag_editor = None;
            }
        };

        Ok(())
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        let _ = self.read_channel(ctx);

        if self.show_progress {
            egui::TopBottomPanel::bottom("loading_bar").show(ctx, |ui| {
                let mut progress =
                    egui::ProgressBar::new(self.progress.0 / self.progress.1).show_percentage();

                if let Some(msg) = &self.progress_message {
                    progress = progress.text(msg);
                }
                ui.add(progress);
            });
        }

        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.set_height(24.0);

                ui.menu_button("File", |ui| {
                    if ui.button("Open File...").clicked() {
                        if let Some(paths) = rfd::FileDialog::new().pick_files() {
                            self.tx.send(FromGUI::RequestPickedNewPosts(paths)).unwrap();
                        }
                        ui.close_menu();
                    }
                    if ui.button("Open Folder...").clicked() {
                        if let Some(paths) = rfd::FileDialog::new().pick_folders() {
                            self.tx.send(FromGUI::RequestPickedNewPosts(paths)).unwrap();
                        }
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Exit").clicked() {
                        frame.close();
                    }
                });

                ui.menu_button("View", |ui| {
                    if ui.button("All Posts").clicked() {
                        self.tx.send(FromGUI::RequestAllPosts).unwrap();
                        ui.close_menu();
                    }

                    ui.separator();
                    if ui.button("Toggle Full Sceeen (F11)").clicked() {
                        self.toggle_fullscreen(frame);
                        ui.close_menu();
                    }

                    let (message, visuals) = match self.settings.dark_mode {
                        true => ("Light", Visuals::light()),
                        false => ("Dark", Visuals::dark()),
                    };
                    if ui.button(format!("Enable {} Mode", message)).clicked() {
                        ctx.set_visuals(visuals);
                        self.settings.dark_mode = !self.settings.dark_mode;
                    }
                });
            });
        });

        let panel_width = match self.changed_size(ctx) {
            true => self.scaled_panel_width(),
            false => self.default_panel_width(),
        };

        egui::SidePanel::right("post_panel")
            .resizable(true)
            .default_width(self.settings.main_panel_width)
            .width_range(panel_width)
            .show(ctx, |ui| {
                ui.set_width(ui.available_width());
                self.settings.main_panel_width = self.settings.window_size.0 - ui.available_width();

                let columns = max((ui.available_width() / THUMBNAIL_SIZE).floor() as _, 1);
                let rows = (self.posts.len() + columns - 1) / columns;

                egui::ScrollArea::vertical()
                    .drag_to_scroll(false)
                    .show_rows(ui, THUMBNAIL_SIZE, rows, |ui, row_range| {
                        ui.set_width(ui.available_width());

                        for y in row_range {
                            ui.horizontal(|ui| {
                                for x in 0..columns {
                                    let n = (self.posts.len() as i64 - (columns * y + x + 1) as i64)
                                        as usize;

                                    if let Some(thumbnail) = self.posts.get_mut(n) {
                                        thumbnail.ui(ui, &self.config, n, self.tx.clone());
                                    }
                                }
                            });
                        }
                    });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            let mut editing_tags = false;
            if let Some(tag_str) = &mut self.tag_editor {
                egui::TopBottomPanel::bottom("tag_editor").show(ui.ctx(), |ui| {
                    ui.add_space(10.0);
                    let editor = ui.add(
                        egui::TextEdit::multiline(tag_str)
                            .desired_rows(1)
                            .desired_width(f32::INFINITY),
                    );
                    ui.add_space(10.0);
                    editing_tags = editor.has_focus();
                    if self.focus_editor {
                        editor.request_focus();
                        self.focus_editor = false;
                    }
                });
            }

            egui::CentralPanel::default().show(ctx, |ui| {
                let search_bar = egui::TextEdit::singleline(&mut self.search)
                    .hint_text("Search")
                    .desired_width(f32::INFINITY)
                    .show(ui)
                    .response;

                ui.input(|i| {
                    if i.key_pressed(Key::I) && !search_bar.has_focus() && !editing_tags {
                        self.focus_search = true;
                    }

                    if i.key_pressed(Key::Enter) && search_bar.lost_focus() {
                        println!("Search: {}", self.search);
                        if self.search.is_empty() {
                            self.tx.send(FromGUI::RequestAllPosts).unwrap();
                        } else {
                            self.tx.send(FromGUI::Search(self.search.clone())).unwrap();
                        }
                    }
                });

                if self.focus_search {
                    search_bar.request_focus();
                    self.focus_search = false;
                }

                if let Some(index) = self.selected {
                    if let Some(thumbnail) = self.posts.get_mut(index) {
                        let post = &mut thumbnail.post;
                        let mut tags: Vec<String> =
                            post.tags.clone().into_iter().collect::<Vec<String>>();
                        tags.sort_unstable();
                        ui.input(|i| {
                            if i.key_pressed(Key::E)
                                && self.tag_editor.is_none()
                                && !search_bar.has_focus()
                            {
                                self.tag_editor = Some(tags.join(" "));
                                self.focus_editor = true;
                            }

                            if i.key_pressed(Key::Enter) && i.modifiers.ctrl {
                                if let Some(tag_str) = &mut self.tag_editor {
                                    let mut new_tags = HashSet::new();
                                    for tag in tag_str.split(' ') {
                                        let tag: String =
                                            tag.chars().filter(|c| !c.is_whitespace()).collect();
                                        if tag.is_empty() {
                                            continue;
                                        }

                                        new_tags.insert(tag.to_string());
                                    }

                                    for tag in new_tags.symmetric_difference(&post.tags.clone()) {
                                        let tag = tag.to_owned();

                                        if post.tags.contains(&tag) {
                                            println!("Removing '{}' from {}", tag, post.id);
                                            post.tags.remove(&tag);
                                            self.tx.send(FromGUI::RemoveTag(post.id, tag)).unwrap();
                                        } else {
                                            println!("Adding '{}' from {}", tag, post.id);
                                            post.tags.insert(tag.clone());
                                            self.tx.send(FromGUI::AddTag(post.id, tag)).unwrap();
                                        }
                                    }
                                    self.tag_editor = None;
                                }
                            }
                        });

                        ui.set_width(ui.available_width());
                        TableBuilder::new(ui)
                            .max_scroll_height(f32::MAX)
                            .striped(true)
                            .resizable(false)
                            .column(Column::remainder())
                            .header(20.0, |mut header| {
                                header.col(|ui| {
                                    ui.strong("Tags");
                                });
                            })
                            .body(|mut body| {
                                for tag in tags {
                                    body.row(18.0, |mut row| {
                                        row.col(|ui| {
                                            ui.label(tag.as_str());
                                        })
                                        .1
                                        .context_menu(
                                            |ui| {
                                                if ui.button("Remove").clicked() {
                                                    println!(
                                                        "Removed tag {} from #{}",
                                                        tag, post.id
                                                    );
                                                    post.tags.remove(&tag);
                                                    self.tx
                                                        .send(FromGUI::RemoveTag(post.id, tag))
                                                        .unwrap();
                                                    ui.close_menu();
                                                }
                                            },
                                        );
                                    });
                                }
                            });
                    }
                }
            });
        });
        ctx.input(|i| {
            if !i.raw.dropped_files.is_empty() {
                let files = i.raw.dropped_files.clone();
                self.tx
                    .send(FromGUI::RequestDroppedNewPosts(files))
                    .unwrap();
            }

            if i.key_pressed(Key::F11) {
                self.toggle_fullscreen(frame);
            }
        });
    }
}

impl App {
    fn changed_size(&mut self, ctx: &Context) -> bool {
        let screen = ctx.screen_rect();
        let (old_width, old_height) = self.settings.window_size;
        if old_width != screen.width() || old_height != screen.height() {
            self.settings.window_size = (screen.width(), screen.height());
            return true;
        }
        false
    }

    fn scaled_panel_width(&self) -> RangeInclusive<f32> {
        let size = self.settings.window_size.0 - self.settings.main_panel_width;
        size..=size
    }

    fn default_panel_width(&self) -> RangeInclusive<f32> {
        THUMBNAIL_SIZE..=self.settings.window_size.0 - 200.0
    }

    fn toggle_fullscreen(&mut self, frame: &mut eframe::Frame) {
        self.settings.fullscreen = !self.settings.fullscreen;
        frame.set_fullscreen(self.settings.fullscreen);
    }
}

pub struct PostThumbnail {
    post: Post,
    texture: Option<Promise<Option<egui::TextureHandle>>>,
}

impl From<Post> for PostThumbnail {
    fn from(post: Post) -> Self {
        PostThumbnail {
            post,
            texture: None,
        }
    }
}

impl PostThumbnail {
    fn load_thumbnail(ctx: egui::Context, path: PathBuf) -> Option<egui::TextureHandle> {
        let image = image::io::Reader::open(path).ok()?.decode().ok()?;
        let size = [image.width() as _, image.height() as _];
        let image = egui::ColorImage::from_rgb(size, image.to_rgb8().as_flat_samples().as_slice());
        Some(ctx.load_texture("thumbnail", image, Default::default()))
    }

    fn ui(&mut self, ui: &mut egui::Ui, config: &Config, index: usize, tx: Sender<FromGUI>) {
        let has_thumbnail = self.texture.get_or_insert_with(|| {
            let ctx = ui.ctx().clone();
            let path = self.post.get_db_thumbnail(config);
            Promise::spawn_thread("load_thumbnail", move || Self::load_thumbnail(ctx, path))
        });

        match has_thumbnail.ready() {
            None => {
                ui.add_sized(THUMBNAIL_VEC2, egui::Spinner::new());
            }
            Some(thumbnail) => {
                let info = format!("#{} {}", &self.post.id, &self.post.original_name);
                let button = match thumbnail {
                    None => ui.add_sized(
                        THUMBNAIL_VEC2,
                        egui::Button::new(info).frame(false).wrap(true),
                    ),
                    Some(texture) => {
                        let size = texture.size_vec2();
                        ui.add_sized(
                            THUMBNAIL_VEC2,
                            egui::ImageButton::new(texture, size).frame(false),
                        )
                        .on_hover_text_at_pointer(info)
                    }
                };
                if button.double_clicked() {
                    let file = &self.post.get_db_file(config);
                    if let Err(e) = opener::open(file) {
                        eprintln!("Failed to open {:?}\n{:#?}", file, e);
                    }
                }

                if button.clicked() {
                    tx.send(FromGUI::SetSelected(Some(index))).unwrap();
                }
            }
        }
    }
}
