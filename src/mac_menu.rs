use crate::UserEvent;
use muda::{
    accelerator::{Accelerator, Code, Modifiers},
    Menu, MenuEvent, MenuItem, PredefinedMenuItem as Native, Submenu,
};
use tao::event_loop::EventLoopProxy;
pub fn create(proxy: EventLoopProxy<UserEvent>) -> Result<Menu, muda::Error> {
    let item = |id: &str, title: &str, key: Code, shift: bool| {
        MenuItem::with_id(
            id,
            title,
            true,
            Some(Accelerator::new(
                Some(if shift {
                    Modifiers::SUPER | Modifiers::SHIFT
                } else {
                    Modifiers::SUPER
                }),
                key,
            )),
        )
    };
    let about = MenuItem::with_id(
        "help",
        format!("About {}", crate::root::app_name()),
        true,
        None,
    );
    let preferences = item("options", "Settings…", Code::Comma, false);
    let quit = item(
        "quit",
        &format!("Quit {}", crate::root::app_name()),
        Code::KeyQ,
        false,
    );
    let app = Submenu::with_items(
        crate::root::app_name(),
        true,
        &[
            &about,
            &preferences,
            &Native::separator(),
            &Native::services(None),
            &Native::separator(),
            &Native::hide(None),
            &Native::hide_others(None),
            &Native::show_all(None),
            &Native::separator(),
            &quit,
        ],
    )?;
    let new = item("new", "New Note", Code::KeyN, false);
    let open = item("open", "Open…", Code::KeyO, false);
    let folder = item("folder", "Open Folder…", Code::KeyO, true);
    let save = item("save", "Save", Code::KeyS, false);
    let save_as = item("saveAs", "Save As…", Code::KeyS, true);
    let close = item("close", "Close Tab", Code::KeyW, false);
    let file = Submenu::with_items(
        "File",
        true,
        &[
            &new,
            &open,
            &folder,
            &Native::separator(),
            &save,
            &save_as,
            &Native::separator(),
            &close,
        ],
    )?;
    let find = item("find", "Find", Code::KeyF, false);
    let replace = MenuItem::with_id(
        "replace",
        "Find and Replace",
        true,
        Some(Accelerator::new(
            Some(Modifiers::SUPER | Modifiers::ALT),
            Code::KeyF,
        )),
    );
    let next = item("findNext", "Find Next", Code::KeyG, false);
    let previous = item("findPrevious", "Find Previous", Code::KeyG, true);
    let line = item("line", "Go to Line…", Code::KeyL, false);
    let edit = Submenu::with_items(
        "Edit",
        true,
        &[
            &Native::undo(None),
            &Native::redo(None),
            &Native::separator(),
            &Native::cut(None),
            &Native::copy(None),
            &Native::paste(None),
            &Native::select_all(None),
            &Native::separator(),
            &find,
            &next,
            &previous,
            &replace,
            &line,
        ],
    )?;
    let view = Submenu::with_items(
        "View",
        true,
        &[
            &item("edit", "Edit / Preview", Code::KeyE, false),
            &item("source", "Source / Rendered", Code::KeyU, false),
            &item("files", "Show / Hide Files", Code::KeyB, false),
            &Native::separator(),
            &item("zoomIn", "Zoom In", Code::Equal, false),
            &item("zoomOut", "Zoom Out", Code::Minus, false),
            &item("zoomReset", "Actual Size", Code::Digit0, false),
        ],
    )?;
    let window = Submenu::with_items(
        "Window",
        true,
        &[
            &item("minimize", "Minimize", Code::KeyM, false),
            &Native::fullscreen(None),
        ],
    )?;
    let help = Submenu::with_items(
        "Help",
        true,
        &[&MenuItem::with_id(
            "help",
            format!("{} Help", crate::root::app_name()),
            true,
            None,
        )],
    )?;
    let menu = Menu::with_items(&[&app, &file, &edit, &view, &window, &help])?;
    menu.init_for_nsapp();
    window.set_as_windows_menu_for_nsapp();
    MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
        let _ = proxy.send_event(UserEvent::MacCommand(event.id.0));
    }));
    Ok(menu)
}

pub fn document_controls(menu: &Menu, image: bool, editable: bool, copyable: bool) {
    fn visit(items: Vec<muda::MenuItemKind>, image: bool, editable: bool, copyable: bool) {
        for item in items {
            if let Some(submenu) = item.as_submenu() {
                visit(submenu.items(), image, editable, copyable);
            }
            if let Some(command) = item.as_menuitem() {
                match command.id().0.as_str() {
                    "save" | "replace" | "edit" | "line" => command.set_enabled(editable),
                    "saveAs" => command.set_enabled(copyable),
                    "find" | "findNext" | "findPrevious" | "source" => command.set_enabled(!image),
                    _ => {}
                }
            }
        }
    }
    visit(menu.items(), image, editable, copyable);
}
