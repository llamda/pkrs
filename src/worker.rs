use std::{
    error::Error,
    path::PathBuf,
    sync::mpsc::{Receiver, Sender},
};

use eframe::egui::Context;

use crate::{
    db::Database,
    gui::PostThumbnail,
    message::{FromGUI, FromWorker},
    post::Post,
    search,
};

pub struct Worker {
    tx: Sender<FromWorker>,
    db: Database,
    ctx: Option<Context>,
}

impl Worker {
    pub fn create(tx: Sender<FromWorker>, rx: Receiver<FromGUI>, db: Database) {
        let mut worker = Worker { tx, db, ctx: None };

        worker.tx.send(FromWorker::RequestContext).unwrap();
        worker.run(rx).unwrap();
    }

    fn send(&self, msg: FromWorker) -> Result<(), Box<dyn Error>> {
        self.tx.send(msg)?;
        if let Some(ctx) = &self.ctx {
            ctx.request_repaint();
        }
        Ok(())
    }

    pub fn run(&mut self, rx: Receiver<FromGUI>) -> Result<(), Box<dyn Error>> {
        for received in rx {
            match received {
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

                FromGUI::RequestDroppedNewPosts(dropped) => {
                    let paths = dropped.into_iter().filter_map(|p| p.path).collect();
                    self.create_posts(paths)?;
                }

                FromGUI::RequestPickedNewPosts(picked) => {
                    self.create_posts(picked)?;
                }

                FromGUI::SetSelected(selected) => {
                    self.send(FromWorker::SetSelected(selected))?;
                }
                FromGUI::RemoveTag(post_id, tag) => {
                    let tag_id = self.db.get_tag_id(&tag)?;
                    self.db.remove_tagging(post_id, tag_id)?;
                }
                FromGUI::AddTag(post_id, tag) => {
                    let tag_id = self.db.get_or_create_tag(&tag)?;
                    self.db.insert_tagging(post_id, tag_id)?;
                }
                FromGUI::Search(query) => {
                    let search: Vec<String> = query.split(' ').map(|s| s.to_owned()).collect();
                    let posts = search::new(search, &mut self.db)?
                        .into_iter()
                        .map(PostThumbnail::from)
                        .collect();
                    self.send(FromWorker::SetPosts(posts))?;
                }
            }
        }
        Ok(())
    }

    fn create_posts(&mut self, paths: Vec<PathBuf>) -> Result<(), Box<dyn Error>> {
        self.send(FromWorker::ShowProgress(true))?;
        self.send(FromWorker::SetProgress(0.0, 100.0))?;
        self.send(FromWorker::SetProgressMessage(Some(
            "Reading...".to_string(),
        )))?;

        let paths = get_paths_recursive(paths);
        let mut current = 0.0;
        let total = paths.len() as f32;
        let mut new_posts = Vec::new();

        for path in paths {
            let status = format!("{}/{}  {}", current, total, path.display());
            self.send(FromWorker::SetProgressMessage(Some(status)))?;

            match Post::new(&path, &mut self.db) {
                Ok(post) => {
                    new_posts.push(PostThumbnail::from(post));
                }
                Err(e) => eprintln!("Failed to add post. {}", e),
            }
            current += 1.0;
            self.send(FromWorker::SetProgress(current, total))?;
        }
        new_posts.reverse();
        self.send(FromWorker::SetPosts(new_posts))?;
        self.send(FromWorker::ShowProgress(false))?;

        Ok(())
    }
}

fn get_paths_recursive(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut entries = Vec::new();

    for path in paths {
        match path.is_dir() {
            true => entries.extend(
                walkdir::WalkDir::new(path)
                    .into_iter()
                    .filter_map(|e| e.ok()),
            ),
            false => files.push(path),
        }
    }

    files.reserve_exact(entries.len());

    for entry in entries {
        let path = entry.into_path();
        if path.is_file() {
            files.push(path);
        }
    }
    files
}
