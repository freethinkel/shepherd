//! Settings' window: the section list beside the shown section's page.
use super::*;

impl Render for SettingsView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        crate::ui::reap_popup(&mut self.open_select, window);
        self.sync_web_fields(window, cx);
        let theme = Theme::of(cx).clone();
        let global = SettingsGlobal::of(cx);
        let settings = global.settings.clone();
        let error = global.error.clone();
        // the number fields follow the setting when it changes elsewhere (a chevron, Cmd+=), even while typed in
        let shown = numbers(&settings);
        for (i, input) in [self.font_size.clone(), self.line_height.clone()].into_iter().enumerate() {
            if shown[i] != self.numbers_shown[i] {
                input.update(cx, |input, cx| input.set_value(shown[i].clone(), window, cx));
            }
        }
        self.numbers_shown = shown;
        let herdr = self.herdr_config.as_ref().ok().cloned().unwrap_or_default();
        // Under herdr's cards: why its config cannot be read, what the last change did, else where it is saved.
        let herdr_note = self
            .herdr_config
            .as_ref()
            .err()
            .cloned()
            .or_else(|| self.herdr_note.clone())
            .unwrap_or_else(|| format!("Saved to {}. herdr reloads it right away.", config_path().display()));

        let nav = div()
            .w(px(settings.sidebar_width))
            .h_full()
            .flex_none()
            .flex()
            .flex_col()
            .pt(px(TITLEBAR_HEIGHT))
            .bg(theme.wash(0.05))
            .border_r_1()
            .border_color(theme.border)
            // Zeron's Back: a muted tab at the top of the column, an arrow and the word.
            .child(
                div().flex_none().px(px(8.)).child(
                    crate::shell::row(&theme, false, "settings-back")
                        .text_color(theme.text_muted)
                        .child(gpui::svg().path("icons/arrow-left.svg").size(px(16.)).flex_none().text_color(theme.text_muted))
                        .child("Back")
                        .on_click(cx.listener(|_, _: &ClickEvent, _, cx| cx.emit(Close))),
                ),
            )
            .child(div().flex_1().min_h_0().flex().flex_col().gap(px(2.)).px(px(8.)).pt(px(12.)).children(Section::ALL.into_iter().map(
                |section| {
                    let selected = self.section == section;
                    crate::shell::row(&theme, selected, SharedString::from(format!("settings-{}", section.label())))
                        .text_color(if selected { theme.text } else { theme.text_muted })
                        .when(selected, |el| el.font_weight(FontWeight::MEDIUM))
                        .child(gpui::svg().path(section.icon()).size(px(16.)).flex_none().text_color(theme.text_muted))
                        .child(section.label())
                        .on_click(cx.listener(move |v, _: &ClickEvent, _, cx| {
                            if section == Section::Integrations && v.section != section {
                                v.load_integrations(cx); // installed from a terminal meanwhile
                            }
                            v.section = section;
                            v.recording = None;
                            cx.notify();
                        }))
                },
            )));

        let body = match self.section {
            Section::General => div().child(page_header(&theme, "General", "The window and what it shows.")).child(
                card(&theme)
                    .child(
                        toggle_row(&theme, true, "show-agents", "Agents", "Every agent herdr finds, at the bottom of the sidebar.", settings.show_agents).on_click(cx.listener(
                            |_, _: &ClickEvent, _, cx| update(cx, |s| s.show_agents = !s.show_agents),
                        )),
                    )
                    .child(card_row(&theme, false).child(row_text(
                        &theme,
                        "Status icons",
                        "How an agent's state shows. Dots and Symbols are herdr's styles; Animated is Zeron's, a pulse while an agent works.",
                    )))
                    .child(
                        div()
                            .mx(px(8.))
                            .mt(px(-8.))
                            .pb(px(8.))
                            .flex()
                            .flex_col()
                            .children([(StatusIcons::Dots, "Dots"), (StatusIcons::Symbols, "Symbols"), (StatusIcons::Animated, "Animated")].map(
                                |(style, label)| {
                                    status_choice(&theme, style, label, settings.status_icons == style)
                                        .on_click(move |_, _, cx| update(cx, |s| s.status_icons = style))
                                },
                            )),
                    )
                    .child(
                        toggle_row(&theme, false, "show-status-bar", "Status bar", "The branch, folder, focused agent and plan usage along the bottom.", settings.show_status_bar).on_click(cx.listener(
                            |_, _: &ClickEvent, _, cx| update(cx, |s| s.show_status_bar = !s.show_status_bar),
                        )),
                    )
                    .child(
                        toggle_row(&theme, false, "copy-on-select", "Copy on select", "A finished selection goes to the clipboard, then clears.", settings.copy_on_select).on_click(cx.listener(
                            |_, _: &ClickEvent, _, cx| update(cx, |s| s.copy_on_select = !s.copy_on_select),
                        )),
                    )
                    // Secure Keyboard Entry is macOS's
                    .when(cfg!(target_os = "macos"), |el| el.child(
                        toggle_row(&theme, false, "secure-input", "Secure keyboard entry", "While Shepherd is in front, other apps cannot read what you type.", settings.secure_input).on_click(cx.listener(
                            |_, _: &ClickEvent, _, cx| update(cx, |s| s.secure_input = !s.secure_input),
                        )),
                    )),
            ),
            Section::Terminal => div()
                .child(page_header(&theme, "Terminal", "The font every pane draws with."))
                .child(
                    card(&theme)
                        .child(
                            card_row(&theme, true)
                                .child(row_text(
                                    &theme,
                                    "Font family",
                                    "Any installed monospace font. Icons missing from it come from installed Nerd Fonts.",
                                ))
                                .child(self.font_select(&theme, &settings.font_family, cx)),
                        )
                        .child(
                            card_row(&theme, false)
                                .child(row_text(&theme, "Font size", "In points, for every pane."))
                                .child(
                                    number_field(&theme, 0, &self.font_size),
                                ),
                        )
                        .child(
                            card_row(&theme, false)
                                .child(row_text(&theme, "Line height", "Row height as a multiple of the font size."))
                                .child(
                                    number_field(&theme, 1, &self.line_height),
                                ),
                        )
                        .child(
                            toggle_row(&theme, false, "animated-cursor", "Animated cursor", "A jump of the cursor leaves a short trail behind it.", settings.animated_cursor).on_click(cx.listener(
                                |_, _: &ClickEvent, _, cx| update(cx, |s| s.animated_cursor = !s.animated_cursor),
                            )),
                        )
                        .child(
                            card_row(&theme, false)
                                .child(row_text(&theme, "Editor", if crate::terminal::pane::is_nvim(settings.editor.as_deref()) {
                                    if cfg!(target_os = "macos") { "Cmd+click on file:line opens it in Neovim, in a pane to the right." } else { "Ctrl+click on file:line opens it in Neovim, in a pane to the right." }
                                } else {
                                    if cfg!(target_os = "macos") { "Cmd+click on file:line opens it in this editor." } else { "Ctrl+click on file:line opens it in this editor." }
                                }))
                                .child(self.editor_select(&theme, settings.editor.as_deref(), cx)),
                        ),
                ),
            Section::Appearance => {
                let modes = [
                    (AppearanceMode::System, "System", "icons/monitor.svg"),
                    (AppearanceMode::Light, "Light", "icons/sun.svg"),
                    (AppearanceMode::Dark, "Dark", "icons/moon.svg"),
                ];
                let font = settings.font();
                let light = Theme::resolve(&settings.theme_light, &settings.theme_dark, AppearanceMode::Light, window.appearance(), &font);
                let dark = Theme::resolve(&settings.theme_light, &settings.theme_dark, AppearanceMode::Dark, window.appearance(), &font);
                let cards = modes.into_iter().map(|(mode, label, icon)| {
                    let selected = settings.appearance == mode;
                    let preview = match mode {
                        AppearanceMode::System => div()
                            .size_full()
                            .flex()
                            .child(div().w_1_2().h_full().overflow_hidden().child(miniature(&light)))
                            .child(div().w_1_2().h_full().overflow_hidden().child(miniature(&dark))),
                        AppearanceMode::Light => miniature(&light),
                        AppearanceMode::Dark => miniature(&dark),
                    };
                    option_card(&theme, icon, label, selected, preview)
                        .id(SharedString::from(format!("mode-{label}")))
                        .on_click(cx.listener(move |_, _: &ClickEvent, _, cx| update(cx, |s| s.appearance = mode)))
                }).collect::<Vec<_>>();
                let light_select = self.theme_select(&theme, Appearance::Light, &settings.theme_light, cx);
                let dark_select = self.theme_select(&theme, Appearance::Dark, &settings.theme_dark, cx);
                div()
                    .child(page_header(&theme, "Appearance", "Window and terminal colors."))
                    .child(section_label(&theme, "Color scheme"))
                    .child(div().flex().gap(px(16.)).children(cards))
                    .child(
                        card(&theme)
                            .child(card_row(&theme, true).child(row_text(&theme, "Light theme", "When the window is light.")).child(light_select))
                            .child(card_row(&theme, false).child(row_text(&theme, "Dark theme", "When the window is dark.")).child(dark_select)),
                    )
                    .child(section_label(&theme, "Window"))
                    .child(
                        card(&theme).mt_0().child(
                            toggle_row(
                                &theme,
                                true,
                                "translucent-window",
                                "Translucent window",
                                if cfg!(target_os = "macos") {
                                    "The desktop shows through the window, blurred."
                                } else if cfg!(target_os = "linux") {
                                    "The desktop shows through the window. Whether it is blurred depends on your compositor."
                                } else {
                                    "The desktop shows through the window."
                                },
                                settings.translucent_window,
                            )
                            .on_click(cx.listener(
                                |_, _: &ClickEvent, _, cx| update(cx, |s| s.translucent_window = !s.translucent_window),
                            )),
                        )
                        // Shepherd draws them on Linux and the BSDs; macOS and Windows have their own
                        .when(!cfg!(any(target_os = "macos", windows)), |el| {
                            el.child(
                                toggle_row(
                                    &theme,
                                    false,
                                    "window-buttons",
                                    "Window buttons",
                                    "Minimize, maximize and close at the titlebar's right end. Hide them when your window manager provides its own controls or tiles windows.",
                                    settings.window_buttons,
                                )
                                .on_click(cx.listener(
                                    |_, _: &ClickEvent, _, cx| update(cx, |s| s.window_buttons = !s.window_buttons),
                                )),
                            )
                        }),
                    )
            }
            Section::Notifications => {
                let shepherd_rows: [(&'static str, &'static str, bool, fn(&mut crate::settings::Settings)); 3] = [
                    (
                        "Notify when an agent needs you",
                        if cfg!(target_os = "macos") {
                            "A macOS notification when an agent you are not looking at waits for you or finishes."
                        } else {
                            "A desktop notification when an agent you are not looking at waits for you or finishes."
                        },
                        settings.notify_agents,
                        |s| s.notify_agents = !s.notify_agents,
                    ),
                    ("Sound", "Play a sound with each notification.", settings.notify_sound, |s| s.notify_sound = !s.notify_sound),
                    (
                        "Dock badge",
                        "Count the agents waiting or done that you have not looked at yet.",
                        settings.dock_badge,
                        |s| s.dock_badge = !s.dock_badge,
                    ),
                ];
                let delivery = herdr.str("ui.toast.delivery").unwrap_or("off").to_string();
                let delay = herdr.int("ui.toast.delay_seconds", 1);
                let sounds = herdr.bool("ui.sound.enabled", true);
                div()
                    .child(page_header(&theme, "Notifications", "When an agent waits for you or finishes."))
                    // the Dock is macOS's
                    .child(card(&theme).children(shepherd_rows.into_iter().filter(|(title, ..)| cfg!(target_os = "macos") || *title != "Dock badge").enumerate().map(|(i, (title, description, on, flip))| {
                        toggle_row(&theme, i == 0, SharedString::from(format!("notify-{i}")), title, description, on).on_click(cx.listener(
                            move |_, _: &ClickEvent, _, cx| update(cx, flip),
                        ))
                    })).when(cfg!(target_os = "macos"), |el| el.child({
                        let denied = cx.has_global::<crate::notify::Denied>();
                        let text = if denied {
                            row_text(&theme, "Turned off in macOS", "macOS blocks Shepherd's notifications, so none show up. Allow them in System Settings.")
                                .text_color(theme.danger)
                        } else {
                            row_text(&theme, "In macOS", "Banners, sounds and whether they show while you use other apps.")
                        };
                        card_row(&theme, false).child(text).child(
                            action(&theme, "notify-system-settings", "Open System Settings")
                                .on_click(|_: &ClickEvent, _, _| crate::notify::open_system_settings()),
                        )
                    })))
                    .child(section_label(&theme, "In herdr's terminal app"))
                    .child(
                        card(&theme)
                            .mt_0()
                            .child(
                                card_row(&theme, true).child(row_text(&theme, "Pop-ups", "Where herdr shows them.")).child(
                                    div().flex().gap(px(8.)).children(
                                        [("off", "Off"), ("herdr", "In herdr"), ("terminal", "Terminal"), ("system", "System")].into_iter().map(
                                            |(value, label)| {
                                                choice(&theme, SharedString::from(format!("delivery-{value}")), label, delivery == value)
                                                    .on_click(cx.listener(move |v, _: &ClickEvent, _, cx| {
                                                        v.edit_herdr("ui.toast.delivery", Some(value.into()), cx)
                                                    }))
                                            },
                                        ),
                                    ),
                                ),
                            )
                            .child(
                                card_row(&theme, false)
                                    .child(row_text(&theme, "Delay", "Seconds before a pop-up shows."))
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(4.))
                                            .child(action(&theme, "delay-down", "−").w(px(32.)).on_click(cx.listener(
                                                move |v, _: &ClickEvent, _, cx| {
                                                    v.edit_herdr("ui.toast.delay_seconds", Some((delay - 1).clamp(0, 30).into()), cx)
                                                },
                                            )))
                                            .child(div().w(px(40.)).flex().justify_center().child(format!("{delay}")))
                                            .child(action(&theme, "delay-up", "+").w(px(32.)).on_click(cx.listener(
                                                move |v, _: &ClickEvent, _, cx| {
                                                    v.edit_herdr("ui.toast.delay_seconds", Some((delay + 1).clamp(0, 30).into()), cx)
                                                },
                                            ))),
                                    ),
                            )
                            .child(
                                toggle_row(&theme, false, "herdr-sounds", "Sounds", "When an agent in a background workspace changes state.", sounds).on_click(cx.listener(move |v, _: &ClickEvent, _, cx| {
                                    v.edit_herdr("ui.sound.enabled", Some((!sounds).into()), cx)
                                })),
                            ),
                    )
                    .child(note(&theme, herdr_note))
            }
            Section::Usage => {
                use crate::settings::UsageAgent;
                let agents = UsageAgent::ALL.map(|agent| {
                    let (source, url) = match agent {
                        UsageAgent::Claude => ("From Claude Code's sign-in.", "https://claude.ai/settings/usage"),
                        UsageAgent::Codex => ("From Codex's ChatGPT sign-in.", "https://chatgpt.com/codex/settings/usage"),
                    };
                    // ponytail: no loading state; the first second after opening reads as not signed in
                    let limits = crate::ui::statusbar::usage_of(agent, cx).filter(|l| !l.is_empty());
                    card(&theme)
                        .child(
                            card_row(&theme, true)
                                .child(row_text(&theme, agent.label(), source))
                                .child(action(&theme, SharedString::from(format!("usage-web-{}", agent.key())), "Open on the web").on_click(move |_: &ClickEvent, _, cx| cx.open_url(url))),
                        )
                        .child(div().px(px(8.)).pb(px(8.)).text_size(px(12.)).map(|el| match &limits {
                            Some(limits) => el.children(limits.iter().map(|l| crate::ui::statusbar::limit_row(&theme, l))),
                            None => el.child(div().px(px(8.)).pb(px(4.)).text_color(theme.text_muted).child(format!("Sign in to {} to see its limits.", agent.label()))),
                        }))
                });
                div()
                    .child(page_header(&theme, "Usage", "Your agents' plan limits, and whose the status bar shows."))
                    .child(
                        card(&theme).child(
                            card_row(&theme, true).child(row_text(&theme, "Status bar shows", "The ring at the bar's right end, and its pop-up.")).child(
                                div().flex().gap(px(8.)).children(UsageAgent::ALL.map(|agent| {
                                    choice(&theme, SharedString::from(format!("usage-agent-{}", agent.key())), agent.label(), settings.usage_agent == agent)
                                        .on_click(move |_: &ClickEvent, _, cx| update(cx, |s| s.usage_agent = agent))
                                })),
                            ),
                        ),
                    )
                    .children(agents)
            }
            Section::Integrations => {
                let body = match &self.integrations {
                    None => note(&theme, "Asking herdr…").into_any_element(),
                    Some(Err(e)) => note(&theme, e.clone()).text_color(theme.danger).into_any_element(),
                    Some(Ok(list)) => card(&theme)
                        .children(list.iter().enumerate().map(|(i, integration)| {
                            let busy = self.integration_busy.as_deref() == Some(integration.target.as_str());
                            let (status, button) = match integration.state.as_str() {
                                "current" => ("Installed", "Uninstall"),
                                "outdated" => ("Update available", "Update"),
                                _ => ("Not installed", "Install"),
                            };
                            let status = match integration.available {
                                true => status.to_string(),
                                false => format!("{status} · `{}` is not on PATH", integration.command),
                            };
                            let (target, install) = (integration.target.clone(), button != "Uninstall");
                            // Zeron's tile: the agent's mark on a soft square, muted with the row when the agent is missing.
                            let mark = crate::ui::command_panel::agent_icon(&integration.target);
                            let ink = if integration.available { theme.text } else { theme.text_muted };
                            let tile = div()
                                .flex_none()
                                .size(px(36.))
                                .rounded(px(10.))
                                .bg(theme.hover)
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(gpui::svg().path(mark).size(px(18.)).text_color(crate::ui::command_panel::icon_ink(mark, ink)));
                            card_row(&theme, i == 0)
                                .min_h(px(52.))
                                .py(px(8.))
                                .when(!integration.available, |el| el.text_color(theme.text_muted))
                                .child(tile)
                                .child(row_text(&theme, integration.label.clone(), status))
                                .child(
                                    action(&theme, SharedString::from(format!("integration-{}", integration.target)), if busy { "…" } else { button })
                                        .when(!busy, |el| {
                                            el.on_click(cx.listener(move |v, _: &ClickEvent, _, cx| {
                                                v.run_integration(target.clone(), install, cx)
                                            }))
                                        }),
                                )
                        }))
                        .into_any_element(),
                };
                div()
                    .child(page_header(
                        &theme,
                        "Integrations",
                        "Hooks that tell herdr what each agent is doing: working, waiting for you, done.",
                    ))
                    .child(body)
                    .children(self.herdr_note.clone().map(|text| note(&theme, text)))
            }
            Section::WebPages => div()
                .child(page_header(
                    &theme,
                    "Web pages",
                    "Local tools, listed in the sidebar above the workspaces and shown in place of the tabs and panes.",
                ))
                .child(self.web_pages(&theme, cx)),
            Section::Herdr => div()
                .child(page_header(&theme, "herdr", "Where Shepherd finds herdr. Empty fields use herdr's defaults."))
                .child(
                    card(&theme)
                        .child(
                            card_row(&theme, true)
                                .child(row_text(&theme, "herdr binary", "A path to herdr, or the one on PATH."))
                                .child(field(&theme, &self.herdr_bin)),
                        )
                        .child(
                            card_row(&theme, false)
                                .child(row_text(&theme, "API socket", "Shepherd switches to a new socket when it restarts."))
                                .child(field(&theme, &self.herdr_socket)),
                        )
                        .child(
                            card_row(&theme, false)
                                .child(row_text(
                                    &theme,
                                    "Connection",
                                    self.ping.clone().unwrap_or_else(|| "Checks that herdr answers on this socket.".into()),
                                ))
                                .child(action(&theme, "ping", "Test connection").on_click(cx.listener(|v, _: &ClickEvent, _, cx| v.test_connection(cx)))),
                        ),
                )
                .child(section_label(&theme, "Server"))
                .child({
                    let resume = herdr.bool("session.resume_agents_on_restore", true);
                    let history = herdr.bool("experimental.pane_history", false);
                    let channel = herdr.str("update.channel").unwrap_or("stable").to_string();
                    let fields = [
                        ("Shell", "For new panes. Empty is $SHELL."),
                        ("New pane folder", "follow, home, current or a path. follow takes the folder of the pane it opens from."),
                        ("Worktrees folder", "Where new worktrees are checked out."),
                    ];
                    card(&theme)
                        .mt_0()
                        .children(fields.into_iter().zip(&self.herdr_fields).enumerate().map(|(i, ((title, description), input))| {
                            card_row(&theme, i == 0).child(row_text(&theme, title, description)).child(field(&theme, input))
                        }))
                        .child(
                            toggle_row(&theme, false, "herdr-resume", "Resume agents", "Reopen agent conversations after herdr restarts.", resume).on_click(cx.listener(move |v, _: &ClickEvent, _, cx| {
                                v.edit_herdr("session.resume_agents_on_restore", Some((!resume).into()), cx)
                            })),
                        )
                        .child(
                            toggle_row(&theme, false, "herdr-history", "Pane history", "Keep recent screen history across server restarts. Experimental.", history).on_click(cx.listener(move |v, _: &ClickEvent, _, cx| {
                                v.edit_herdr("experimental.pane_history", Some((!history).into()), cx)
                            })),
                        )
                        .child(
                            card_row(&theme, false).child(row_text(&theme, "Updates", "herdr's release channel.")).child(
                                div().flex().gap(px(8.)).children([("stable", "Stable"), ("preview", "Preview")].into_iter().map(|(value, label)| {
                                    choice(&theme, SharedString::from(format!("channel-{value}")), label, channel == value).on_click(cx.listener(
                                        move |v, _: &ClickEvent, _, cx| v.edit_herdr("update.channel", Some(value.into()), cx),
                                    ))
                                })),
                            ),
                        )
                })
                .child(note(&theme, herdr_note)),
            Section::About => {
                let updates = Updates::of(cx).clone();
                let button = match &updates {
                    Updates::Available(r) => {
                        let r = r.clone();
                        let label = if updater::notifies() { format!("Open {}", r.version) } else { format!("Update to {}", r.version) };
                        action(&theme, "update-app", label)
                            .on_click(move |_: &ClickEvent, _, cx| updater::install(r.clone(), cx))
                    }
                    _ => action(&theme, "check-updates", "Check for Updates").on_click(|_: &ClickEvent, _, cx| updater::check(cx, false)),
                };
                div().child(page_header(&theme, "About", "Shepherd's version. Updates come from its GitHub releases.")).child(
                    card(&theme).child(
                        card_row(&theme, true)
                            .child(row_text(&theme, format!("Shepherd {}", env!("CARGO_PKG_VERSION")), updates.line()))
                            .when(updater::checks(), |row| row.child(button.when(updates.busy(), |b| b.opacity(0.5)))),
                    ),
                )
            }
            Section::Shortcuts => {
                let bindings = effective(&settings.keys);
                div()
                    .child(
                        div()
                            .flex()
                            .items_start()
                            .child(page_header(&theme, "Shortcuts", "Click a shortcut, then press the new chord. Esc cancels, ⌫ unbinds.").flex_1())
                            .child(action(&theme, "reset-keys", "Reset to defaults").on_click(cx.listener(|v, _: &ClickEvent, _, cx| {
                                v.recording = None;
                                update(cx, |s| s.keys.clear())
                            }))),
                    )
                    .child(card(&theme).children(ACTIONS.iter().enumerate().map(|(i, (action, label))| {
                        let chord = bindings.iter().find(|(a, _)| a == action).map(|(_, k)| display_chord(k));
                        let recording = self.recording == Some(*action);
                        let action = *action;
                        let hover = theme.border_strong;
                        card_row(&theme, i == 0).min_h(px(48.)).py(px(8.)).child(div().flex_1().child(*label)).child(
                            div()
                                .id(SharedString::from(format!("record-{action}")))
                                .h(px(28.))
                                .min_w(px(64.))
                                .px(px(10.))
                                .flex()
                                .items_center()
                                .justify_center()
                                .rounded(px(6.))
                                .bg(theme.surface)
                                .border_1()
                                .font_family(theme.font_terminal.clone())
                                .text_size(px(12.))
                                .cursor_pointer()
                                .map(|el| match (recording, &chord) {
                                    (true, _) => el.border_color(theme.accent).text_color(theme.accent).child("Press keys…"),
                                    (false, Some(chord)) => el.border_color(theme.border).hover(move |s| s.border_color(hover)).child(chord.clone()),
                                    (false, None) => {
                                        el.border_color(theme.border).text_color(theme.text_faint).hover(move |s| s.border_color(hover)).child("—")
                                    }
                                })
                                .on_click(cx.listener(move |v, _: &ClickEvent, window, cx| {
                                    v.recording = Some(action);
                                    window.focus(&v.focus, cx);
                                    cx.notify();
                                })),
                        )
                    })))
            }
        };

        div()
            .id("settings")
            .track_focus(&self.focus)
            .size_full()
            .flex()
            .on_key_down(cx.listener(|v, e: &KeyDownEvent, _, cx| {
                if e.keystroke.key == "escape" && v.recording.is_none() {
                    cx.stop_propagation();
                    cx.emit(Close);
                }
            }))
            .child(nav)
            .child(
                div().id("settings-body").flex_1().min_w_0().h_full().overflow_y_scroll().child(
                    div()
                        .w_full()
                        .max_w(px(PAGE_MAX_WIDTH))
                        .mx_auto()
                        .px(px(40.))
                        .pt(px(TITLEBAR_HEIGHT + 16.))
                        .pb(px(48.))
                        .children(error.map(|e| {
                            div()
                                .mb(px(16.))
                                .px_3()
                                .py_2()
                                .rounded(px(12.))
                                .border_1()
                                .border_color(theme.danger.opacity(0.2))
                                .bg(theme.danger.opacity(0.06))
                                .text_color(theme.danger)
                                .child(e)
                        }))
                        .child(body),
                ),
            )
    }
}
