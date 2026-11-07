//! A file tab's editor, Zeron's `crates/ui/src/files/editor.rs` and `editor_adapter.rs` (MIT): gpui-base's
//! `EditorState`, colored by `zeron_syntax` through a highlighter that shifts its spans with each edit until the next
//! highlight lands. Its colors are the panes' (`crate::ui::markdown::syntax_color`), not Zeron's palette.
use crate::theme::Theme;
use gpui::{AppContext as _, Context, DispatchPhase, Entity, HighlightStyle, IntoElement, ScrollWheelEvent, SharedString, Styled as _, Window, canvas, px};
use gpui_base::input::{EditorState, FoldRange, HighlightStyleResolver, InputEdit, InputEditorStyle, InputHighlighter, Rope};
use std::{cell::Cell, ops::Range, rc::Rc, sync::Arc};
use zeron_syntax::{HighlightKind, HighlightedDocument};

/// The theme's color for each highlight name.
struct StyleResolver {
    theme: Theme,
}

impl HighlightStyleResolver for StyleResolver {
    fn style(&self, name: &str) -> Option<HighlightStyle> {
        kind_for_name(name).map(|kind| HighlightStyle { color: Some(crate::ui::markdown::syntax_color(kind, &self.theme)), ..Default::default() })
    }
}

#[derive(Clone)]
struct Highlighter {
    language: SharedString,
    spans: Vec<(Range<usize>, HighlightKind)>,
}

impl Highlighter {
    fn new(source: &str, document: &HighlightedDocument) -> Self {
        Self { language: format!("{:?}", document.language).into(), spans: absolute_spans(source, document) }
    }
}

impl InputHighlighter for Highlighter {
    fn language(&self) -> SharedString {
        self.language.clone()
    }

    /// Spans after an edit move with it; those it touched go until the next highlight.
    fn update(&mut self, edit: Option<InputEdit>, _text: &Rope, _folding: bool, _window: &mut Window, _cx: &mut Context<EditorState>) {
        let Some(edit) = edit else { return };
        let removed = edit.old_end_byte.saturating_sub(edit.start_byte);
        let inserted = edit.new_end_byte.saturating_sub(edit.start_byte);
        self.spans.retain_mut(|(range, _)| {
            if range.end <= edit.start_byte {
                return true;
            }
            if range.start >= edit.old_end_byte {
                if inserted >= removed {
                    let delta = inserted - removed;
                    range.start = range.start.saturating_add(delta);
                    range.end = range.end.saturating_add(delta);
                } else {
                    let delta = removed - inserted;
                    range.start = range.start.saturating_sub(delta);
                    range.end = range.end.saturating_sub(delta);
                }
                return true;
            }
            false
        });
    }

    fn styles(&self, range: &Range<usize>, resolver: &dyn HighlightStyleResolver) -> Vec<(Range<usize>, HighlightStyle)> {
        if range.is_empty() {
            return Vec::new();
        }
        let mut runs = Vec::new();
        let mut cursor = range.start;
        for (span, kind) in &self.spans {
            if span.end <= range.start {
                continue;
            }
            if span.start >= range.end {
                break;
            }
            let start = span.start.max(range.start).max(cursor);
            let end = span.end.min(range.end);
            if start > cursor {
                runs.push((cursor..start, HighlightStyle::default()));
            }
            if start < end {
                runs.push((start..end, resolver.style(name_for_kind(*kind)).unwrap_or_default()));
                cursor = end;
            }
        }
        if cursor < range.end {
            runs.push((cursor..range.end, HighlightStyle::default()));
        }
        runs
    }

    fn fold_ranges(&self, _text: &Rope) -> Vec<FoldRange> {
        Vec::new()
    }
}

pub(super) fn editor_style(theme: &Theme) -> InputEditorStyle {
    InputEditorStyle {
        foreground: theme.text.opacity(0.93),
        muted_foreground: theme.text_faint,
        background: gpui::transparent_black(),
        border: theme.border,
        selection: theme.accent.opacity(0.22),
        caret: theme.text,
        highlight_styles: Arc::new(StyleResolver { theme: theme.clone() }),
        editor_active_line: Some(theme.wash(0.025)),
        editor_gutter_background: Some(gpui::transparent_black()),
        ..Default::default()
    }
}

