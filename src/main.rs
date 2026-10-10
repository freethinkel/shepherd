fn main() {
    let rt = tokio::runtime::Runtime::new().expect("tokio");
    let handle = rt.handle().clone();
    gpui_platform::application().run(move |cx| {
        shepherd::assets::register_fonts(cx);
        cx.set_global(shepherd::theme::Theme::for_appearance(cx.window_appearance()));
        shepherd::shell::Shell::open(handle.clone(), cx);
        cx.activate(true);
    });
    drop(rt);
}
