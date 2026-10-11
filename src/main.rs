fn main() {
    let rt = tokio::runtime::Runtime::new().expect("tokio");
    let handle = rt.handle().clone();
    gpui_platform::application().with_assets(shepherd::assets::Assets).run(move |cx| {
        gpui_base::init(cx);
        shepherd::notify::set_app_icon();
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
        shepherd::bindings::init_menu_actions(cx);
        cx.on_action(|_: &shepherd::bindings::OpenSettings, cx| shepherd::shell::Shell::toggle_settings(cx));
        cx.set_global(shepherd::ui::settings::Tokio(handle.clone()));
        shepherd::update::start(cx);
        shepherd::notify::warn_if_denied(cx);

        shepherd::shell::Shell::open(handle.clone(), cx);
        if std::env::var_os("SHEPHERD_SETTINGS").is_some() {
            shepherd::shell::Shell::toggle_settings(cx);
        }
        // `SHEPHERD_PANEL=commands|agents`: opens that panel once the first snapshot is likely in (screenshots).
        if let Ok(which) = std::env::var("SHEPHERD_PANEL") {
            use shepherd::ui::command_panel::Kind;
            let kind = if which == "agents" { Kind::Agents } else { Kind::Commands };
            cx.spawn(async move |cx| {
                cx.background_executor().timer(std::time::Duration::from_secs(2)).await;
                cx.update(|cx| {
                    let Some(window) = cx.windows().into_iter().find_map(|w| w.downcast::<shepherd::shell::Shell>()) else { return };
                    let _ = window.update(cx, |shell, window, cx| shell.toggle_command_panel(kind, window, cx));
                })
            })
            .detach();
        }
        cx.activate(true);
    });
    drop(rt);
}
