use std::{
    error::Error,
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
use eframe::egui;
use poll_promise::Promise;

static THUMBNAIL_SIZE: f32 = thumbnail::THUMBNAIL_SIZE as f32;
static THUMBNAIL_VEC2: [f32; 2] = [THUMBNAIL_SIZE, THUMBNAIL_SIZE];

pub fn run(db: Database) -> Result<(), eframe::Error> {
    let (from_worker, to_gui) = mpsc::channel::<FromWorker>();
    let (from_gui, to_worker) = mpsc::channel::<FromGUI>();
    let config = db.config.clone();

    thread::spawn(move || Worker::create(from_worker, to_worker, db));
    MyApp::create(from_gui, to_gui, config)
}

impl MyApp {
    fn create(
        tx: Sender<FromGUI>,
        rx: Receiver<FromWorker>,
        config: Config,
    ) -> Result<(), eframe::Error> {
        let options = eframe::NativeOptions {
            drag_and_drop_support: true,
            centered: true,
            initial_window_size: Some(egui::vec2(1280.0, 720.0)),
            ..Default::default()
        };

        let app = MyApp {
            tx,
            rx,
            config,
            posts: vec![],
            progress: (0.0, 0.0),
            show_progress: false,
            progress_message: None,
        };

        app.tx.send(FromGUI::RequestAllPosts).unwrap();
        eframe::run_native("window", options, Box::new(|_cc| Box::new(app)))
    }
}

pub struct MyApp {
    tx: Sender<FromGUI>,
    rx: Receiver<FromWorker>,
    config: Config,
    posts: Vec<PostThumbnail>,
    progress: (f32, f32),
    show_progress: bool,
    progress_message: Option<String>,
}

impl MyApp {
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

impl eframe::App for MyApp {
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

        egui::SidePanel::right("post_panel")
            .resizable(true)
            .default_width(1280.0 * 0.8)
            .width_range(THUMBNAIL_SIZE..=frame.info().window_info.size[0] - 200.0)
            .show(ctx, |ui| {
                ui.set_width(ui.available_width());

                let columns = (ui.available_width() / THUMBNAIL_SIZE).floor() as _;
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
            ui.label("tag panel");
        });

        ctx.input(|i| {
            if !i.raw.dropped_files.is_empty() {
                let files = i.raw.dropped_files.clone();
                self.tx.send(FromGUI::RequestCreateNewPosts(files)).unwrap();
            }
        });
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
