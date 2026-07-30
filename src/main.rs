use core::time::Duration;

use std::sync::mpsc;

use artifacts_api::{Character, CooldownSchema};
use artifacts_core::{Command, CommandModifier};

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

// pub fn character_action_post_request_builder<'cn, 'an>(
//     character_action_uri: CharacterActionUri<'cn, 'an>,
// ) -> ureq::RequestBuilder<ureq::typestate::WithBody> {
//     apply_headers(ureq::post(character_action_uri))
// }

// pub struct CharacterActionEndpoint {
//     action_name: &'static str,
// }

// impl CharacterActionEndpoint {
//     pub fn into_post_request_for_character(
//         self,
//         c: &Character,
//     ) -> ureq::RequestBuilder<ureq::typestate::WithBody> {
//         apply_headers(ureq::post(CharacterActionUri {
//             character_name: c.name,
//             action_name: self.action_name,
//         }))
//     }
// }

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

// fn send_character_action_post_request<ResponseSchema>(
//     api_key: &str,
//     character_name: &str,
//     action_name: &str,
//     body: impl ureq::AsSendBody,
// ) -> anyhow::Result<ResponseSchema>
// where
//     ResponseSchema: serde::de::DeserializeOwned,
// {
//     apply_headers(
//         ureq::post(CharacterActionUri {
//             character_name,
//             action_name,
//         }),
//         api_key,
//     )
//     .send(body)
//     .map_err(Into::into)
//     .map(ureq::http::Response::into_body)
//     .map(ureq::Body::into_reader)
//     .and_then(|rdr| serde_json::from_reader(rdr).map_err(Into::into))
// }

// pub struct Coordinates(i64, i64);

// pub struct CharacterMoveActionRequestBody {
//     pub target_coordinates: Coordinates,
// }
// use serde::Serialize;

// use artifacts_api::ItemSlot;

// pub struct CharacterMoveActionRequest {
//     coordinates: CharacterMoveActionTargetCoordinates,
// }

// pub struct CharacterMoveActionTargetCoordinates {
//     x: i64,
//     y: i64,
// }

// impl CharacterActionRequest for CharacterMoveActionRequest {
//     fn body(&self) -> Box<[u8]> {
//         let (x, y) = (self.coordinates.x, self.coordinates.y);
//         serde_json::json!({"x": x, "y": y})
//             .to_string()
//             .into_bytes()
//             .into_boxed_slice()
//     }
//     fn post_request_builder_for_character(
//         &self,
//         c: &Character,
//     ) -> ureq::RequestBuilder<ureq::typestate::WithBody> {
//         CharacterActionEndpoint {
//             action_name: "move",
//         }
//         .into_post_request_for_character(c)
//     }

//     type ResponseSchema = artifacts_api::CharacterMovementResponseSchema;
// }

pub struct CharacterRestActionRequest;

// impl CharacterActionRequest for CharacterRestActionRequest {
//     type ResponseSchema = artifacts_api::CharacterRestResponseSchema;
// }

// pub trait CharacterActionEndpoint {
//     type Response;
//     fn action_uri_component(&self) -> ActionUriComponent;
//     fn post_request_builder(
//         &self,
//         character_name: &str,
//     ) -> ureq::RequestBuilder<ureq::typestate::WithBody> {
//         use ureq::http::Uri;
//         let uri = Uri::builder()
//             .scheme("https")
//             .authority("api.artifactsmmo.com")
//             .path_and_query(format!(
//                 "/my/{character_name}/action/{}",
//                 self.action_name_path_component()
//             ))
//             .build()?;
//         let req = apply_headers(ureq::post())
//     }
//     fn make_post_request(&self) -> anyhow::Result<Self::Response>;
// }
pub struct RestEndpoint {}

// impl CharacterActionEndpoint for RestEndpoint {
//     type Response = SkillResponseSchema;
//     fn action_name_path_component(&self) -> &'static str {
//         "rest"
//     }
//     fn url(&self) -> ureq::http::Uri {
//         use ureq::http::uri;
//     }
//     fn make_post_request(&self) -> anyhow::Result<Self::Response> {}
// }

