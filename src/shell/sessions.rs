//! herdr sessions and saved SSH machines, Arc's spaces: a bar of them under the sidebar, a two-finger swipe to the
//! next or last, and the dialogs that make and delete them.
use super::*;
use crate::herdr::{Machine, SessionInfo};
use gpui::{ScrollDelta, ScrollWheelEvent};

/// A session here or a machine elsewhere.
#[derive(Clone)]
pub(super) enum Place {
    Session(SessionInfo),
    Machine(Machine),
}

impl Place {
    /// Its name and icon in Shepherd: the ones given in Edit, else herdr's name and no icon.
    pub(super) fn look(&self, cx: &App) -> (String, Option<String>) {
        let look = SettingsGlobal::of(cx).settings.place_looks.get(&self.key()).cloned().unwrap_or_default();
        let herdr = match self {
            Place::Session(s) => s.name.clone(),
            Place::Machine(m) => m.label.clone(),
        };
        (look.name.filter(|n| !n.is_empty()).unwrap_or(herdr), look.icon.filter(|i| !i.is_empty()))
    }

    /// herdr's default session keeps itself: it can be edited, not deleted.
    pub(super) fn deletable(&self) -> bool {
        !matches!(self, Place::Session(s) if s.default)
    }

    /// The socket its snapshot is read from: a session's own, a machine's relay.
    fn socket(&self, rt: &tokio::runtime::Handle) -> Option<String> {
        match self {
            Place::Session(s) => s.running.then(|| s.socket_path.clone()),
            Place::Machine(m) => crate::herdr::relay(m, rt).ok().map(|p| p.to_string_lossy().into_owned()),
        }
    }
    /// Where its last snapshot is kept while another place is shown, and its look in the settings.
    pub(super) fn key(&self) -> String {
        match self {
            Place::Session(s) => s.socket_path.clone(),
            Place::Machine(m) => format!("machine:{}", m.id),
        }
    }
}

/// A horizontal two-finger swipe over the sidebar, from its first event to its last.
#[derive(Default)]
pub(super) struct Swipe {
    /// Decided by the gesture's first clear movement: true across, false down (the list scrolls instead).
    across: Option<bool>,
    /// How far the fingers have moved across, in px.
    travel: f32,
}

/// Past this the list goes to the next place when the fingers lift; short of it, it springs back.
const SWIPE_COMMIT: f32 = 60.;

/// Edit a place's name and icon.
pub(super) struct PlaceEdit {
    pub(super) place: Place,
    pub(super) name: Entity<InputState>,
    /// `icon:<name>` of a Solar Linear icon, an emoji, or `None` for the name's first letter.
    pub(super) icon: Option<String>,
    /// The picker shows the emoji rather than the drawn icons.
    pub(super) emoji: bool,
    /// Narrows the icons or emoji by name.
    pub(super) search: Entity<InputState>,
    /// The grid's scroll, which only lays out the rows in view.
    pub(super) emoji_scroll: gpui::UniformListScrollHandle,
    _events: Vec<Subscription>,
}

