fn main() {
    let rt = tokio::runtime::Runtime::new().expect("tokio");
    let handle = rt.handle().clone();
    gpui_platform::application().with_assets(shepherd::assets::Assets).run(move |cx| {
        gpui_base::init(cx);
        shepherd::ui::command_panel::init(cx);
        shepherd::assets::register_fonts(cx);
        let path = shepherd::settings::Settings::path();
        let (settings, error) = shepherd::settings::Settings::load(&path);
        shepherd::herdr::set_binary(settings.herdr_bin.clone());
        shepherd::herdr::set_socket(settings.herdr_socket.clone());
        cx.set_global(settings.theme(cx.window_appearance()));
        shepherd::bindings::rebind(cx, &settings.keys);
        cx.set_global(shepherd::settings::SettingsGlobal { settings, error, path });
        cx.on_action(|_: &shepherd::bindings::Quit, cx| cx.quit());
        cx.on_action(|_: &shepherd::bindings::OpenSettings, cx| shepherd::shell::Shell::toggle_settings(cx));
        cx.set_global(shepherd::ui::settings::Tokio(handle.clone()));

        shepherd::shell::Shell::open(handle.clone(), cx);
        if std::env::var_os("SHEPHERD_SETTINGS").is_some() {
            shepherd::shell::Shell::toggle_settings(cx);
        }
        cx.activate(true);
    });
    drop(rt);
}
