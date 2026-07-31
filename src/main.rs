use core::time::Duration;

use std::sync::mpsc;

use artifacts_api::{Character, CooldownSchema};
use artifacts_core::Command;

fn load_dotenv_file() -> dotenvy::Result<()> {
    use dotenvy::{dotenv, from_path};
    dotenv().and_then(from_path)
}

pub struct CharacterActionUri<'cn, 'an> {
    character_name: &'cn str,
    action_name: &'an str,
}

impl<'cn, 'an> TryFrom<CharacterActionUri<'cn, 'an>> for ureq::http::Uri {
    type Error = ureq::http::Error;
    fn try_from(
        CharacterActionUri {
            character_name,
            action_name,
        }: CharacterActionUri<'cn, 'an>,
    ) -> Result<Self, Self::Error> {
        ureq::http::Uri::builder()
            .scheme("https")
            .authority("api.artifactsmmo.com")
            .path_and_query(format!("/my/{character_name}/action/{action_name}"))
            .build()
    }
}

pub trait CharacterActionRequest {
    fn body(&self) -> Box<[u8]> {
        [].into()
    }

    fn post_request_builder_for_character(
        &self,
        c: &Character,
    ) -> ureq::RequestBuilder<ureq::typestate::WithBody>;

    type ResponseSchema: serde::de::DeserializeOwned;
    fn make_post_request(&self, c: &Character) -> anyhow::Result<Self::ResponseSchema> {
        self.post_request_builder_for_character(c)
            .send(self.body().as_ref())
            .map_err(Into::into)
            .map(ureq::http::Response::into_body)
            .map(ureq::Body::into_reader)
            .and_then(|rdr| serde_json::from_reader(rdr).map_err(Into::into))
    }
}

#[allow(unused)]
const CHICKEN: (i32, i32) = (0, 1);
#[allow(unused)]
const ASH_TREE: (i32, i32) = (-1, 0);
#[allow(unused)]
const WEAPONCRAFTING_WORKSHOP: (i32, i32) = (2, 1);

pub struct CommandWorker {
    api_key: String,
    character_name: String,
    pub cmd_rx: mpsc::Receiver<artifacts_core::UnmodifiedCommand>,
}

fn calculate_sleep_time(cooldown_schema: &CooldownSchema) -> Duration {
    let started_at = chrono::DateTime::parse_from_rfc3339(&cooldown_schema.started_at)
        .expect("bad started at timestamp")
        .to_utc();
    let expiration = chrono::DateTime::parse_from_rfc3339(&cooldown_schema.expiration)
        .expect("bad expiration timestamp")
        .to_utc();
    Duration::from_secs_f64(
        expiration
            .signed_duration_since(started_at)
            .as_seconds_f64(),
    )
}

impl CommandWorker {
    pub fn new(
        api_key: String,
        character_name: String,
    ) -> (
        mpsc::Sender<artifacts_core::UnmodifiedCommand>,
        CommandWorker,
    ) {
        let (cmd_tx, cmd_rx) = mpsc::channel();
        (
            cmd_tx,
            CommandWorker {
                api_key,
                character_name,
                cmd_rx,
            },
        )
    }
    /// returns the `handle` to the thread, consuming the worker into it
    pub fn spawn(
        self,
        mut data_callback: impl FnMut(artifacts_core::network::ActionResponseDataSchema)
        + Send
        + Sync
        + 'static,
        mut error_callback: impl for<'a> FnMut(&'a str, artifacts_core::network::CodedErrorObject)
        + Send
        + Sync
        + 'static,
    ) -> std::thread::JoinHandle<()> {
        std::thread::spawn(move || {
            // Track the active cooldown
            let mut cd_slot = None;
            while let Ok(artifacts_core::UnmodifiedCommand {
                name: command_name,
                arguments: command_arguments,
            }) = self.cmd_rx.recv()
            {
                match artifacts_core::network::command_handler_router(
                    &self.api_key,
                    &self.character_name,
                    &command_name,
                    &command_arguments,
                ) {
                    Ok(response_schema) => {
                        // update cooldown and give data callback
                        if let Some(cooldown_schema) = response_schema.cooldown_schema() {
                            cd_slot = Some(calculate_sleep_time(cooldown_schema));
                        };
                        data_callback(response_schema);
                        // fallthrough to cooldown branch
                    }
                    Err(artifacts_core::network::ActionRequestError::ApiError(e)) => {
                        // give to error-callback
                        error_callback(&command_name, e);
                        // skip cooldown, next iteration
                        continue;
                    }
                    // more severe situation, something failed which we cannot truly handle...
                    Err(e) => {
                        tracing::warn!("failed to handle command: {e}");
                        // go on from this command, we may now loop on multiple
                        // cooldown error states for a bit, this needs to be handled!
                        //
                        // i.e. more instrumentation of this thread for example,
                        //      a communication bus for errors back to UI
                        continue;
                    }
                };

                // before continuing to the next iteration to check if we have pending command,
                // sleep for the cooldown (if any) of the just invoked command
                //
                // We make sure to take, leaving the Option empty
                if let Some(current_cmd_cooldown_dur) = cd_slot.take() {
                    std::thread::sleep(current_cmd_cooldown_dur);
                }
            }
        })
    }
}

fn main() -> anyhow::Result<()> {
    // tracing_subscriber::fmt().init();
    let log_file_path = format!(
        "logs/artifacts-mmorpg-{}.log",
        chrono::Local::now().format("%d%m%YT%H%M%S")
    );
    println!("filename {log_file_path:?}");
    let logfile = std::fs::File::create_new(log_file_path)?;
    let (non_blocking, _guard) = tracing_appender::non_blocking(logfile);
    tracing_subscriber::fmt().with_writer(non_blocking).init();
    use std::env;
    load_dotenv_file().expect("failed to load .env file!");
    let api_key =
        env::var("ARTIFACTS_API_TOKEN").expect("ARTIFACTS_API_TOKEN missing from environment!");

    let mut args = std::env::args();
    let _program_name = args.next().unwrap();

    let character_name = args.next().unwrap();

    tracing::info!("character name is {character_name:?}");
    tracing::info!("args are {args:?}");

    let mut tracker = artifacts_core::PlayerTracker::default();

    let mut arg_parser = artifacts_core::LineParser::new(&mut args);

    use std::sync::mpsc;

    let (response_tx, response_rx) =
        mpsc::channel::<artifacts_core::network::ActionResponseDataSchema>();

    let (cmd_tx, task_worker) = CommandWorker::new(api_key.clone(), character_name.clone());
    // just drain the damn args
    while let Some(cmd_iter) = arg_parser
        .parse_next_command()
        .map(Command::into_unmodified)
    {
        for cmd in cmd_iter {
            cmd_tx.send(cmd).expect("failed this damn");
        }
    }

    let _task_handling_thread = task_worker.spawn(
        move |response_schema| {
            response_tx
                .send(response_schema)
                .expect("failed to send response!");
        },
        |command_name, error| tracing::info!("command {command_name:?} hit error {error:?}"),
    );

    artifacts_tui::App::new(character_name.clone(), cmd_tx).run(
        character_name,
        response_rx,
        &mut tracker,
    )?;

    Ok(())
}

// pub enum CommandArgument {
//     Required(&'static str),
//     Optional(&'static str),
// }