/// An editor over a file's `text`, read-only when only part of it was read.
pub(super) fn new_file_editor<T: 'static>(text: impl Into<SharedString>, path: &str, readonly: bool, theme: &Theme, window: &mut Window, cx: &mut Context<T>) -> Entity<EditorState> {
    let editor = cx.new(|cx| EditorState::new(window, cx).language(path.to_string()).line_number(true).folding(false).soft_wrap(false).default_value(text));
    editor.update(cx, |state, cx| {
        state.set_editor_style(editor_style(theme));
        state.set_readonly(readonly, cx);
    });
    editor
}

/// A highlight of `source` into the editor, if the editor still holds `source`: a stale one would color shifted bytes.
pub(super) fn install_highlighter(editor: &Entity<EditorState>, source: String, document: Arc<HighlightedDocument>, cx: &mut gpui::App) {
    if editor.read(cx).value().as_ref() != source {
        return;
    }
    editor.update(cx, |state, cx| {
        state.set_highlighter_factory(Rc::new(move |_| Some(Box::new(Highlighter::new(&source, &document)) as Box<_>)), cx);
    });
}

/// The file as it is on disk now, into an editor without changes: its cursor and scroll stay.
pub(super) fn replace_file_contents<T: 'static>(editor: &Entity<EditorState>, text: impl Into<SharedString>, window: &mut Window, cx: &mut Context<T>) {
    let text = text.into();
    editor.update(cx, |state, cx| {
        let selection = state.selected_range();
        let scroll_offset = state.scroll_offset();
        state.set_value(text, window, cx);
        let end = state.value().len();
        state.set_selected_range(selection.start.min(end)..selection.end.min(end), cx);
        state.set_scroll_offset(scroll_offset, cx);
    });
}

/// The terminal's wheel speed over the editor, which scrolls 1:1: laid over it, this takes the wheel first. Offsets
/// land after the next layout, so a frame's events add up from the one before rather than the editor's stale offset.
pub(super) fn terminal_wheel(editor: Entity<EditorState>) -> impl IntoElement {
    canvas(
        |_, _, _| {},
        move |bounds, _, window, _| {
            let pending = Rc::new(Cell::new(None));
            window.on_mouse_event(move |e: &ScrollWheelEvent, phase, _, cx| {
                if phase != DispatchPhase::Capture || !bounds.contains(&e.position) {
                    return;
                }
                editor.update(cx, |state, cx| {
                    let delta = e.delta.pixel_delta(state.line_height().unwrap_or(px(20.))) * crate::terminal::pane::WHEEL_SPEED;
                    let at = pending.get().unwrap_or(state.scroll_offset()) + delta;
                    pending.set(Some(at));
                    state.set_scroll_offset(at, cx);
                });
                cx.stop_propagation();
            });
        },
    )
    .absolute()
    .inset_0()
}

fn absolute_spans(source: &str, document: &HighlightedDocument) -> Vec<(Range<usize>, HighlightKind)> {
    let mut starts = vec![0];
    starts.extend(source.bytes().enumerate().filter_map(|(index, byte)| (byte == b'\n').then_some(index + 1)));
    let mut spans = Vec::new();
    for (line_index, line) in document.lines.iter().enumerate() {
        let Some(line_start) = starts.get(line_index).copied() else { break };
        for span in line {
            let range = line_start.saturating_add(span.range.start)..line_start.saturating_add(span.range.end);
            if range.start <= range.end && range.end <= source.len() && source.is_char_boundary(range.start) && source.is_char_boundary(range.end) {
                spans.push((range, span.kind));
            }
        }
    }
    spans.sort_unstable_by_key(|(range, _)| (range.start, range.end));
    spans
}

