#[cfg(not(target_os = "linux"))]
pub use rfd::{FileDialog, MessageButtons, MessageDialog, MessageDialogResult};

#[cfg(target_os = "linux")]
mod platform {
    use gtk::prelude::*;
    use std::path::{Path, PathBuf};

    #[derive(Clone, Debug, PartialEq)]
    pub enum MessageDialogResult {
        Yes,
        No,
        Cancel,
        Custom(String),
    }
    // Preserve the same button contract as the native platform backends.
    #[allow(clippy::enum_variant_names)]
    pub enum MessageButtons {
        YesNo,
        YesNoCancel,
        YesNoCancelCustom(String, String, String),
    }
    pub struct MessageDialog {
        title: String,
        description: String,
        buttons: MessageButtons,
    }
    fn parent() -> Option<gtk::Window> {
        gtk::Window::list_toplevels()
            .into_iter()
            .filter_map(|widget| widget.downcast::<gtk::Window>().ok())
            .find(|window| window.is_active())
    }
    impl MessageDialog {
        pub fn new() -> Self {
            Self {
                title: String::new(),
                description: String::new(),
                buttons: MessageButtons::YesNoCancel,
            }
        }
        pub fn set_title(mut self, title: impl Into<String>) -> Self {
            self.title = title.into();
            self
        }
        pub fn set_description(mut self, text: impl Into<String>) -> Self {
            self.description = text.into();
            self
        }
        pub fn set_buttons(mut self, buttons: MessageButtons) -> Self {
            self.buttons = buttons;
            self
        }
        pub fn show(self) -> MessageDialogResult {
            #[cfg(feature = "smoke")]
            eprintln!(
                "DIALOG_GTK initialized={} main={} authorized={}",
                gtk::is_initialized(),
                gtk::is_initialized_main_thread(),
                crate::root::smoke_authorized(&crate::root::app_root())
            );
            if !gtk::is_initialized_main_thread() {
                return MessageDialogResult::Cancel;
            }
            let mut builder = gtk::MessageDialog::builder()
                .modal(true)
                .message_type(gtk::MessageType::Question)
                .text(&self.title)
                .secondary_text(&self.description);
            if let Some(parent) = parent() {
                builder = builder.transient_for(&parent);
            }
            // Text properties are literal; filenames never become C format strings.
            let dialog = builder.build();
            let (yes, no, cancel) = match &self.buttons {
                MessageButtons::YesNo => ("Yes", "No", None),
                MessageButtons::YesNoCancel => ("Yes", "No", Some("Cancel")),
                MessageButtons::YesNoCancelCustom(yes, no, cancel) => {
                    (yes.as_str(), no.as_str(), Some(cancel.as_str()))
                }
            };
            dialog.add_button(yes, gtk::ResponseType::Yes);
            dialog.add_button(no, gtk::ResponseType::No);
            if let Some(cancel) = cancel {
                dialog.add_button(cancel, gtk::ResponseType::Cancel);
            }
            dialog.set_default_response(if cancel.is_some() {
                gtk::ResponseType::Cancel
            } else {
                gtk::ResponseType::No
            });
            #[cfg(feature = "smoke")]
            if crate::root::smoke_authorized(&crate::root::app_root()) {
                let response = std::env::var("SMOKE_DIALOG_RESPONSE").unwrap_or_default();
                if !response.is_empty() {
                    let copy = dialog.clone();
                    gtk::glib::timeout_add_local_once(
                        std::time::Duration::from_millis(80),
                        move || {
                            if response == "close" {
                                copy.close();
                                return;
                            }
                            copy.response(match response.as_str() {
                                "yes" => gtk::ResponseType::Yes,
                                "no" => gtk::ResponseType::No,
                                _ => gtk::ResponseType::Cancel,
                            })
                        },
                    );
                }
            }
            #[cfg(feature = "smoke")]
            eprintln!(
                "DIALOG_PRESENTED literal={}",
                dialog.secondary_text().as_deref() == Some(self.description.as_str())
            );
            let response = dialog.run();
            dialog.close();
            #[cfg(feature = "smoke")]
            eprintln!("DIALOG_RESPONSE {:?}", response);
            match (response, self.buttons) {
                (gtk::ResponseType::Yes, MessageButtons::YesNoCancelCustom(yes, _, _)) => {
                    MessageDialogResult::Custom(yes)
                }
                (gtk::ResponseType::No, MessageButtons::YesNoCancelCustom(_, no, _)) => {
                    MessageDialogResult::Custom(no)
                }
                (gtk::ResponseType::Yes, _) => MessageDialogResult::Yes,
                (gtk::ResponseType::No, _) => MessageDialogResult::No,
                _ => MessageDialogResult::Cancel,
            }
        }
    }
    #[derive(Default)]
    pub struct FileDialog {
        directory: Option<PathBuf>,
        name: Option<String>,
        filters: Vec<(String, Vec<String>)>,
    }
    impl FileDialog {
        pub fn new() -> Self {
            Self::default()
        }
        pub fn set_directory(mut self, path: impl AsRef<Path>) -> Self {
            self.directory = Some(path.as_ref().into());
            self
        }
        pub fn set_file_name(mut self, name: impl Into<String>) -> Self {
            self.name = Some(name.into());
            self
        }
        pub fn add_filter(mut self, name: impl Into<String>, extensions: &[&str]) -> Self {
            self.filters.push((
                name.into(),
                extensions.iter().map(|s| s.to_string()).collect(),
            ));
            self
        }
        fn run(self, action: gtk::FileChooserAction, multiple: bool) -> Option<Vec<PathBuf>> {
            if !gtk::is_initialized_main_thread() {
                return None;
            }
            let owner = parent();
            let saving = action == gtk::FileChooserAction::Save;
            let dialog = gtk::FileChooserNative::new(
                Some(if saving {
                    "Save a copy"
                } else {
                    "Choose a file or folder"
                }),
                owner.as_ref(),
                action,
                Some(if saving { "Save" } else { "Open" }),
                Some("Cancel"),
            );
            dialog.set_modal(true);
            dialog.set_select_multiple(multiple);
            dialog.set_do_overwrite_confirmation(true);
            if let Some(path) = self.directory {
                dialog.set_current_folder(path);
            }
            if saving {
                if let Some(name) = self.name {
                    dialog.set_current_name(&name);
                }
            }
            for (name, extensions) in self.filters {
                let filter = gtk::FileFilter::new();
                filter.set_name(Some(&name));
                for extension in extensions {
                    if extension == "*" {
                        filter.add_pattern("*");
                    } else {
                        filter.add_pattern(&format!("*.{}", extension));
                        filter.add_pattern(&format!("*.{}", extension.to_uppercase()));
                    }
                }
                dialog.add_filter(filter);
            }
            let accepted = dialog.run() == gtk::ResponseType::Accept;
            let result = accepted.then(|| dialog.filenames());
            dialog.destroy();
            result.filter(|files| !files.is_empty())
        }
        pub fn pick_files(self) -> Option<Vec<PathBuf>> {
            self.run(gtk::FileChooserAction::Open, true)
        }
        pub fn pick_folder(self) -> Option<PathBuf> {
            self.run(gtk::FileChooserAction::SelectFolder, false)?
                .into_iter()
                .next()
        }
        pub fn save_file(self) -> Option<PathBuf> {
            self.run(gtk::FileChooserAction::Save, false)?
                .into_iter()
                .next()
        }
    }
}
#[cfg(target_os = "linux")]
pub use platform::{FileDialog, MessageButtons, MessageDialog, MessageDialogResult};