#[allow(unused)]
const CHICKEN: (i32, i32) = (0, 1);
#[allow(unused)]
const ASH_TREE: (i32, i32) = (-1, 0);
#[allow(unused)]
const WEAPONCRAFTING_WORKSHOP: (i32, i32) = (2, 1);
pub struct CommandInterpreter<I> {
    pub it: I,
}
impl<I> CommandInterpreter<I>
where
    I: Iterator<Item = Command>,
{
    pub fn interpret_next(
        &mut self,
        api_key: &str,
        character_name: &str,
        // tracker: &mut artifacts_core::PlayerTracker,
        mut callback: impl FnMut(artifacts_core::ActionResponseDataSchema),
    ) -> anyhow::Result<()> {
        tracing::debug!("interpreting next");
        let Some(Command {
            name,
            modifier,
            arguments,
        }) = self.it.next()
        else {
            return Ok(());
        };

        let repeat_times = match modifier {
            Some(CommandModifier::Repeat(repeat_times)) => repeat_times,
            _ => 1,
        };

        for _ in 0..repeat_times {
            let res =
                artifacts_core::command_handler_router(api_key, character_name, &name, &arguments)?;
            callback(res);
            // match artifacts_core::command_handler_router(api_key, character_name, command_name, command_arguments) {
            //     Ok(res) => callback(res),
            //     Err(e) => {

            //     }
            // }
            // match handle_parsed_args(api_key, character_name, &name, &arguments)? {
            //     Some(res) => {
            //         // tracker.update_from_response_schema(&res);
            //         callback(res)
            //     }
            //     None => {
            //         tracing::warn!("empty response!")
            //     }
            // }
            // while tracker.cooldown.is_active() {
            //     tracing::info!(
            //         "[{repeat_index:2}/{repeat_times:2}] {remaining} / {total}",
            //         repeat_index = repeat_index + 1,
            //         repeat_times = repeat_times,
            //         remaining = tracker.cooldown.remaining().unwrap().as_seconds_f32(),
            //         total = tracker.cooldown.total().unwrap().as_seconds_f32()
            //     );
            //     let sleep_secs = 5u64;
            //     let sleep_dur = Duration::from_secs(sleep_secs);
            //     std::thread::sleep(sleep_dur);
            // }
        }
        Ok(())
    }
}

