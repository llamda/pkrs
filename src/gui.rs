use std::path::Path;

use crate::{db::Database, post::Post, thumbnail::THUMBNAIL_SIZE};
use eframe::egui;
use poll_promise::Promise;

static THUMBNAIL: f32 = THUMBNAIL_SIZE as f32;
static THUMBNAIL_VEC2: [f32; 2] = [THUMBNAIL, THUMBNAIL];

impl MyApp {
    pub fn create(db: Database) -> Result<(), eframe::Error> {
        let options = eframe::NativeOptions {
            drag_and_drop_support: true,
            centered: true,
            initial_window_size: Some(egui::vec2(1280.0, 720.0)),
            ..Default::default()
        };

        let posts_db = db.all().unwrap();
        let mut posts = Vec::new();
        for post in posts_db {
            posts.push(PostThumbnail {
                post,
                texture: None,
            });
        }

        let app = MyApp { db, posts };

        eframe::run_native("window", options, Box::new(|_cc| Box::new(app)))
    }
}

pub struct MyApp {
    db: Database,
    posts: Vec<PostThumbnail>,
}

pub struct PostThumbnail {
    post: Post,
    texture: Option<Promise<egui::TextureHandle>>,
}

fn load_thumbnail(ctx: egui::Context, path: &Path) -> egui::TextureHandle {
    let image = image::io::Reader::open(path).unwrap().decode().unwrap();
    let size = [image.width() as _, image.height() as _];
    let image = egui::ColorImage::from_rgb(size, image.to_rgb8().as_flat_samples().as_slice());
    ctx.load_texture("thumbnail", image, Default::default())
}

impl PostThumbnail {
    fn ui(&mut self, ui: &mut egui::Ui, db: &Database) {
        let texture = self.texture.get_or_insert_with(|| {
            let path = self.post.get_db_thumbnail(db);
            let ctx = ui.ctx().clone();
            Promise::spawn_thread("load_thumbnail", move || load_thumbnail(ctx, &path))
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
                    opener::open(self.post.get_db_file(db)).unwrap();
                }
            }
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::SidePanel::right("post_panel")
            .resizable(true)
            .default_width(1280.0 * 0.8)
            .width_range(THUMBNAIL..=frame.info().window_info.size[0] - 200.0)
            .show(ctx, |ui| {
                ui.set_width(ui.available_width());

                let columns = (ui.available_width() / THUMBNAIL).floor() as _;
                let rows = (self.posts.len() + columns - 1) / columns;

                egui::ScrollArea::vertical()
                    .drag_to_scroll(false)
                    .show_rows(ui, THUMBNAIL, rows, |ui, row_range| {
                        ui.set_width(ui.available_width());

                        for y in row_range {
                            ui.horizontal(|ui| {
                                for x in 0..columns {
                                    let n = y * columns + x;

                                    if let Some(thumbnail) = self.posts.get_mut(n) {
                                        thumbnail.ui(ui, &self.db);
                                    }
                                }
                            });
                        }
                    });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("tag panel");
        });
    }
}
