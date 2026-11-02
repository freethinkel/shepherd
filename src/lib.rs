// ponytail: GPUI code passes callbacks, tuples of fn tables and render state around by hand; Zed allows these too.
#![allow(clippy::type_complexity, clippy::too_many_arguments)]

pub mod agents;
pub mod analytics;
pub mod annotations;
pub mod assets;
pub mod backend;
pub mod bindings;
pub mod changes;
pub mod commands;
pub mod hc;
pub mod herdr;
pub mod mock;
pub mod model;
pub mod notify;
pub mod process;
pub mod rex;
pub mod right_panel;
pub mod settings;
pub mod shell;
pub mod terminal;
pub mod theme;
pub mod tmux;
pub mod ui;
pub mod update;
pub mod web;
pub mod worktree;
