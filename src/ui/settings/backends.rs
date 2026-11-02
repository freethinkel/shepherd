//! Settings → Backend's blocks of each backend: herdr's integrations, and what herdr-core and Rex say of themselves.
use super::*;
use crate::backend::{Fact, Job, Kind};

/// One of `config.toml`'s settings as a field.
#[derive(Clone, Copy)]
pub(super) struct HcField {
    pub key: &'static str,
    title: &'static str,
    description: &'static str,
    /// What an empty field means.
    pub empty: &'static str,
}

pub(super) const HC_FIELDS: [HcField; 3] = [
    HcField { key: "shell", title: "Shell", description: "For new terminals, with its arguments.", empty: "$SHELL" },
    HcField { key: "history_mb", title: "History", description: "Scrollback kept for each live terminal, in MB.", empty: "5" },
    HcField { key: "wait_timeout", title: "Wait", description: "Seconds an agent's `hc wait` runs before it checks in.", empty: "180" },
];

/// `key`'s value as its field shows it.
pub(super) fn hc_text(config: &HerdrConfig, key: &str) -> Option<String> {
    match key {
        "shell" => config.strs(key).map(|argv| argv.join(" ")),
        _ => config.number(key).map(|n| n.to_string()),
    }
}

/// A field's `text` as `config.toml` takes it, `None` for hc's default. Checked here, as hc refuses to run on a file it
/// cannot read.
// ponytail: `shell` splits on spaces, so a path with a space goes in the file by hand
fn hc_value(key: &str, text: &str) -> Result<Option<toml_edit::Value>, String> {
    if text.is_empty() {
        return Ok(None);
    }
    if key == "shell" {
        return Ok(Some(text.split_whitespace().collect()));
    }
    match text.parse::<f64>() {
        Ok(number) if number.is_finite() && number >= 0. => Ok(Some(number.into())),
        _ => Err(format!("{text} is not a number of zero or more")),
    }
}

impl SettingsView {
    /// Asks the backend shown what it has to say, off the main thread.
    pub(super) fn load_facts(&mut self, kind: Kind, cx: &mut Context<Self>) {
        self.facts = Some(Vec::new());
        let asked = cx.background_spawn(async move { kind.facts() });
        cx.spawn(async move |this, cx| {
            let facts = asked.await;
            let _ = this.update(cx, |v, cx| {
                if v.backend == kind {
                    v.facts = Some(facts);
                    cx.notify();
                }
            });
        })
        .detach();
    }

    /// A button of a fact. What it says goes under the block, and the facts are asked again. herdr-core's jobs restart
    /// the core, which takes the window's connection with it.
    fn run_job(&mut self, kind: Kind, job: Job, cx: &mut Context<Self>) {
        self.job = Some(job.label);
        self.backend_note = None;
        let run = cx.background_spawn(async move { kind.run(job.args) });
        cx.spawn(async move |this, cx| {
            let done = run.await;
            let _ = this.update(cx, |v, cx| {
                if kind == Kind::Hc && done.is_ok() {
                    crate::shell::reconnect(cx);
                }
                v.job = None;
                v.backend_note = Some(match done {
                    Ok(said) if said.trim().is_empty() => format!("{}: done", job.label),
                    Ok(said) => said.trim().to_string(),
                    Err(e) => format!("{e:#}"),
                });
                v.facts = None;
                cx.notify();
            });
        })
        .detach();
    }

    /// Saves a field of herdr-core's, if it changed. The file is read again first, so edits made elsewhere are kept.
    pub(super) fn commit_hc(&mut self, key: &str, text: String, cx: &mut Context<Self>) {
        if text == self.hc_config.as_ref().ok().and_then(|c| hc_text(c, key)).unwrap_or_default() {
            return;
        }
        match hc_value(key, &text) {
            Ok(value) => self.edit_hc(key, value, cx),
            Err(e) => {
                self.backend_note = Some(e);
                cx.notify();
            }
        }
    }

    fn edit_hc(&mut self, key: &str, value: Option<toml_edit::Value>, cx: &mut Context<Self>) {
        let path = crate::hc::facts::config_path();
        let saved = HerdrConfig::load(&path).and_then(|mut config| {
            config.set(key, value)?;
            config.save(&path)?;
            Ok(config)
        });
        match saved {
            Ok(config) => {
                self.hc_config = Ok(config);
                self.backend_note = None;
            }
            Err(e) => self.backend_note = Some(format!("{e:#}")),
        }
        cx.notify();
    }

    fn fact_row(&self, theme: &Theme, first: bool, kind: Kind, fact: Fact, cx: &mut Context<Self>) -> Div {
        let jobs = fact.actions.iter().map(|job| {
            let job = *job;
            let label = if self.job == Some(job.label) { "…" } else { job.label };
            action(theme, SharedString::from(format!("job-{}", job.label)), label).when(self.job.is_none(), |el| {
                el.on_click(cx.listener(move |v, _: &ClickEvent, _, cx| v.run_job(kind, job, cx)))
            })
        });
        let jobs: Vec<_> = jobs.collect();
        // a Rex with no init.lua reads an empty one the same way, so there is something to show
        let show = fact.file.clone().map(|file| {
            action(theme, "show-file", "Show file").on_click(move |_: &ClickEvent, _, cx| {
                if !file.exists() {
                    let made = file.parent().map_or(Ok(()), std::fs::create_dir_all).and_then(|_| std::fs::write(&file, ""));
                    if let Err(e) = made {
                        crate::ui::toast::error(cx, format!("Couldn't make {}: {e}", file.display()));
                    }
                }
                cx.reveal_path(&file);
            })
        });
        card_row(theme, first).child(row_text(theme, fact.title, fact.detail)).child(div().flex().flex_none().gap(px(8.)).children(show).children(jobs))
    }

