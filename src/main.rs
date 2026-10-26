use openwave::app::App;
use std::path::PathBuf;

fn main() -> eframe::Result {
    let mut app = App::new();
    let paths: Vec<PathBuf> = std::env::args_os().skip(1).map(PathBuf::from).collect();

    if !paths.is_empty() {
        app.add_files(paths);
    }

    app.run()
}
