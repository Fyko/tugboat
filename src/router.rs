use std::pin::Pin;
use std::sync::RwLock;
use std::{collections::HashMap, process::Command};

use crate::command::CommandPath;
use crate::responder::Responder;
use anyhow::{Error, Result};
use axum::{
    body::{Body, Bytes},
    http::{Request, Response},
};
use ed25519_dalek::{PublicKey, Signature, Verifier};
use futures::Future;
use twilight_model::application::interaction::application_command::{
    CommandData, CommandOptionValue,
};
use twilight_model::application::interaction::{Interaction, InteractionData, InteractionType};

// pub type CommandHandler<T = Box<CommandData>> = &'a dyn Fn<T>(T) -> impl Responder;

pub trait CommandHandler<Args>: Clone + 'static {
    type Output;
    type Future: Future<Output = Self::Output>;

    fn call(&self, args: Args) -> Self::Future;
}

impl<Func, Fut> CommandHandler<CommandData> for Func
where
    Func: Fn(CommandData) -> Fut + Clone + 'static,
    Fut: Future,
{
    type Output = Fut::Output;
    type Future = Fut;

    fn call(&self, data: CommandData) -> Self::Future {
        (self)(data)
    }
}

type BoxedCommandHandler = Box<dyn Fn(CommandData) -> Pin<Box<dyn Future<Output = Response<()>> + Send>> + Send + Sync>;

pub struct InteractionRouter {
    pub commands: RwLock<HashMap<String, Box<dyn Fn(CommandData) -> Pin<Box<Response<()>>>>>>,
}

pub trait FromRequest: Sized {
    type Error: Into<Error>;
    type Future: Future<Output = Result<Self>>;
}

impl InteractionRouter {
    pub fn new() -> Self {
        InteractionRouter {
            commands: RwLock::new(HashMap::new()),
        }
    }

    /// Register a command handler
    pub fn command(&self, name: impl Into<CommandPath>, func: impl Fn(CommandData) -> Response<()>) -> &Self {
        let path: CommandPath = name.into();
        let hash = match path {
            CommandPath::Root(x) => x.to_string(),
            CommandPath::Sub(x) => x.join("|"),
        };

        if let Ok(mut lock) = self.commands.write() {
            let boxed = Box::new(move |command: CommandData| {
                Box::pin(func(command))
            });

            lock.insert(hash, boxed);
        }

        self
    }

    async fn handle_command(&self, data: Box<CommandData>) -> Result<Response<()>> {
        let elements = vec![data.name.clone()];

        // TODO: subcommands
        let _: Vec<bool> = data
            .options
            .iter()
            .map(|opts| {
                match &opts.value {
                    CommandOptionValue::SubCommandGroup(x) => {
                        println!("subcommand group: {:?}", x);
                    }
                    CommandOptionValue::SubCommand(x) => {
                        println!("subcommand: {:?}", x);
                    }
                    _ => {}
                }

                true
            })
            .collect();

        let hash = elements.join("|");

        match self.commands.read() {
            Ok(lock) => {
                if let Some(handler) = lock.get(&hash) {
                    return handler(data);
                }

                tracing::error!("No handler found for command {}", hash);

                anyhow::anyhow!("No handler found for command {}", hash);
            }
            _ => unreachable!(),
        }
    }

    pub async fn handle_interaction(&self, interaction: Interaction) -> Result<Response<()>> {
        match interaction.kind {
            InteractionType::Ping => Response::ok("{ \"type\": 1 }"),
            InteractionType::ApplicationCommand => {
                let data = match interaction.data {
                    Some(InteractionData::ApplicationCommand(data)) => Some(data),
                    _ => None,
                }
                .expect("Expected application command data");

                self.handle_command(data).await
            }
            InteractionType::ApplicationCommandAutocomplete => todo!(),
            InteractionType::MessageComponent => todo!(),
            _ => {
                tracing::error!("Unhandled interaction type received! {:?}", interaction);
                Response::error("Unhandled interaction type received!", 500)
            }
        }
    }

    pub async fn handle_request(&self, req: Request<Body>, body: Bytes) -> Result<Response<()>> {
        let public_key = std::env::var("DISCORD_PUBLIC_KEY")?.to_string();
        let public_key = PublicKey::from_bytes(hex::decode(public_key).unwrap().as_ref()).unwrap();

        let headers = req.headers().to_owned();
        // inspired by gearbot2 <3
        if let (Some(signature), Some(timestamp)) = (
            headers.get("X-Signature-Ed25519"),
            headers.get("X-Signature-Timestamp"),
        ) {
            if let Ok(decoded) = hex::decode(signature) {
                if let Ok(req_signature) = Signature::from_bytes(&decoded) {
                    if public_key
                        .verify(&[timestamp.as_bytes(), &body].concat(), &req_signature)
                        .is_ok()
                    {
                        match serde_json::from_slice(&body) {
                            Ok(interaction) => {
                                tracing::info!("interaction: {:#?}", interaction);
                                return self.handle_interaction(interaction).await;
                            }
                            Err(e) => {
                                tracing::info!("Error deserializing interaction: {}", e);
                                return Response::error("Error deserializing interaction", 400);
                            }
                        }
                    }
                }
            }
        }

        Response::error("Unauthorized.", 401)
    }
}

#[test]
fn test_command_path() {
    let router = InteractionRouter::new();

    router.command("test", |_| {
        println!("test");

        Response::ok("test")
    });

    router.command(vec!["test", "sub"], |_| {
        println!("test sub");

        Response::ok("test sub")
    });

    assert!(router.commands.read().unwrap().contains_key("test"));
    assert!(router.commands.read().unwrap().contains_key("test|sub"));
}
