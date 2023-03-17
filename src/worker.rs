use std::{
    error::Error,
    sync::mpsc::{Receiver, Sender},
};

use eframe::egui::Context;

use crate::{
    db::Database,
    gui::PostThumbnail,
    message::{FromGUI, FromWorker},
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

    pub fn run(&mut self) -> Result<(), Box<dyn Error>> {
        for recieved in &self.rx {
            match recieved {
                FromGUI::SendContext(ctx) => self.ctx = Some(ctx),
                FromGUI::RequestAllPosts => {
                    let post_thumbnails = self
                        .db
                        .all()?
                        .into_iter()
                        .map(PostThumbnail::from)
                        .collect();

                    self.tx.send(FromWorker::SetPosts(post_thumbnails))?;
                }
            }

            if let Some(ctx) = &self.ctx {
                ctx.request_repaint();
            }
        }
        Ok(())
    }
}
