use crate::UserEvent;
use tao::event_loop::EventLoopProxy;

pub struct SystemTray {
    #[cfg(target_os = "windows")]
    icon: Option<tray_icon::TrayIcon>,
}
impl SystemTray {
    pub fn new(_proxy: EventLoopProxy<UserEvent>) -> Self {
        #[cfg(target_os = "windows")]
        {
            use tray_icon::{
                menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
                Icon, MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent,
            };
            let create = || -> Result<tray_icon::TrayIcon, Box<dyn std::error::Error>> {
                let show = MenuItem::with_id(
                    "show",
                    format!("Show {}", crate::root::app_name()),
                    true,
                    None,
                );
                let new = MenuItem::with_id("new", "New note", true, None);
                let open = MenuItem::with_id("open", "Open file…", true, None);
                let quit = MenuItem::with_id("quit", "Quit", true, None);
                let separator = PredefinedMenuItem::separator();
                let menu = Menu::with_items(&[&show, &new, &open, &separator, &quit])?;
                Ok(TrayIconBuilder::new()
                    .with_tooltip(crate::root::app_name())
                    .with_icon(Icon::from_rgba(crate::icon::rgba(32), 32, 32)?)
                    .with_menu(Box::new(menu))
                    .with_menu_on_left_click(false)
                    .build()?)
            };
            let icon = match create() {
                Ok(icon) => Some(icon),
                Err(e) => {
                    crate::log::line(&format!("tray unavailable: {}", e));
                    None
                }
            };
            let menu_proxy = _proxy.clone();
            MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
                let _ = menu_proxy.send_event(UserEvent::Tray(event.id.0));
            }));
            TrayIconEvent::set_event_handler(Some(move |event| {
                if matches!(
                    event,
                    TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    }
                ) {
                    let _ = _proxy.send_event(UserEvent::Tray("show".into()));
                }
            }));
            Self { icon }
        }
        #[cfg(not(target_os = "windows"))]
        Self {}
    }
    pub fn available(&self) -> bool {
        #[cfg(target_os = "windows")]
        {
            self.icon.is_some()
        }
        #[cfg(not(target_os = "windows"))]
        {
            false
        }
    }
}
