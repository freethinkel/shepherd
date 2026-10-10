fn main() {
    let args: Vec<String> = std::env::args().collect();
    if let [_, cmd, pane] = args.as_slice() {
        if cmd == "attach" {
            if let Err(err) = shepherd::attach::run(pane) {
                eprintln!("shepherd attach: {err:#}");
                std::process::exit(1);
            }
            return;
        }
    }
    let rt = tokio::runtime::Runtime::new().expect("tokio");
    let handle = rt.handle().clone();
    gpui_platform::application().run(move |cx| {
        let app = shepherd::ghostty::App::init(cx);
        shepherd::shell::Shell::open(app, handle.clone(), cx);
        cx.activate(true);
    });
    drop(rt);
}
