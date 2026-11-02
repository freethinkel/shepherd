use shepherd::right_panel::{Offer, PanelState, Surface, clamp_width, fits, offered, slides};

fn b(id: &str) -> Surface {
    Surface::Browser(id.into())
}

fn panel(tabs: &[Surface]) -> PanelState {
    let mut p = PanelState::default();
    tabs.iter().for_each(|s| p.add(s.clone()));
    p
}

// also a link clicked with the panel closed
#[test]
fn add_opens_the_panel_on_the_tab() {
    let mut p = PanelState::default();
    p.add(b("1"));
    assert!(p.open);
    assert_eq!(p.active, Some(b("1")));
    p.add(Surface::PullRequest);
    p.add(b("1")); // present: activated, not doubled
    assert_eq!(p.tabs, [b("1"), Surface::PullRequest]);
    assert_eq!(p.active, Some(b("1")));
}

#[test]
fn closing_the_active_tab_falls_back_to_the_last_visited() {
    let mut p = panel(&[b("1"), b("2"), b("3")]);
    p.activate(&b("1"));
    p.activate(&b("3"));
    p.close(&b("3"));
    assert_eq!(p.active, Some(b("1")));
}

#[test]
fn closing_another_tab_keeps_the_active_one() {
    let mut p = panel(&[b("1"), b("2")]);
    p.close(&b("1"));
    assert_eq!(p.active, Some(b("2")));
}

#[test]
fn closing_the_last_tab_closes_the_panel_and_the_next_open_shows_the_picker() {
    let mut p = panel(&[b("1")]);
    p.close(&b("1"));
    assert!(!p.open);
    assert_eq!(p.active, None);
    p.toggle_seen(false);
    assert!(p.open);
    assert_eq!(p.active, None);
}

#[test]
fn reorder_leaves_the_active_tab_and_ignores_bad_indices() {
    let mut p = panel(&[b("1"), b("2"), b("3")]);
    p.activate(&b("2"));
    p.reorder(0, 2);
    assert_eq!(p.tabs, [b("2"), b("3"), b("1")]);
    assert_eq!(p.active, Some(b("2")));
    p.reorder(0, 9);
    p.reorder(9, 0);
    assert_eq!(p.tabs, [b("2"), b("3"), b("1")]);
}

#[test]
fn step_wraps_and_a_lone_tab_stays() {
    let mut p = panel(&[b("1"), b("2")]);
    p.step(true);
    assert_eq!(p.active, Some(b("1")));
    p.step(false);
    assert_eq!(p.active, Some(b("2")));
    let mut one = panel(&[b("1")]);
    one.step(true);
    assert_eq!(one.active, Some(b("1")));
    PanelState::default().step(true); // no tabs: no panic
}

#[test]
fn the_pull_request_toggle_opens_switches_and_closes() {
    let mut p = panel(&[b("1")]);
    p.toggle_seen(true); // closed
    p.toggle_pull_request(false);
    assert!(p.open);
    assert_eq!(p.active, Some(Surface::PullRequest));
    p.activate(&b("1"));
    p.toggle_pull_request(true);
    assert_eq!(p.active, Some(Surface::PullRequest));
    p.toggle_pull_request(true);
    assert!(!p.open);
    assert_eq!(p.tabs, [b("1"), Surface::PullRequest]); // the tab stays
}

#[test]
fn a_browser_tab_whose_page_is_gone_falls_back() {
    let mut p = panel(&[Surface::PullRequest, b("1"), b("2")]);
    p.retain_browsers(&["1"]);
    assert_eq!(p.tabs, [Surface::PullRequest, b("1")]);
    assert_eq!(p.active, Some(b("1")));
}

#[test]
fn offers_follow_the_remote_and_the_platform() {
    assert_eq!(offered(true, true, true), [Offer::Browser, Offer::Diffs, Offer::PullRequest]);
    assert_eq!(offered(false, true, true), [Offer::Browser, Offer::Diffs]);
    assert_eq!(offered(false, false, true), [Offer::Browser]);
    assert_eq!(offered(true, true, false), [Offer::Diffs, Offer::PullRequest]);
    assert!(offered(false, false, false).is_empty());
}

#[test]
fn widths_keep_the_centre_column_and_the_saved_value_survives_a_small_window() {
    assert_eq!(clamp_width(520., 1400., 240.), 520.);
    assert_eq!(clamp_width(9000., 1400., 240.), 860.); // 1400 - 240 - 300
    assert_eq!(clamp_width(100., 1400., 240.), 360.);
    assert_eq!(clamp_width(520., 800., 0.), 500.);
    assert!(fits(660., 0.));
    assert!(!fits(659., 0.));
    assert!(!fits(800., 200.));
}

#[test]
fn a_tab_strip_key_comes_back_as_its_surface() {
    for surface in [b("browser-3"), Surface::diff(2), Surface::PullRequest] {
        assert_eq!(Surface::of_key(surface.key()), surface);
    }
}

#[test]
fn a_visible_page_keeps_its_hands_off_the_keyboard_until_asked() {
    use shepherd::right_panel::page_takes_keyboard as takes;
    assert!(takes(true, false, false)); // a sidebar page has the window to itself, as today
    assert!(!takes(false, false, false)); // a panel page beside a terminal
    assert!(takes(false, true, false)); // clicked into
    assert!(!takes(false, true, true)); // a popup, the address field or a note holds it
    assert!(!takes(true, false, true));
}

#[test]
fn closing_a_tab_another_panel_holds_leaves_an_empty_open_panel_open() {
    let mut p = PanelState::default();
    p.open = true;
    p.close(&b("1"));
    assert!(p.open);
}
#[test]
fn the_toggles_go_by_what_is_on_screen() {
    let mut p = panel(&[b("1"), Surface::PullRequest]);
    p.toggle_seen(false); // open but hidden, in a window too narrow: it stays, to come back
    assert!(p.open);
    p.toggle_seen(true);
    assert!(!p.open);
    p.toggle_seen(false);
    assert!(p.open);
    p.toggle_pull_request(false); // on the tab, hidden: stays open
    assert!(p.open);
    p.toggle_pull_request(true);
    assert!(!p.open);
}

#[test]
fn the_panel_slides_when_opened_or_closed_in_place_only() {
    let at = |ws: &str, open| (Some(ws.to_string()), open);
    assert!(slides(&at("a", false), &at("a", true)));
    assert!(slides(&at("a", true), &at("a", false)));
    assert!(!slides(&at("a", true), &at("a", true)));
    // another workspace brings its own panel, and the first snapshot the one it finds
    assert!(!slides(&at("a", false), &at("b", true)));
    assert!(!slides(&(None, false), &at("a", true)));
}

#[test]
fn closing_leaves_the_expansion_by_every_way() {
    let closed = |close: fn(&mut PanelState)| {
        let mut p = panel(&[Surface::PullRequest]);
        p.expanded = true;
        close(&mut p);
        assert!(!p.open && !p.expanded);
    };
    closed(PanelState::shut);
    closed(|p| p.toggle_seen(true));
    closed(|p| p.toggle_pull_request(true));
    closed(|p| p.close(&Surface::PullRequest));
}
