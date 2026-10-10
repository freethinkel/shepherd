pub mod divider;
pub mod settings;

use crate::theme::Theme;
use gpui_base::input::InputEditorStyle;

/// gpui-base inputs are unstyled: colors from the theme, as Collie does.
pub fn input_style(theme: &Theme) -> InputEditorStyle {
    InputEditorStyle {
        foreground: theme.text,
        muted_foreground: theme.text_faint,
        border: theme.border,
        selection: theme.accent.opacity(0.3),
        caret: theme.text,
        ..Default::default()
    }
}