    /// The block of herdr-core or Rex.
    pub(super) fn backend_block(&self, theme: &Theme, kind: Kind, cx: &mut Context<Self>) -> Option<Div> {
        if !matches!(kind, Kind::Hc | Kind::Rex) {
            return None;
        }
        let facts = self.facts.clone().unwrap_or_default();
        let asking = facts.is_empty();
        let rows: Vec<Div> = facts.into_iter().enumerate().map(|(i, fact)| self.fact_row(theme, i == 0, kind, fact, cx)).collect();
        let hc = kind == Kind::Hc;
        let notes = self.hc_config.as_ref().ok().is_none_or(|c| c.bool("notes", true));
        let sync = SettingsGlobal::of(cx).settings.hc_sync;
        let settings = hc.then(|| {
            let fields = HC_FIELDS.iter().zip(&self.hc_fields).map(|(f, input)| {
                let description = format!("{} Empty is {}.", f.description, f.empty);
                card_row(theme, false).child(row_text(theme, f.title, description)).child(field(theme, input))
            });
            let fields: Vec<_> = fields.collect();
            let toggle = toggle_row(theme, false, "hc-notes", "Notes", "hc types notes to a parent agent, and tags messages between agents. Off, it types nothing on its own.", notes)
                .on_click(cx.listener(move |v, _: &ClickEvent, _, cx| v.edit_hc("notes", Some((!notes).into()), cx)));
            let share = toggle_row(theme, false, "hc-sync", "Sync with the Herdr app", "The same workspaces, tabs and splits in both. Off, Shepherd keeps its own layout and only shows the app's terminals.", sync)
                .on_click(|_: &ClickEvent, _, cx| {
                    update(cx, |s| s.hc_sync = !s.hc_sync);
                    crate::shell::reconnect(cx);
                });
            (share, fields, toggle)
        });
        let (share, fields, toggle) = match settings {
            Some((share, fields, toggle)) => (Some(share), fields, Some(toggle)),
            None => (None, Vec::new(), None),
        };
        let word = match (&self.backend_note, &self.hc_config) {
            (Some(said), _) => Some(said.clone()),
            (None, Err(why)) if hc => Some(why.clone()),
            (None, _) if hc => Some(format!("Saved to {}. hc reads it each time it needs a setting.", crate::hc::facts::config_path().display())),
            _ => None,
        };
        Some(
            div()
                .child(section_label(theme, kind.name()))
                .child(card(theme).mt_0().when(asking, |c| c.child(card_row(theme, true).child(row_text(theme, kind.name(), "Asking…")))).children(rows).children(share).children(fields).children(toggle))
                .children(word.map(|text| note(theme, text))),
        )
    }

    /// herdr's agent integrations.
    pub(super) fn integrations_block(&self, theme: &Theme, cx: &mut Context<Self>) -> gpui::AnyElement {
        match &self.integrations {
            None => note(theme, "Asking herdr…").into_any_element(),
            Some(Err(e)) => note(theme, e.clone()).text_color(theme.danger).into_any_element(),
            Some(Ok(list)) => card(theme)
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
                    card_row(theme, i == 0)
                        .min_h(px(52.))
                        .py(px(8.))
                        .when(!integration.available, |el| el.text_color(theme.text_muted))
                        .child(tile)
                        .child(row_text(theme, integration.label.clone(), status))
                        .child(
                            action(theme, SharedString::from(format!("integration-{}", integration.target)), if busy { "…" } else { button })
                                .when(!busy, |el| {
                                    el.on_click(cx.listener(move |v, _: &ClickEvent, _, cx| {
                                        v.run_integration(target.clone(), install, cx)
                                    }))
                                }),
                        )
                }))
                .into_any_element(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_field_goes_in_the_file_as_hc_reads_it() {
        assert_eq!(hc_value("shell", "").unwrap().map(|v| v.to_string()), None, "hc's default");
        assert_eq!(hc_value("shell", "nu  -l").unwrap().unwrap().to_string(), r#"["nu", "-l"]"#);
        assert_eq!(hc_value("history_mb", "2.5").unwrap().unwrap().as_float(), Some(2.5));
        assert_eq!(hc_value("wait_timeout", "180").unwrap().unwrap().as_float(), Some(180.));
        for bad in ["-1", "a lot", "inf", "NaN"] {
            assert!(hc_value("history_mb", bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn a_field_shows_what_the_file_holds() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, "shell = ['nu', '-l']\nhistory_mb = 5\nwait_timeout = 2.5\n").unwrap();
        let config = HerdrConfig::load(&path).unwrap();
        assert_eq!(hc_text(&config, "shell").as_deref(), Some("nu -l"));
        assert_eq!(hc_text(&config, "history_mb").as_deref(), Some("5"));
        assert_eq!(hc_text(&config, "wait_timeout").as_deref(), Some("2.5"));
        assert_eq!(hc_text(&config, "notes"), None);
    }
}
