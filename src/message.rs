use crate::gui::PostThumbnail;

pub enum FromWorker {
    RequestContext,
    SetPosts(Vec<PostThumbnail>),
    ShowProgress(bool),
    SetProgress(f32, f32),
    SetProgressMessage(Option<String>),
}
pub enum FromGUI {
    SendContext(eframe::egui::Context),
    RequestAllPosts,
    RequestCreateNewPosts(Vec<eframe::egui::DroppedFile>),
}