/// A tile of the icon picker: the name's first letter (no icon), or a Solar icon by name.
#[derive(Clone, Copy)]
pub(super) enum IconTile {
    Letter,
    Solar(&'static str),
}

/// The icon picker's rows: the letter, then every Solar Linear icon whose name holds `query` (`-` read as a space).
pub(super) fn icon_rows(query: &str) -> Vec<Vec<IconTile>> {
    let query = query.trim().to_lowercase();
    let tiles: Vec<IconTile> = query
        .is_empty()
        .then_some(IconTile::Letter)
        .into_iter()
        .chain(crate::assets::solar_names().filter(|n| query.is_empty() || n.replace('-', " ").contains(&query) || n.contains(&query)).map(IconTile::Solar))
        .collect();
    tiles.chunks(EMOJI_PER_ROW).map(<[IconTile]>::to_vec).collect()
}

/// Columns of the picker's grid; its tiles stretch to share the dialog's width.
pub(super) const EMOJI_PER_ROW: usize = 8;

/// A row of the emoji grid: a group's heading, or up to [`EMOJI_PER_ROW`] emoji.
pub(super) enum EmojiRow {
    Heading(&'static str),
    Emoji(Vec<&'static str>),
}

/// Every emoji (in its default skin tone) whose name or shortcode holds `query`, grouped as Unicode groups them.
pub(super) fn emoji_rows(query: &str) -> Vec<EmojiRow> {
    let query = query.trim().to_lowercase();
    let matches = |e: &emojis::Emoji| query.is_empty() || e.name().contains(&query) || e.shortcodes().any(|c| c.contains(&query));
    let mut rows = Vec::new();
    for group in emojis::Group::iter() {
        let found: Vec<&'static str> = group
            .emojis()
            .filter(|e| e.skin_tone().is_none_or(|t| t == emojis::SkinTone::Default))
            .filter(|e| matches(e))
            .map(|e| e.as_str())
            .collect();
        if found.is_empty() {
            continue;
        }
        rows.push(EmojiRow::Heading(group_name(group)));
        rows.extend(found.chunks(EMOJI_PER_ROW).map(|chunk| EmojiRow::Emoji(chunk.to_vec())));
    }
    rows
}

/// A group's heading in the emoji grid, one row tall, its name on the row's floor.
pub(super) fn emoji_heading(name: &'static str, color: gpui::Hsla) -> gpui::Div {
    div().h(px(36.)).flex().items_end().pb(px(4.)).text_size(px(11.)).text_color(color).child(name)
}

/// The emoji grid's sticky heading: the top row's group stays at the list's top on the dialog's card, and the next
/// group's heading pushes it up as it arrives.
pub(super) struct StickyEmojiHeading {
    pub(super) headings: Vec<(usize, &'static str)>,
    pub(super) bg: gpui::Hsla,
    pub(super) color: gpui::Hsla,
}

impl gpui::UniformListDecoration for StickyEmojiHeading {
    fn compute(
        &self,
        visible: std::ops::Range<usize>,
        _: gpui::Bounds<Pixels>,
        scroll: gpui::Point<Pixels>,
        row: Pixels,
        _: usize,
        _: &mut Window,
        _: &mut App,
    ) -> gpui::AnyElement {
        let Some(&(_, name)) = self.headings.iter().rev().find(|(i, _)| *i <= visible.start) else {
            return div().into_any_element();
        };
        // the next heading's top within the view, once it's under this one
        let push = self.headings.iter().find(|(i, _)| *i > visible.start).map_or(px(0.), |(i, _)| (row * *i as f32 + scroll.y - row).min(px(0.)));
        // laid out from the content's top (as a root, where a margin is dropped): undo the scroll to stay put
        div().w_full().pt(-scroll.y + push).child(emoji_heading(name, self.color).bg(self.bg)).into_any_element()
    }
}

fn group_name(group: emojis::Group) -> &'static str {
    use emojis::Group::*;
    match group {
        SmileysAndEmotion => "Smileys",
        PeopleAndBody => "People",
        AnimalsAndNature => "Nature",
        FoodAndDrink => "Food",
        TravelAndPlaces => "Places",
        Activities => "Activities",
        Objects => "Objects",
        Symbols => "Symbols",
        Flags => "Flags",
    }
}

pub(super) struct SessionDialog {
    pub(super) name: Entity<InputState>,
    /// The SSH host of a remote session; shown while `remote` is on.
    pub(super) host: Entity<InputState>,
    pub(super) remote: bool,
    /// Bumped on every flip of `remote`: the host field's slide restarts.
    pub(super) flips: u64,
    _events: Vec<Subscription>,
}

impl Shell {
    /// At launch, the machine the settings name gets its relay back, on the socket the settings already hold.
    pub(super) fn resume_machine(&mut self, cx: &App) {
        let Some(id) = SettingsGlobal::of(cx).settings.herdr_machine.clone() else { return };
        // ponytail: `herdr machine list` only reads herdr's catalog, so launch waits on it briefly
        let Some(machine) = crate::herdr::machines().ok().and_then(|l| l.into_iter().find(|m| m.id == id)) else { return };
        if crate::herdr::relay(&machine, &self.rt).is_ok() {
            crate::herdr::show_machine(Some(machine.target.clone()));
            self.machine_label = Some(machine.label);
        }
    }

    /// The name of the place shown: its name in Shepherd, else the machine's label or the session's name, or `custom`
    /// for a socket of one's own.
    pub(super) fn place_label(&self, cx: &App) -> String {
        match self.places.iter().find(|p| self.places_shown(p)) {
            Some(place) => place.look(cx).0,
            None => self.machine_label.clone().or_else(crate::herdr::current_session).unwrap_or_else(|| "custom".into()),
        }
    }

    /// Reads the snapshots of the places on either side, so a swipe shows them beside the list.
    pub(super) fn prefetch_neighbours(&mut self, cx: &mut Context<Self>) {
        for travel in [-1., 1.] {
            let Some(place) = self.neighbour(travel) else { continue };
            let Some(socket) = place.socket(&self.rt) else { continue };
            let (key, client) = (place.key(), Client::at(socket.into()));
            let call = self.rt.spawn(async move { client.call("session.snapshot", json!({})).await });
            cx.spawn(async move |this, cx| {
                let Ok(Ok(result)) = call.await else { return };
                let Ok(snapshot) = Snapshot::from_result(&result) else { return };
                let _ = this.update(cx, |s, cx| {
                    s.snapshots.insert(key, snapshot);
                    cx.notify();
                });
            })
            .detach();
        }
    }

    pub(super) fn show_place_emoji(&mut self, emoji: bool, cx: &mut Context<Self>) {
        if let Some(edit) = self.place_edit.as_mut() {
            edit.value.emoji = emoji;
            edit.value.emoji_scroll = Default::default(); // each tab opens at its top
            cx.notify();
        }
    }

    pub(super) fn pick_place_icon(&mut self, icon: Option<String>, cx: &mut Context<Self>) {
        if let Some(edit) = self.place_edit.as_mut() {
            edit.value.icon = icon;
            cx.notify();
        }
    }

    pub(super) fn open_place_menu(&mut self, place: Place, at: gpui::Point<Pixels>, cx: &mut Context<Self>) {
        self.place_menu = Some(crate::ui::Popup::new((place, at)));
        cx.notify();
    }

    pub(super) fn open_place_edit(&mut self, place: Place, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(m) = self.place_menu.as_mut() {
            m.close()
        }
        let theme = Theme::of(cx).clone();
        let look = SettingsGlobal::of(cx).settings.place_looks.get(&place.key()).cloned().unwrap_or_default();
        let (shown_name, _) = place.look(cx);
        let mut field = |placeholder: &'static str, value: String, cx: &mut Context<Self>| {
            cx.new(|cx| {
                let mut input = InputState::new(window, cx).placeholder(placeholder).default_value(value);
                input.set_editor_style(crate::ui::input_style(&theme));
                input
            })
        };
        let name = field("Name", shown_name, cx);
        let search = field("Search", String::new(), cx);
        name.update(cx, |input, cx| {
            input.focus(window, cx);
            input.select_all(window, cx);
        });
        let enter = |shell: &mut Shell, _: &Entity<InputState>, event: &InputEvent, _: &mut Window, cx: &mut Context<Shell>| {
            if let InputEvent::PressEnter { .. } = event {
                shell.finish_place_edit(true, cx);
            }
        };
        let refilter = |_: &mut Shell, _: &Entity<InputState>, event: &InputEvent, _: &mut Window, cx: &mut Context<Shell>| {
            if let InputEvent::Change = event {
                cx.notify();
            }
        };
        let events = vec![cx.subscribe_in(&name, window, enter), cx.subscribe_in(&search, window, refilter)];
        self.place_edit = Some(crate::ui::Popup::new(PlaceEdit {
            place,
            name,
            emoji: look.icon.as_deref().is_some_and(|i| !i.starts_with("icon:")),
            icon: look.icon,
            search,
            emoji_scroll: Default::default(),
            _events: events,
        }));
        cx.notify();
    }

    /// Saves the name and icon in the settings; a machine's name goes to herdr too, as its label.
    pub(super) fn finish_place_edit(&mut self, save: bool, cx: &mut Context<Self>) {
        let Some(edit) = self.place_edit.as_mut().filter(|e| e.is_open()) else { return };
        edit.close();
        self.focused = None;
        if save {
            let place = edit.value.place.clone();
            let name = edit.value.name.read(cx).value().trim().to_string();
            let icon = edit.value.icon.clone().unwrap_or_default();
            let herdr = match &place {
                Place::Session(s) => s.name.clone(),
                Place::Machine(m) => m.label.clone(),
            };
            let look = crate::settings::PlaceLook { name: (!name.is_empty() && name != herdr).then(|| name.clone()), icon: (!icon.is_empty()).then_some(icon) };
            let key = place.key();
            save_from_sidebar(cx, move |s| {
                if look == Default::default() {
                    s.place_looks.remove(&key);
                } else {
                    s.place_looks.insert(key, look);
                }
            });
            if let Place::Machine(m) = place.clone() && !name.is_empty() && name != herdr {
                let task = cx.background_spawn(async move { crate::herdr::rename_machine(&m.id, &name) });
                cx.spawn(async move |this, cx| {
                    let done = task.await;
                    let _ = this.update(cx, |s, cx| {
                        if let Err(e) = done {
                            crate::ui::toast::error(cx, format!("{e:#}"));
                        }
                        s.refresh_places(cx);
                    });
                })
                .detach();
            }
        }
        cx.notify();
    }

    /// Asks herdr for its sessions and enabled machines off the main thread, for the bar.
    pub(super) fn refresh_places(&mut self, cx: &mut Context<Self>) {
        let list = cx.background_spawn(async {
            let sessions = crate::herdr::sessions()?;
            // an older herdr without machines lists none
            let machines = crate::herdr::machines().unwrap_or_default();
            anyhow::Ok(
                sessions.into_iter().map(Place::Session).chain(machines.into_iter().filter(|m| m.enabled).map(Place::Machine)).collect::<Vec<_>>(),
            )
        });
        cx.spawn(async move |this, cx| {
            let Ok(places) = list.await else { return };
            let _ = this.update(cx, |s, cx| {
                s.places = places;
                s.prefetch_neighbours(cx);
                cx.notify();
            });
        })
        .detach();
    }

    pub(super) fn switch_to(&mut self, place: Place, cx: &mut Context<Self>) {
        match place {
            Place::Session(s) => self.switch_session(s, cx),
            Place::Machine(m) => self.switch_machine(m, cx),
        }
    }

    /// One event of a scroll over the sidebar: a horizontal two-finger swipe drags the workspace list toward the
    /// neighbouring place and, when the fingers lift far enough, moves there. Momentum after the lift is ignored.
    pub(super) fn sidebar_scroll(&mut self, event: &ScrollWheelEvent, cx: &mut Context<Self>) {
        use gpui::TouchPhase;
        let delta = match event.delta {
            ScrollDelta::Pixels(d) => (f32::from(d.x), f32::from(d.y)),
            ScrollDelta::Lines(d) => (d.x * 20., d.y * 20.),
        };
        match event.touch_phase {
            TouchPhase::Started => {
                if self.bar_bounds.get().is_some_and(|b| b.contains(&event.position)) {
                    return; // the bar scrolls itself
                }
                self.swipe = Some(Swipe::default());
                self.prefetch_neighbours(cx);
            }
            TouchPhase::Moved => {
                let Some(swipe) = self.swipe.as_mut() else { return };
                if swipe.across.is_none() && delta.0.hypot(delta.1) > 2. {
                    swipe.across = Some(delta.0.abs() > delta.1.abs() * 1.5);
                }
                if swipe.across == Some(true) {
                    swipe.travel += delta.0;
                    self.slide = None;
                    cx.stop_propagation();
                    cx.notify();
                }
            }
            TouchPhase::Ended | TouchPhase::Cancelled => {
                let Some(swipe) = self.swipe.take() else { return };
                if swipe.across != Some(true) {
                    return;
                }
                let offset = self.swipe_offset(swipe.travel);
                match self.neighbour(swipe.travel).filter(|_| swipe.travel.abs() > SWIPE_COMMIT) {
                    Some(place) => {
                        let left = self.places.iter().find(|p| self.places_shown(p)).cloned();
                        self.switch_to(place, cx);
                        // the new list carries on from where it was beside the old one, which goes out the other side
                        let width = self.sidebar_width;
                        let from = if swipe.travel < 0. { width + offset } else { offset - width };
                        self.slide = Some((from, Instant::now(), left));
                    }
                    None => self.slide = Some((offset, Instant::now(), None)),
                }
                cx.notify();
            }
        }
    }

    /// The place a swipe of `travel` px goes to: fingers to the left bring the next one, as in Arc.
    fn neighbour(&self, travel: f32) -> Option<Place> {
        let at = self.places.iter().position(|p| self.places_shown(p))?;
        let to = if travel < 0. { at.checked_add(1)? } else { at.checked_sub(1)? };
        self.places.get(to).cloned()
    }

    /// Whether `place` is the one shown.
    pub(super) fn places_shown(&self, place: &Place) -> bool {
        match (place, &self.machine_label) {
            (Place::Machine(m), Some(label)) => &m.label == label,
            (Place::Session(s), None) => crate::herdr::current_session().as_deref() == Some(s.name.as_str()),
            _ => false,
        }
    }

    /// The list's offset under the fingers: it follows them most of the way to the next place, and past that (or where no place
    /// is next) against a pull that grows the further they go, as iOS's rubber band: never past a sidebar's width more.
    fn swipe_offset(&self, travel: f32) -> f32 {
        let width = self.sidebar_width;
        let reach = if self.neighbour(travel).is_some() { width * 0.85 } else { 0. };
        let over = (travel.abs() - reach).max(0.);
        travel.signum() * (travel.abs().min(reach) + (1. - 1. / (over * 0.4 / width + 1.)) * width)
    }

    /// The workspace list's offset now (under the fingers, springing home after a swipe, else 0), and the place shown
    /// beside it with that place's offset: the one a swipe goes to, or the one a finished swipe left.
    pub(super) fn list_offset(&mut self, window: &mut Window) -> (f32, Option<(Place, f32)>) {
        let width = self.sidebar_width;
        if let Some(swipe) = self.swipe.as_ref().filter(|s| s.across == Some(true)) {
            let offset = self.swipe_offset(swipe.travel);
            let beside = self.neighbour(swipe.travel).map(|p| (p, if swipe.travel < 0. { offset + width } else { offset - width }));
            return (offset, beside);
        }
        let Some((from, at, left)) = self.slide.clone() else { return (0., None) };
        let t = at.elapsed().as_secs_f32() / SPRING_DURATION.as_secs_f32();
        if t >= 1. {
            self.slide = None;
            return (0., None);
        }
        window.request_animation_frame();
        let offset = from * (1. - damped_spring(0.85, t));
        (offset, left.map(|p| (p, if from > 0. { offset - width } else { offset + width })))
    }

    /// Another place's sidebar as a swipe shows it beside this one's: its name, then its workspaces and agents from its
    /// last snapshot, or a word on why there is none.
    pub(super) fn place_preview(&self, place: &Place, theme: &Theme, window: &mut Window, cx: &mut App) -> gpui::AnyElement {
        let settings = SettingsGlobal::of(cx).settings.clone();
        let heading = div().px(px(16.)).pt(px(8.)).pb(px(6.)).text_size(px(11.)).text_color(theme.text_faint).truncate().child(place.look(cx).0);
        let column = div().size_full().flex().flex_col().child(heading);
        let Some(snapshot) = self.snapshots.get(&place.key()) else {
            if let Place::Session(s) = place && !s.running {
                let why = "Not running: it starts when you get there";
                return column.child(div().px(px(16.)).pt(px(8.)).text_size(px(11.)).text_color(theme.text_faint).child(why)).into_any_element();
            }
            return column.child(div().px(px(16.)).pt(px(12.)).child(crate::ui::loader::spinner(2.5, window))).into_any_element();
        };
        let groups = crate::model::groups(snapshot.workspace_tree());
        let workspaces = div().flex_1().min_h_0().overflow_hidden().px(px(8.)).flex().flex_col().gap(px(2.)).children(groups.into_iter().flat_map(|(w, kids)| {
            let dir = w.worktree.as_ref().map(|t| t.checkout_path.clone()).filter(|d| !d.is_empty());
            let seed = w.worktree.as_ref().map_or(w.label.clone(), |t| t.repo_key.clone());
            let icon = crate::ui::project_icon::project_icon(&w.label, &seed, dir.as_deref(), 14., false, theme, cx);
            let head = div().h(px(32.)).px(px(8.)).flex().items_center().gap(px(8.)).child(icon).child(div().truncate().child(w.label.clone()));
            std::iter::once(head.into_any_element()).chain(
                kids.into_iter().map(|k| div().h(px(32.)).pl(px(30.)).flex().items_center().text_color(theme.text_muted).child(div().truncate().child(k.label.clone())).into_any_element()),
            )
        }));
        let new_workspace = div()
            .flex_none()
            .m(px(8.))
            .h(px(32.))
            .px(px(8.))
            .flex()
            .items_center()
            .gap(px(8.))
            .text_color(theme.text_muted)
            .child(div().text_size(px(15.)).child("+"))
            .child("New workspace");
        // its waits are not tracked here, so they go without
        let mut agents = super::sidebar::agent_entries(snapshot, self.agent_sort, &Default::default());
        let any_agents = !agents.is_empty();
        if settings.attention_agents {
            agents.retain(|(p, _, _)| crate::notify::wants_attention(p.agent_status));
        }
        let agents = (settings.show_agents && any_agents).then(|| {
            let heading = div().px(px(16.)).pt(px(12.)).pb(px(4.)).border_t_1().border_color(theme.border).text_size(px(11.)).text_color(theme.text_faint).child("Agents");
            let list = div()
                .flex_none()
                .map(|el| match self.agents_height {
                    Some(h) => el.max_h(px(h)),
                    None => el.max_h(gpui::relative(0.4)),
                })
                .overflow_hidden()
                .px(px(8.))
                .flex()
                .flex_col()
                .gap(px(2.))
                .children(super::sidebar::agent_rows(agents, self.agent_sort, settings.compact_agents, None, theme, |row, _| row));
            [heading.into_any_element(), list.into_any_element()]
        });
        column.child(workspaces).child(new_workspace).children(agents.into_iter().flatten()).into_any_element()
    }

    /// Moves the window to a session on this Mac: its socket goes in the settings, and everything the last server's
    /// ids named is dropped. A stopped session's server is started by the sync loop, as at launch.
    pub(super) fn switch_session(&mut self, session: SessionInfo, cx: &mut Context<Self>) {
        // the default session is where Shepherd looks without a socket set, so it leaves the setting empty
        let socket = (!session.default).then_some(session.socket_path);
        self.move_to(socket, None, cx);
    }

    /// Moves the window to a saved machine, through its relay.
    pub(super) fn switch_machine(&mut self, machine: Machine, cx: &mut Context<Self>) {
        match crate::herdr::relay(&machine, &self.rt) {
            Ok(socket) => {
                self.move_to(Some(socket.to_string_lossy().into_owned()), Some(machine.id), cx);
                crate::herdr::show_machine(Some(machine.target.clone()));
                self.machine_label = Some(machine.label);
            }
            Err(e) => crate::ui::toast::error(cx, format!("Couldn't reach {}: {e:#}", machine.label)),
        }
    }

    fn move_to(&mut self, socket: Option<String>, machine: Option<String>, cx: &mut Context<Self>) {
        // the place left keeps its last snapshot, so coming back shows its workspaces before herdr answers
        if let (Some(left), Some(snapshot)) = (self.places.iter().find(|p| self.places_shown(p)).map(Place::key), self.snapshot.take()) {
            self.snapshots.insert(left, snapshot);
        }
        save_from_sidebar(cx, |s| {
            s.herdr_socket = socket.clone();
            s.herdr_machine = machine.clone();
        });
        crate::herdr::set_socket(socket.clone()); // also when the settings could not be saved
        crate::herdr::show_machine(None); // a machine's switch names it right after
        self.machine_label = None;
        self.client = socket.map_or_else(Client::local, |p| Client::at(p.into()));
        let key = match &machine {
            Some(id) => format!("machine:{id}"),
            None => self.places.iter().find_map(|p| match p {
                Place::Session(s) if crate::herdr::current_session().as_deref() == Some(s.name.as_str()) => Some(s.socket_path.clone()),
                _ => None,
            }).unwrap_or_default(),
        };
        self.snapshot = self.snapshots.get(&key).cloned();
        self.error = None;
        self.connecting = Some(Instant::now());
        self.panes.clear();
        self.recent_panes.clear();
        self.focused = None;
        self.selected_tab = None;
        self.shown_workspace = None;
        self.marked_seen = None;
        self.focus_next = None;
        self.renaming = None;
        self.settling = None;
        self.pending_worktrees.clear();
        self.removing.clear();
        self.unseen.clear();
        self.since = Default::default();
        self.connect(cx);
        self.refresh_places(cx);
        self.prefetch_neighbours(cx);
        cx.notify();
    }

    pub(super) fn open_session_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let theme = Theme::of(cx).clone();
        let mut field = |placeholder: &'static str, cx: &mut Context<Self>| {
            cx.new(|cx| {
                let mut input = InputState::new(window, cx).placeholder(placeholder);
                input.set_editor_style(crate::ui::input_style(&theme));
                input
            })
        };
        let (name, host) = (field("Name", cx), field("user@host or a Host from ~/.ssh/config", cx));
        name.update(cx, |input, cx| input.focus(window, cx));
        let enter = |shell: &mut Shell, _: &Entity<InputState>, event: &InputEvent, _: &mut Window, cx: &mut Context<Shell>| {
            if let InputEvent::PressEnter { .. } = event {
                shell.finish_session_dialog(true, cx);
            }
        };
        let events = vec![cx.subscribe_in(&name, window, enter), cx.subscribe_in(&host, window, enter)];
        self.session_dialog = Some(crate::ui::Popup::new(SessionDialog { name, host, remote: false, flips: 0, _events: events }));
        cx.notify();
    }

