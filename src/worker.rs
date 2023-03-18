use std::{
    error::Error,
    sync::mpsc::{Receiver, Sender},
};

use eframe::egui::Context;

use crate::{
    db::Database,
    gui::PostThumbnail,
    message::{FromGUI, FromWorker},
    post::Post,
};

pub struct Worker {
    tx: Sender<FromWorker>,
    rx: Receiver<FromGUI>,
    db: Database,
    ctx: Option<Context>,
}

impl Worker {
    pub fn create(tx: Sender<FromWorker>, rx: Receiver<FromGUI>, db: Database) {
        let mut worker = Worker {
            tx,
            rx,
            db,
            ctx: None,
        };

        worker.tx.send(FromWorker::RequestContext).unwrap();
        worker.run().unwrap();
    }

    fn send(&self, msg: FromWorker) -> Result<(), Box<dyn Error>> {
        self.tx.send(msg)?;
        if let Some(ctx) = &self.ctx {
            ctx.request_repaint();
        }
        Ok(())
    }

    pub fn run(&mut self) -> Result<(), Box<dyn Error>> {
        for recieved in &self.rx {
            match recieved {
                FromGUI::SendContext(ctx) => self.ctx = Some(ctx),

                FromGUI::RequestAllPosts => {
                    let mut posts: Vec<PostThumbnail> = self
                        .db
                        .all()?
                        .into_iter()
                        .map(PostThumbnail::from)
                        .collect();

                    posts.reverse();
                    self.send(FromWorker::SetPosts(posts))?;
                }

                FromGUI::RequestCreateNewPosts(dropped) => {
                    let mut progress = 0.0;
                    let total = dropped.len() as f32;
                    self.send(FromWorker::ShowProgress(true))?;
                    self.send(FromWorker::SetProgress(progress, total))?;

                    let mut new_posts = Vec::new();
                    for file in dropped {
                        if let Some(path) = file.path {
                            if path.is_file() {
                                let status = format!(
                                    "{}/{} {}",
                                    progress,
                                    total,
                                    path.file_name().unwrap().to_str().unwrap(),
                                );
                                self.send(FromWorker::SetProgressMessage(Some(status)))?;

                                match Post::new(&path, &mut self.db) {
                                    Ok(post) => {
                                        new_posts.push(PostThumbnail::from(post));
                                    }
                                    Err(e) => eprintln!("Failed to add post. {}", e),
                                }
                            }
                        }
                        progress += 1.0;
                        self.send(FromWorker::SetProgress(progress, total))?;
                    }

                    new_posts.reverse();
                    self.send(FromWorker::SetPosts(new_posts))?;
                    self.send(FromWorker::ShowProgress(false))?;
                }
            }
        }
        Ok(())
    }
}
