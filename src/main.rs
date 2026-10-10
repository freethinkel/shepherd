fn main() {
    let rt = tokio::runtime::Runtime::new().expect("tokio");
    let handle = rt.handle().clone();
    gpui_platform::application().run(move |cx| {
        gpui_base::init(cx);
        shepherd::assets::register_fonts(cx);
        let path = shepherd::settings::Settings::path();
        let (settings, error) = shepherd::settings::Settings::load(&path);
        shepherd::herdr::set_binary(settings.herdr_bin.clone());
        cx.set_global(settings.theme(cx.window_appearance()));
        cx.set_global(shepherd::settings::SettingsGlobal { settings, error, path });
        shepherd::shell::Shell::open(handle.clone(), cx);
        cx.activate(true);
    });
    drop(rt);
}
