use std::path::{Path, PathBuf};

use zintl_ui_desktop::*;

/// Save and open dialogs that start in a directory the test provides.
pub struct MainView {
    directory: Option<PathBuf>,
    saving: Store<bool>,
    opening: Store<bool>,
    result: Store<String>,
}

impl MainView {
    /// `directory` is where both dialogs start; the test passes an empty temporary directory.
    pub fn new(directory: Option<String>) -> Self {
        Self {
            directory: directory.map(PathBuf::from),
            saving: Store::default(),
            opening: Store::default(),
            result: Store::default(),
        }
    }

    /// Names a chosen file by its directory and file name, which do not depend on symlinks.
    fn describe(path: &Path) -> String {
        let name = |path: Option<&Path>| {
            path.and_then(Path::file_name)
                .map_or(String::new(), |name| name.to_string_lossy().into_owned())
        };
        format!("{}/{}", name(path.parent()), name(Some(path)))
    }
}

impl View for MainView {
    type Output = RenderNode;

    fn init(&mut self, cx: &mut Context<'_>) {
        self.saving = cx.store(false);
        self.opening = cx.store(false);
        self.result = cx.store("None".to_owned());
    }

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        let (saving, opening, result) = (self.saving, self.opening, self.result);
        let report = move |prefix: &'static str| {
            move |cx: &mut Context<'_>, paths: &[PathBuf]| {
                let names = paths.iter().map(|path| Self::describe(path)).collect::<Vec<_>>();
                cx.update(result, |result| *result = format!("{prefix}: {}", names.join(",")));
            }
        };
        let cancelled = move |cx: &mut Context<'_>| {
            cx.update(result, |result| *result = "Cancelled".to_owned());
        };
        let mut save = FileDialog::save()
            .file_name("route.txt")
            .bind(saving)
            .on_select(report("Saved"))
            .on_cancel(cancelled);
        let mut open = FileDialog::open()
            .allowed_extensions(["txt"])
            .bind(opening)
            .on_select(report("Opened"))
            .on_cancel(cancelled);
        if let Some(directory) = &self.directory {
            save = save.directory(directory.clone());
            open = open.directory(directory.clone());
        }
        Window::new(Rect::new(100.0, 100.0, 560.0, 300.0), "Zintl File Dialog UI Test")
            .id("main-window")
            .file_dialog(save)
            .file_dialog(open)
            .content(
                VStack::new(list![
                    Button::new("Save").id("save").on_click(move |cx| {
                        cx.update(saving, |shown| *shown = true);
                    }),
                    Button::new("Open").id("open").on_click(move |cx| {
                        cx.update(opening, |shown| *shown = true);
                    }),
                    cx.watch(saving, |shown| Text::new(format!("Saving: {shown}")).id("saving-value")),
                    cx.watch(opening, |shown| {
                        Text::new(format!("Opening: {shown}")).id("opening-value")
                    }),
                    cx.watch(result, |result| Text::new(result.clone()).id("result")),
                ])
                .spacing(10.0),
            )
    }
}