pub struct CommandQueue {
    pub rx: mpsc::Receiver<Command>,
}
pub struct CommandWorker {
    api_key: String,
    character_name: String,
    pub response_tx: mpsc::Sender<artifacts_core::ActionResponseDataSchema>,
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
        mpsc::Receiver<artifacts_core::ActionResponseDataSchema>,
    ) {
        let (cmd_tx, cmd_rx) = mpsc::channel();
        let (response_tx, response_rx) = mpsc::channel();
        (
            cmd_tx,
            CommandWorker {
                api_key,
                character_name,
                response_tx,
                cmd_rx,
            },
            response_rx,
        )
    }
    /// returns the `handle` to the thread, consuming the worker into it
    pub fn spawn(self) -> std::thread::JoinHandle<()> {
        std::thread::spawn(move || {
            // Track the active cooldown
            let mut cd_slot = None;
            loop {
                // use try-recv to not block, and continue on empty
                //
                // this polls the command-queue
                let artifacts_core::UnmodifiedCommand {
                    name: command_name,
                    arguments: command_arguments,
                } = match self.cmd_rx.try_recv() {
                    Ok(command) => command,
                    // this is an irrecoverable state for now
                    Err(mpsc::TryRecvError::Disconnected) => break,
                    // this simply implies we have
                    Err(mpsc::TryRecvError::Empty) => continue,
                };

                let response_schema = match artifacts_core::command_handler_router(
                    &self.api_key,
                    &self.character_name,
                    &command_name,
                    &command_arguments,
                ) {
                    Ok(response_schema) => response_schema,
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

                // check if the response implies some form of cooldown
                if let Some(cooldown_schema) = response_schema.cooldown_schema() {
                    cd_slot = Some(calculate_sleep_time(cooldown_schema));
                }

                // send the response back out
                self.response_tx
                    .send(response_schema)
                    .expect("failed to reply with response schema after handling command");

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

    // let mut cds = ActionTracker::new(0);
    let mut tracker = artifacts_core::PlayerTracker::default();
    // let mut cd = artifacts_core::CooldownTracker::new();

    let mut arg_parser = artifacts_core::ArgParser::new(&mut args);

    use std::sync::mpsc;

    // let (cmd_tx, mut cmd_queue) = CommandQueue::new();
    // let (res_tx, res_rx) = mpsc::channel::<artifacts_core::ActionResponseDataSchema>();

    let (cmd_tx, command_worker, response_rx) =
        CommandWorker::new(api_key.clone(), character_name.clone());
    let _command_handling_thread = command_worker.spawn();

    // let mut interpreter = CommandInterpreter {
    //     it: core::iter::from_fn(|| arg_parser.parse_next_command()),
    // };
    // loop {
    //     let interpreter_callback = |res: artifacts_core::ActionResponseDataSchema| {
    //         tracing::info!("handling response {res:#?}");
    //         tracker
    //             .update_from_response_schema(&res)
    //             .expect("corrupt response schema or sm");
    //         while tracker.cooldown.is_active() {
    //             tracing::info!(
    //                 "cooldown: {elapsed} / {total}",
    //                 elapsed = tracker.cooldown.elapsed().unwrap().as_seconds_f32(),
    //                 total = tracker.cooldown.total().unwrap().as_seconds_f32()
    //             );
    //             let sleep_secs = 5u64;
    //             let sleep_dur = Duration::from_secs(sleep_secs);
    //             std::thread::sleep(sleep_dur);
    //         }
    //         // cooldown bit
    //     };
    //     match interpreter.interpret_next(&api_key, &character_name, interpreter_callback) {
    //         // here we can do some other work now, we have interpreted the NEXT command, that can come from the internal iterator or some other source as we have proven here
    //         // naturally, we can write something which iterates based on some internal channel!
    //         Ok(()) => {}
    //         Err(e) => {
    //             tracing::error!("hit an error: {e}");
    //             break Err(e);
    //         }
    //     }
    // }?;

    /// returns `Some` if we handled any commands
    pub fn command_loop_once<I>(
        it: &mut I,
        cmd_tx: &mpsc::Sender<Command>,
        response_rx: &mpsc::Receiver<artifacts_core::ActionResponseDataSchema>,
        // tracker: &mut artifacts_core::PlayerTracker,
    ) -> Option<()>
    where
        I: Iterator<Item = Command>,
    {
        let cmd = it.next()?;

        let repeat_count = match &cmd.modifier {
            Some(CommandModifier::Repeat(times)) => *times as usize,
            _ => 1,
        };

        cmd_tx.send(cmd).expect("failed to send command");

        // if let Ok(value) = response_rx.try_recv() {
        //     tracker
        //         .update_from_response_schema(&value)
        //         .expect("failed to update!");
        // };

        Some(())
    }

    // let mut parser = core::iter::from_fn(|| arg_parser.parse_next_command());
    // loop {
    //     let Some(()) = command_loop_once(&mut parser, &cmd_tx, &res_rx) else {
    //         tracing::error!("hit end!");
    //         break;
    //     };
    // }
    // while let Some(()) = command_loop_once(&mut parser, cmd_tx, response_rx, &mut tracker) {}
    // let _parser_thread = std::thread::spawn(move || {
    //     while let Some(cmd) = arg_parser.parse_next_command() {
    //         let repeat_count = match &cmd.modifier {
    //             Some(CommandModifier::Repeat(times)) => *times as usize,
    //             _ => 1,
    //         };

    //         cmd_tx.send(cmd).expect("failed to send command");
    //     }
    // });

    let mut app = artifacts_tui::App::new(character_name.clone(), cmd_tx);
    // let mut term = artifacts_tui::ratatui::init();
    // loop {
    //     term.draw(|frame| app.draw(frame, &mut tracker).expect("failed to draw"))?;
    //     app.handle_events(&mut tracker)?;

    //     if !tracker.cooldown.is_active()
    //         && let Some(cmd) = arg_parser.parse_next_command()
    //     {
    //         let repeat_count = match &cmd.modifier {
    //             Some(CommandModifier::Repeat(times)) => *times as usize,
    //             _ => 1,
    //         };

    //         app.cmd_tx.send(cmd).expect("failed to send command");
    //     }
    // }
    artifacts_tui::ratatui::run(|terminal| {
        loop {
            if app.stop_flag.load(core::sync::atomic::Ordering::Acquire) {
                break;
            };

            let Ok(()) = app.run_once_inner(terminal, &mut tracker) else {
                break;
            };

            match response_rx.try_recv() {
                Ok(response_schema) => {
                    tracker
                        .update_from_response_schema(&character_name, &response_schema)
                        .expect("failed to update!");
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    break;
                }
                Err(_) => {
                    // println!("")
                }
            };

            if let Some(cmd) = arg_parser.parse_next_command() {
                let repeat_count = match &cmd.modifier {
                    Some(CommandModifier::Repeat(times)) => *times as usize,
                    _ => 1,
                };
                // println!("sending {repeat_count} x {cmd:#?}");
                for _ in 0..repeat_count {
                    let cmd = artifacts_core::UnmodifiedCommand {
                        name: cmd.name.clone(),
                        arguments: cmd.arguments.clone(),
                    };
                    app.cmd_tx
                        .send(cmd.clone())
                        .expect("failed to send command")
                }
            }

            // if !tracker.cooldown.is_active()
            //     && let Some(cmd) = arg_parser.parse_next_command()
            // {

            //     app.cmd_tx.send(cmd).expect("failed to send command");
            // }
        }
    });

    // while let Some(cmd) = arg_parser.parse_next_command() {
    //     let repeat_count = match &cmd.modifier {
    //         Some(CommandModifier::Repeat(times)) => *times as usize,
    //         _ => 1,
    //     };

    //     cmd_tx.send(cmd).expect("failed to send command");

    //     // repeat the command here, will likely fuck up on cooldowns however...
    //     // for cmd in core::iter::repeat_n(cmd, repeat_count) {
    //     // }

    //     if let Ok(value) = response_rx.try_recv() {
    //         tracker
    //             .update_from_response_schema(&value)
    //             .expect("failed to update!");
    //     };
    // }

    // while let Some(arg) = arg_parser.parse_next_command() {
    //     let Command {
    //         name: command_name,
    //         modifier: command_modifier,
    //         arguments: command_arguments,
    //     } = arg;

    //     let repeat_times = command_modifier
    //         .map(|modifier| match modifier {
    //             CommandModifier::Repeat(repeat_times) => repeat_times,
    //             #[expect(unreachable_patterns)]
    //             _ => 1,
    //         })
    //         .unwrap_or(1);

    //     for repeat_index in 0..repeat_times {
    //         match handle_parsed_args(&api_key, &character_name, &command_name, &command_arguments)?
    //         {
    //             Some(res) => {
    //                 tracing::info!("response was {res:#?}");
    //                 // update_player_cooldown_tracker(&res, &mut tracker.cooldown)?;
    //                 if let artifacts_core::ActionResponseDataSchema::Move(
    //                     character_movement_data_schema,
    //                 ) = &res
    //                 {
    //                     tracker.position.update_position_from_map_schema(
    //                         &character_movement_data_schema.destination,
    //                     )
    //                 }
    //             }
    //             None => {
    //                 tracing::warn!("empty response!")
    //             }
    //         }
    //         while tracker.cooldown.is_active() {
    //             tracing::info!(
    //                 "[{repeat_index:2}/{repeat_times:2}] {elapsed:.3} / {total:.3}",
    //                 repeat_index = repeat_index + 1,
    //                 repeat_times = repeat_times,
    //                 elapsed = tracker.cooldown.elapsed().unwrap().as_seconds_f32(),
    //                 total = tracker.cooldown.total().unwrap().as_seconds_f32()
    //             );
    //             let sleep_secs = 5u64;
    //             let sleep_dur = Duration::from_secs(sleep_secs);
    //             std::thread::sleep(sleep_dur);
    //         }
    //     }
    // }

    Ok(())
}

pub enum CommandArgument {
    Required(&'static str),
    Optional(&'static str),
}
