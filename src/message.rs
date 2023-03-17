use crate::gui::PostThumbnail;

pub enum FromWorker {
    RequestContext,
    SetPosts(Vec<PostThumbnail>),
}
pub enum FromGUI {
    SendContext(eframe::egui::Context),
    RequestAllPosts,
}
