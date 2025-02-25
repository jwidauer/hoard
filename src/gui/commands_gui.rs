use crate::command::hoard_command::HoardCommand;
use crate::command::trove::CommandTrove;
use crate::config::HoardConfig;
use crate::gui::event::{Config, Event, Events};
use crate::gui::help::{draw as draw_help, key_handler as key_handler_help};
use crate::gui::inline_edit::controls::key_handler as key_handler_inline_edit;
use crate::gui::list_search::controls::key_handler as key_handler_list_search;
use crate::gui::list_search::render::draw as draw_list_search;
use crate::gui::new_command::controls::key_handler as key_handler_create_command;
use crate::gui::new_command::render::draw as draw_new_command_input;
use crate::gui::parameter_input::controls::key_handler as key_handler_parameter_input;
use crate::gui::parameter_input::render::draw as draw_parameter_input;
use eyre::Result;
use std::fmt;
use std::io::stdout;
use std::time::Duration;
use termion::raw::IntoRawMode;
use termion::screen::IntoAlternateScreen;
use tui::{backend::TermionBackend, widgets::ListState, Terminal};

pub struct State {
    pub input: String,
    pub commands: Vec<HoardCommand>,
    pub command_list_state: ListState,
    pub namespace_tab_state: ListState,
    pub should_exit: bool,
    pub should_delete: bool,
    pub draw_state: DrawState,
    pub control_state: ControlState,
    pub edit_selection: EditSelection,
    pub new_command: Option<HoardCommand>,
    pub string_to_edit: String,
    pub parameter_token: String,
    pub parameter_ending_token: String,
    pub selected_command: Option<HoardCommand>,
    pub provided_parameter_count: u16,
    pub error_message: String,
}

impl State {
    pub fn update_string_to_edit(&mut self) -> &mut Self {
        let selected_idx = self.command_list_state.selected().unwrap();
        let cloned_selected_command = self.commands.get(selected_idx).unwrap().clone();
        match self.edit_selection {
            EditSelection::Name => self.string_to_edit = cloned_selected_command.name,
            EditSelection::Tags => self.string_to_edit = cloned_selected_command.tags_as_string(),
            EditSelection::Description => {
                self.string_to_edit = cloned_selected_command.description.unwrap_or_default();
            }

            EditSelection::Command => self.string_to_edit = cloned_selected_command.command,
            EditSelection::Namespace => (),
        };
        self
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum DrawState {
    Search,
    ParameterInput,
    Help,
    Create,
}

#[derive(Debug, Eq, PartialEq)]
pub enum ControlState {
    Search,
    Edit,
}

impl fmt::Display for ControlState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Search => write!(f, "Search (<Tab>/<Ctrl-E> to edit)"),
            Self::Edit => write!(
                f,
                "Edit (<Enter> to confirm. <Tab> to switch. <Esc> to abort)"
            ),
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum EditSelection {
    Name,
    Tags,
    Description,
    Command,
    Namespace,
}

impl fmt::Display for EditSelection {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Name => write!(f, "Name"),
            Self::Tags => write!(f, "Tags"),
            Self::Description => write!(f, "Description"),
            Self::Command => write!(f, "Command"),
            Self::Namespace => write!(f, "Namespace"),
        }
    }
}

impl EditSelection {
    pub const fn next(&self) -> Self {
        match self {
            Self::Name | Self::Namespace | Self::Command => Self::Tags,
            Self::Tags => Self::Description,
            Self::Description => Self::Command,
        }
    }
    pub const fn edit_next(&self) -> Self {
        match self {
            Self::Command => Self::Namespace,
            Self::Namespace => Self::Name,
            Self::Name => Self::Description,
            Self::Description => Self::Tags,
            Self::Tags => Self::Command,
        }
    }
}

#[allow(clippy::too_many_lines)]
pub fn run(trove: &mut CommandTrove, config: &HoardConfig) -> Result<Option<HoardCommand>> {
    let events = Events::with_config(Config {
        tick_rate: Duration::from_millis(200),
    });
    let trove_clone = trove.clone();
    let mut app_state = State {
        input: String::new(),
        commands: trove_clone.commands.clone(),
        command_list_state: ListState::default(),
        namespace_tab_state: ListState::default(),
        should_exit: false,
        should_delete: false,
        draw_state: DrawState::Search,
        control_state: ControlState::Search,
        edit_selection: EditSelection::Command,
        new_command: None,
        string_to_edit: String::new(),
        parameter_token: config.parameter_token.as_ref().unwrap().clone(),
        parameter_ending_token: config.parameter_ending_token.as_ref().unwrap().clone(),

        selected_command: None,
        provided_parameter_count: 0,
        error_message: String::new(),
    };

    app_state.command_list_state.select(Some(0));
    app_state.namespace_tab_state.select(Some(0));

    let stdout = stdout().into_raw_mode()?;
    let stdout = stdout.into_alternate_screen().unwrap();
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    //let menu_titles = vec!["List", "Search", "Add", "Delete", "Quit"];
    let mut namespace_tabs: Vec<&str> = trove_clone.namespaces();
    namespace_tabs.insert(0, "All");
    loop {
        // Draw GUI
        match app_state.draw_state {
            DrawState::Search => {
                draw_list_search(&mut app_state, config, &mut namespace_tabs, &mut terminal)?;
            }
            DrawState::ParameterInput => {
                draw_parameter_input(&mut app_state, config, &mut terminal)?;
            }
            DrawState::Help => {
                draw_help(config, &mut terminal)?;
            }
            DrawState::Create => {
                draw_new_command_input(
                    &mut app_state,
                    config,
                    &mut terminal,
                    &config.default_namespace,
                )?;
            }
        }

        if let Event::Input(input) = events.next()? {
            let command = match app_state.draw_state {
                DrawState::Search => match app_state.control_state {
                    ControlState::Search => key_handler_list_search(
                        input,
                        &mut app_state,
                        &trove.commands,
                        &namespace_tabs,
                    ),
                    ControlState::Edit => key_handler_inline_edit(input, &mut app_state),
                },
                DrawState::ParameterInput => key_handler_parameter_input(input, &mut app_state),
                DrawState::Help => key_handler_help(input, &mut app_state),
                DrawState::Create => {
                    key_handler_create_command(input, &mut app_state, &config.default_namespace)
                }
            };

            if let Some(output) = command {
                if app_state.draw_state == DrawState::Create {
                    trove.add_command(output);
                    app_state.commands = trove.commands.clone();
                    app_state.draw_state = DrawState::Search;
                } else if app_state.control_state == ControlState::Edit {
                    // Command has been edited
                    trove.update_command_by_name(&output);
                    app_state.commands = trove.commands.clone();
                    app_state.control_state = ControlState::Search;
                } else if app_state.should_delete {
                    trove.remove_command(&output.name).ok();
                    app_state.commands = trove.commands.clone();
                    app_state.should_delete = false;
                } else {
                    // Command has been selected
                    terminal.show_cursor()?;
                    return Ok(Some(output));
                }
            }

            if app_state.should_exit {
                terminal.show_cursor()?;
                return Ok(None);
            }
        }
    }
}