    pub(super) fn toggle_session_remote(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(dialog) = self.session_dialog.as_mut() else { return };
        dialog.value.remote = !dialog.value.remote;
        dialog.value.flips += 1;
        let focus = if dialog.value.remote { &dialog.value.host } else { &dialog.value.name };
        focus.update(cx, |input, cx| input.focus(window, cx));
        cx.notify();
    }

    /// Create (and move to) the session, here or on an SSH host, or cancel. Input herdr would refuse keeps the dialog
    /// open.
    pub(super) fn finish_session_dialog(&mut self, create: bool, cx: &mut Context<Self>) {
        let Some(dialog) = self.session_dialog.as_mut().filter(|d| d.is_open()) else { return };
        let name = dialog.value.name.read(cx).value().trim().to_string();
        let host = dialog.value.host.read(cx).value().trim().to_string();
        let remote = dialog.value.remote;
        // a remote session may go unnamed: the host's default one
        let name = if remote && name.is_empty() { "default".to_string() } else { name };
        if create && !crate::herdr::valid_session_name(&name) {
            crate::ui::toast::error(cx, "A session's name takes ASCII letters, digits, '.', '_' and '-'.");
            return;
        }
        if create && remote && host.is_empty() {
            crate::ui::toast::error(cx, "Which host? An SSH target such as user@host or a Host from ~/.ssh/config.");
            return;
        }
        dialog.close();
        self.focused = None; // an input held focus: back to herdr's focused pane
        if create && remote {
            crate::ui::toast::info(cx, format!("Setting up {name} on {host}…"), None);
            let task = cx.background_spawn(async move { crate::herdr::add_machine(&host, &name) });
            cx.spawn(async move |this, cx| {
                let added = task.await;
                let _ = this.update(cx, |s, cx| match added {
                    Ok(machine) => s.switch_machine(machine, cx),
                    Err(e) => crate::ui::toast::error(cx, format!("{e:#}")),
                });
            })
            .detach();
        } else if create {
            // a name that exists moves there instead
            let known = self.places.iter().find_map(|p| match p {
                Place::Session(s) if s.name == name => Some(s.clone()),
                _ => None,
            });
            self.switch_session(known.unwrap_or_else(|| crate::herdr::new_session(&name)), cx);
        }
        cx.notify();
    }

