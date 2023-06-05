use crate::gui::PostThumbnail;

pub enum FromWorker {
    RequestContext,
    SetPosts(Vec<PostThumbnail>),
    ShowProgress(bool),
    SetProgress(f32, f32),
    SetProgressMessage(Option<String>),
    SetSelected(Option<usize>),
}
pub enum FromGUI {
    SendContext(eframe::egui::Context),
    RequestAllPosts,
    RequestDroppedNewPosts(Vec<eframe::egui::DroppedFile>),
    RequestPickedNewPosts(Vec<std::path::PathBuf>),
    SetSelected(Option<usize>),
    RemoveTag(i64, String),
    AddTag(i64, String),
}
