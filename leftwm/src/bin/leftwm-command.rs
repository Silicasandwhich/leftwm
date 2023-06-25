use anyhow::anyhow;
use anyhow::{Context, Result};
use clap::{arg, command};
use leftwm_core::CommandPipe;
use leftwm_core::ReturnPipe;
use std::fs::OpenOptions;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::process::exit;
use xdg::BaseDirectories;

#[tokio::main]
async fn main() -> Result<()> {
    let matches = get_command().get_matches();

    let file_name = CommandPipe::pipe_name();
    let file_path = BaseDirectories::with_prefix("leftwm")?
        .find_runtime_file(&file_name)
        .with_context(|| format!("ERROR: Couldn't find {}", file_name.display()))?;
    let mut file = OpenOptions::new()
        .append(true)
        .open(file_path)
        .with_context(|| format!("ERROR: Couldn't open {}", file_name.display()))?;
    let mut exit_state = Ok(());
    if let Some(commands) = matches.get_many::<String>("COMMAND") {
        let mut ret_pipe = get_return_pipe().await?;
        for command in commands {
            if let Err(e) = writeln!(file, "{command}") {
                eprintln!(" ERROR: Couldn't write to commands.pipe: {e}");
                continue;
            }
            tokio::select! {
                Some(res) = ret_pipe.read_return() => {
                if let Some((result, msg)) = res.split_once(' '){
                        match result{
                            "OK:" => println!("{command}: {msg}"),
                            "ERROR:" => {eprintln!("{command}: {msg}");exit_state = Err(anyhow!("one or more errors occured when parsing commands"));},
                            _ => println!("{command}: {res}"),
                        }
                    }else{
                        println!("{command}: {res}");
                    }
            }
                _ = timeout(5000) => {eprintln!(" WARN: timeout connecting to return pipe. Command may have executed, but errors will not be displayed."); exit(1)},
            }
        }
    }

    if matches.get_flag("list") {
        print_commandlist();
    }
    exit_state
}

fn get_command() -> clap::Command {
    command!("LeftWM Command")
        .about("Sends external commands to LeftWM. After executing a command, check the logs or use \"leftwm-log\" to see any errors")
        .help_template(leftwm::utils::get_help_template())
        .args(&[
            arg!(-l --list "Print a list of available commands with their arguments."),
            arg!([COMMAND] ... "The command to be sent. See 'list' flag."),
        ])
}

fn print_commandlist() {
    println!(
        "
        Available Commands:

        Commands without arguments:

        UnloadTheme
        SoftReload
        ToggleFullScreen
        ToggleSticky
        SwapScreens
        MoveWindowToNextTag
        MoveWindowToPreviousTag
        MoveWindowToLastWorkspace
        MoveWindowToNextWorkspace
        MoveWindowToPreviousWorkspace
        FloatingToTile
        TileToFloating
        ToggleFloating
        MoveWindowUp
        MoveWindowDown
        MoveWindowTop
        FocusWindowUp
        FocusWindowDown
        FocusWindowTop
        FocusWorkspaceNext
        FocusWorkspacePrevious
        NextLayout
        PreviousLayout
        RotateTag
        ReturnToLastTag
        CloseWindow

        Commands with arguments:
            Use quotations for the command and arguments, like this:
            leftwm-command \"<command> <args>\"

        LoadTheme              Args: <Path_to/theme.ron>
            Note: `theme.toml` will be deprecated but stays for backwards compatibility for a while
        AttachScratchPad       Args: <ScratchpadName>
        ReleaseScratchPad      Args: <tag_index> or <ScratchpadName>
        NextScratchPadWindow   Args: <ScratchpadName>
        PrevScratchPadWindow   Args: <ScratchpadName>
        ToggleScratchPad       Args: <ScratchpadName>
        SendWorkspaceToTag     Args: <workspaxe_index> <tag_index> (int)
        SendWindowToTag        Args: <tag_index> (int)
        SetLayout              Args: <LayoutName>
        SetMarginMultiplier    Args: <multiplier-value> (float)
        FocusWindow            Args: <WindowClass> or <visible-window-index> (int)
        FocusNextTag           Args: <behavior> (string, optional)
        FocusPreviousTag       Args: <behavior> (string, optional)

        For more information please visit:
        https://github.com/leftwm/leftwm/wiki/External-Commands
         "
    );
}

#[derive(thiserror::Error, Debug, Clone, PartialEq, Eq, Hash)]
pub enum Error {
    #[error("Couldn't create the file: '{0}'")]
    CreateFile(PathBuf),

    #[error("Couldn't connect to file: '{0}'")]
    ConnectToFile(PathBuf),
}

async fn get_return_pipe() -> Result<ReturnPipe, Error> {
    let file_name = ReturnPipe::pipe_name();

    let pipe_file =
        place_runtime_file(&file_name).map_err(|_| Error::CreateFile(file_name.clone()))?;

    ReturnPipe::new(pipe_file)
        .await
        .map_err(|_| Error::ConnectToFile(file_name))
}

fn place_runtime_file<P>(path: P) -> std::io::Result<PathBuf>
where
    P: AsRef<Path>,
{
    xdg::BaseDirectories::with_prefix("leftwm")?.place_runtime_file(path)
}

async fn timeout(mills: u64) {
    use tokio::time::{sleep, Duration};
    sleep(Duration::from_millis(mills)).await;
}
