//! Native app menu: File (New/Open), View (Easy/Expert/Archive) and Mode (Splats/Dots/Discs).

use tauri::menu::{AboutMetadata, Menu, MenuBuilder, MenuItem, SubmenuBuilder};
use tauri::{AppHandle, Emitter, Manager, Runtime};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    ViewEasy,
    ViewExpert,
    ViewArchive,
    Settings,
    FileNew,
    FileOpen,
    ModeSplats,
    ModeDots,
    ModeDiscs,
}

/// Maps a menu item id to a frontend action.
pub fn menu_action(id: &str) -> Option<MenuAction> {
    match id {
        "view-easy" => Some(MenuAction::ViewEasy),
        "view-expert" => Some(MenuAction::ViewExpert),
        "view-archive" => Some(MenuAction::ViewArchive),
        "view-settings" => Some(MenuAction::Settings),
        "file-new" => Some(MenuAction::FileNew),
        "file-open" => Some(MenuAction::FileOpen),
        "mode-splats" => Some(MenuAction::ModeSplats),
        "mode-dots" => Some(MenuAction::ModeDots),
        "mode-discs" => Some(MenuAction::ModeDiscs),
        _ => None,
    }
}

impl MenuAction {
    pub fn emit<R: Runtime>(self, app: &AppHandle<R>) {
        match self {
            Self::ViewEasy => {
                let _ = app.emit("menu-view", "easy");
            }
            Self::ViewExpert => {
                let _ = app.emit("menu-view", "expert");
            }
            Self::ViewArchive => {
                let _ = app.emit("menu-view", "archive");
            }
            Self::Settings => {
                let _ = app.emit("menu-settings", ());
            }
            Self::FileNew => {
                let _ = app.emit("menu-project", "new");
            }
            Self::FileOpen => {
                let _ = app.emit("menu-project", "open");
            }
            Self::ModeSplats => {
                let _ = app.emit("menu-mode", "splats");
            }
            Self::ModeDots => {
                let _ = app.emit("menu-mode", "dots");
            }
            Self::ModeDiscs => {
                let _ = app.emit("menu-mode", "discs");
            }
        }
    }
}

/// Installs the macOS global menu and forwards File/View/Mode items to the webview.
pub fn install(app: &AppHandle) -> tauri::Result<()> {
    app.set_menu(build(app)?)?;
    app.on_menu_event(|app, event| {
        if let Some(action) = menu_action(event.id().as_ref()) {
            action.emit(app);
        }
    });
    Ok(())
}

fn build<R: Runtime, M: Manager<R>>(app: &M) -> tauri::Result<Menu<R>> {
    let app_menu = SubmenuBuilder::new(app, "Simple 3DGS")
        .about(Some(AboutMetadata {
            name: Some("Simple 3DGS".into()),
            ..Default::default()
        }))
        .separator()
        .services()
        .separator()
        .hide()
        .hide_others()
        .show_all()
        .separator()
        .quit()
        .build()?;

    let file = SubmenuBuilder::new(app, "File")
        .item(&item(app, "file-new", "New Project", Some("CmdOrCtrl+N"))?)
        .item(&item(app, "file-open", "Open…", Some("CmdOrCtrl+O"))?)
        .build()?;

    let edit = SubmenuBuilder::new(app, "Edit")
        .undo()
        .redo()
        .separator()
        .cut()
        .copy()
        .paste()
        .select_all()
        .build()?;

    let view = SubmenuBuilder::new(app, "View")
        .item(&item(app, "view-easy", "Easy", Some("CmdOrCtrl+1"))?)
        .item(&item(app, "view-expert", "Expert", Some("CmdOrCtrl+2"))?)
        .item(&item(app, "view-archive", "Archive", Some("CmdOrCtrl+3"))?)
        .separator()
        .item(&item(
            app,
            "view-settings",
            "Settings…",
            Some("CmdOrCtrl+,"),
        )?)
        .separator()
        .fullscreen()
        .build()?;

    let mode = SubmenuBuilder::new(app, "Mode")
        .item(&item(
            app,
            "mode-splats",
            "Splats",
            Some("CmdOrCtrl+Shift+1"),
        )?)
        .item(&item(app, "mode-dots", "Dots", Some("CmdOrCtrl+Shift+2"))?)
        .item(&item(
            app,
            "mode-discs",
            "Discs",
            Some("CmdOrCtrl+Shift+3"),
        )?)
        .build()?;

    let window = SubmenuBuilder::new(app, "Window")
        .minimize()
        .maximize()
        .separator()
        .close_window()
        .build()?;

    MenuBuilder::new(app)
        .item(&app_menu)
        .item(&file)
        .item(&edit)
        .item(&view)
        .item(&mode)
        .item(&window)
        .build()
}

fn item<R: Runtime, M: Manager<R>>(
    app: &M,
    id: &str,
    text: &str,
    accelerator: Option<&str>,
) -> tauri::Result<MenuItem<R>> {
    MenuItem::with_id(app, id, text, true, accelerator)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_file_view_and_mode_ids() {
        assert_eq!(menu_action("view-easy"), Some(MenuAction::ViewEasy));
        assert_eq!(menu_action("view-expert"), Some(MenuAction::ViewExpert));
        assert_eq!(menu_action("view-archive"), Some(MenuAction::ViewArchive));
        assert_eq!(menu_action("view-settings"), Some(MenuAction::Settings));
        assert_eq!(menu_action("file-new"), Some(MenuAction::FileNew));
        assert_eq!(menu_action("file-open"), Some(MenuAction::FileOpen));
        assert_eq!(menu_action("mode-splats"), Some(MenuAction::ModeSplats));
        assert_eq!(menu_action("mode-dots"), Some(MenuAction::ModeDots));
        assert_eq!(menu_action("mode-discs"), Some(MenuAction::ModeDiscs));
        assert_eq!(menu_action("quit"), None);
    }
}