    pub(super) fn confirm_delete(&mut self, place: Place, cx: &mut Context<Self>) {
        self.session_delete = Some(crate::ui::Popup::new(place));
        cx.notify();
    }

    /// A session: herdr stops its server, closing its panes, and deletes it. A machine: herdr forgets it, and its
    /// sessions keep running on the host. The one shown now leaves for this Mac's default session first.
    pub(super) fn finish_delete(&mut self, delete: bool, cx: &mut Context<Self>) {
        let Some(confirm) = self.session_delete.as_mut().filter(|c| c.is_open()) else { return };
        confirm.close();
        let place = confirm.value.clone();
        if !delete {
            return cx.notify();
        }
        if self.places_shown(&place) {
            self.move_to(None, None, cx);
        }
        let (label, task) = match place {
            Place::Session(s) => (s.name.clone(), cx.background_spawn(async move { crate::herdr::delete_session(&s.name, s.running) })),
            Place::Machine(m) => (m.label.clone(), cx.background_spawn(async move { crate::herdr::remove_machine(&m.id) })),
        };
        cx.spawn(async move |this, cx| {
            let done = task.await;
            let _ = this.update(cx, |s, cx| {
                if let Err(e) = done {
                    crate::ui::toast::error(cx, format!("Couldn't delete {label}: {e:#}"));
                }
                s.refresh_places(cx);
            });
        })
        .detach();
        cx.notify();
    }
}

