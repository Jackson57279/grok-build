//! `/connect` -- connect a Cursor account and bill usage to Cursor.

use crate::app::actions::Action;
use crate::slash::command::{CommandExecCtx, CommandResult, SlashCommand};

pub struct ConnectCommand;

impl SlashCommand for ConnectCommand {
    fn name(&self) -> &str {
        "connect"
    }

    fn description(&self) -> &str {
        "Connect your Cursor account (bill usage to Cursor)"
    }

    fn usage(&self) -> &str {
        "/connect"
    }

    fn run(&self, _ctx: &mut CommandExecCtx, _args: &str) -> CommandResult {
        CommandResult::Action(Action::ConnectCursor)
    }
}