const KINDS: [HighlightKind; 31] = [
    HighlightKind::Comment,
    HighlightKind::Keyword,
    HighlightKind::String,
    HighlightKind::StringSpecial,
    HighlightKind::Escape,
    HighlightKind::Number,
    HighlightKind::Boolean,
    HighlightKind::Type,
    HighlightKind::TypeBuiltin,
    HighlightKind::Constructor,
    HighlightKind::Function,
    HighlightKind::FunctionBuiltin,
    HighlightKind::Macro,
    HighlightKind::Property,
    HighlightKind::Constant,
    HighlightKind::Variable,
    HighlightKind::VariableSpecial,
    HighlightKind::Parameter,
    HighlightKind::Operator,
    HighlightKind::Punctuation,
    HighlightKind::Tag,
    HighlightKind::Attribute,
    HighlightKind::Label,
    HighlightKind::MarkupHeading,
    HighlightKind::MarkupRaw,
    HighlightKind::MarkupLink,
    HighlightKind::MarkupReference,
    HighlightKind::MarkupEmphasis,
    HighlightKind::MarkupStrong,
    HighlightKind::Embedded,
    HighlightKind::Invalid,
];

fn name_for_kind(kind: HighlightKind) -> &'static str {
    match kind {
        HighlightKind::Comment => "comment",
        HighlightKind::Keyword => "keyword",
        HighlightKind::String => "string",
        HighlightKind::StringSpecial => "string_special",
        HighlightKind::Escape => "escape",
        HighlightKind::Number => "number",
        HighlightKind::Boolean => "boolean",
        HighlightKind::Type => "type",
        HighlightKind::TypeBuiltin => "type_builtin",
        HighlightKind::Constructor => "constructor",
        HighlightKind::Function => "function",
        HighlightKind::FunctionBuiltin => "function_builtin",
        HighlightKind::Macro => "macro",
        HighlightKind::Property => "property",
        HighlightKind::Constant => "constant",
        HighlightKind::Variable => "variable",
        HighlightKind::VariableSpecial => "variable_special",
        HighlightKind::Parameter => "parameter",
        HighlightKind::Operator => "operator",
        HighlightKind::Punctuation => "punctuation",
        HighlightKind::Tag => "tag",
        HighlightKind::Attribute => "attribute",
        HighlightKind::Label => "label",
        HighlightKind::MarkupHeading => "markup_heading",
        HighlightKind::MarkupRaw => "markup_raw",
        HighlightKind::MarkupLink => "markup_link",
        HighlightKind::MarkupReference => "markup_reference",
        HighlightKind::MarkupEmphasis => "markup_emphasis",
        HighlightKind::MarkupStrong => "markup_strong",
        HighlightKind::Embedded => "embedded",
        HighlightKind::Invalid => "invalid",
    }
}

fn kind_for_name(name: &str) -> Option<HighlightKind> {
    KINDS.into_iter().find(|kind| name_for_kind(*kind) == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn highlighted(source: &str) -> HighlightedDocument {
        zeron_syntax::highlight(zeron_syntax::HighlightRequest { source, path: Some("a.rs"), fence_tag: None }).unwrap()
    }

    #[test]
    fn line_relative_spans_become_absolute_and_runs_cover_the_request_in_order() {
        let source = "let café = \"😀\";\nlet n = 42;";
        let highlighter = Highlighter::new(source, &highlighted(source));
        let string = source.find('"').unwrap();
        assert!(highlighter.spans.iter().any(|(r, k)| *k == HighlightKind::String && r.start == string));
        let resolver = StyleResolver { theme: Theme::for_appearance(gpui::WindowAppearance::Dark) };
        let runs = highlighter.styles(&(0..source.len()), &resolver);
        assert_eq!(runs.first().unwrap().0.start, 0);
        assert_eq!(runs.last().unwrap().0.end, source.len());
        assert!(runs.windows(2).all(|pair| pair[0].0.end == pair[1].0.start));
    }

    #[test]
    fn every_kind_has_a_name_and_back() {
        for kind in KINDS {
            assert_eq!(kind_for_name(name_for_kind(kind)), Some(kind));
        }
    }
}
