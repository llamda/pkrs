use std::{
    cmp::max,
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
            focus_search: false,
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
    focus_search: bool,
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
            FromWorker::SetPosts(posts) => self.posts = posts,
            FromWorker::ShowProgress(b) => self.show_progress = b,
            FromWorker::SetProgress(current, total) => self.progress = (current, total),
            FromWorker::SetProgressMessage(message) => self.progress_message = message,
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
                                    let n = self.posts.len() as i64 - (columns * y + x + 1) as i64;

                                    if let Some(thumbnail) = self.posts.get_mut(n as usize) {
                                        thumbnail.ui(ui, &self.config);
                                    }
                                }
                            });
                        }
                    });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            let search_bar = egui::TextEdit::singleline(&mut self.search)
                .hint_text("Search")
                .desired_width(f32::INFINITY)
                .show(ui)
                .response;

            ui.input(|i| {
                if i.key_pressed(Key::I) && !search_bar.has_focus() {
                    self.focus_search = true;
                }

                if i.key_pressed(Key::Enter) && search_bar.lost_focus() {
                    println!("Search: {}", self.search);
                }
            });

            if self.focus_search {
                search_bar.request_focus();
                self.focus_search = false;
            }

            ui.label("tag panel");
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
    texture: Option<Promise<egui::TextureHandle>>,
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
    fn load_thumbnail(ctx: egui::Context, path: PathBuf) -> egui::TextureHandle {
        let image = image::io::Reader::open(path).unwrap().decode().unwrap();
        let size = [image.width() as _, image.height() as _];
        let image = egui::ColorImage::from_rgb(size, image.to_rgb8().as_flat_samples().as_slice());
        ctx.load_texture("thumbnail", image, Default::default())
    }

    fn ui(&mut self, ui: &mut egui::Ui, config: &Config) {
        let texture = self.texture.get_or_insert_with(|| {
            let ctx = ui.ctx().clone();
            let path = self.post.get_db_thumbnail(config);
            Promise::spawn_thread("load_thumbnail", move || Self::load_thumbnail(ctx, path))
        });

        match texture.ready() {
            None => {
                ui.add_sized(THUMBNAIL_VEC2, egui::Spinner::new());
            }
            Some(texture) => {
                let size = texture.size_vec2();

                let button = ui
                    .add_sized(
                        THUMBNAIL_VEC2,
                        egui::ImageButton::new(texture, size).frame(false),
                    )
                    .on_hover_text_at_pointer(format!(
                        "#{} {}",
                        &self.post.id, &self.post.original_name
                    ));

                if button.double_clicked() {
                    opener::open(self.post.get_db_file(config)).unwrap();
                }
            }
        }
    }
}