/// A place's icon drawn at `size`: a drawn icon (`icon:<name>`) in `color`, or the emoji.
pub(super) fn icon_mark(icon: &str, size: f32, color: gpui::Hsla) -> gpui::AnyElement {
    match icon.strip_prefix("icon:") {
        Some(name) => gpui::svg().path(SharedString::from(format!("icons/solar/{name}.svg"))).size(px(size)).flex_none().text_color(color).into_any_element(),
        None => div().text_size(px(size)).child(icon.to_string()).into_any_element(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emoji_search_by_name() {
        let all: usize = emoji_rows("").iter().map(|r| if let EmojiRow::Emoji(e) = r { e.len() } else { 0 }).sum();
        assert!(all > 1500, "{all}");
        let rocket = emoji_rows("rocket");
        assert!(rocket.iter().any(|r| matches!(r, EmojiRow::Emoji(e) if e.contains(&"🚀"))));
        assert!(emoji_rows("no such emoji at all").is_empty());
    }

    #[test]
    fn solar_icons_by_name() {
        assert!(crate::assets::solar_names().count() > 1000);
        assert!(crate::assets::solar_icon("rocket-2").is_some_and(|svg| svg.starts_with("<svg") && svg.contains("currentColor")));
        let found: Vec<_> = icon_rows("home").concat();
        assert!(found.iter().any(|t| matches!(t, IconTile::Solar("home-2"))));
        assert!(matches!(icon_rows("")[0][0], IconTile::Letter));
    }
}
