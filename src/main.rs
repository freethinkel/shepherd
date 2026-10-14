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
        // off macOS an app with no windows left has nothing to show: closing the last one quits, as GNOME expects
        if !cfg!(target_os = "macos") {
            cx.on_window_closed(|cx, _| {
                if cx.windows().is_empty() {
                    cx.quit();
                }
            })
            .detach();
        }
        shepherd::bindings::init_menu_actions(cx);
        cx.on_action(|_: &shepherd::bindings::OpenSettings, cx| shepherd::shell::Shell::toggle_settings(cx));
        use shepherd::{settings::step_font_size, ui::settings::update};
        cx.on_action(|_: &shepherd::bindings::FontSizeUp, cx| update(cx, |s| s.font_size = step_font_size(s.font_size, 1.0)));
        cx.on_action(|_: &shepherd::bindings::FontSizeDown, cx| update(cx, |s| s.font_size = step_font_size(s.font_size, -1.0)));
        cx.on_action(|_: &shepherd::bindings::ToggleSecureInput, cx| update(cx, |s| s.secure_input = !s.secure_input));
        cx.on_action(|_: &shepherd::bindings::FontSizeReset, cx| {
            update(cx, |s| s.font_size = shepherd::settings::Settings::default().font_size)
        });
        cx.set_global(shepherd::ui::settings::Tokio(handle.clone()));
        shepherd::update::start(cx);
        shepherd::notify::warn_if_denied(cx);

        shepherd::shell::Shell::open(handle.clone(), cx);
        if std::env::var_os("SHEPHERD_SETTINGS").is_some() {
            shepherd::shell::Shell::toggle_settings(cx);
        }
        // `SHEPHERD_PANEL=commands|agents|workspaces`: opens that panel once the first snapshot is likely in (screenshots).
        if let Ok(which) = std::env::var("SHEPHERD_PANEL") {
            use shepherd::ui::command_panel::Kind;
            let kind = match which.as_str() {
                "agents" => Kind::Agents,
                "workspaces" => Kind::Workspaces,
                _ => Kind::Commands,
            };
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
