mod backend;
mod env;
mod frontend;
pub mod strategy;

use crate::{message::Message, Args};
use env::Env;
use std::sync::Arc;
use tokio::sync::mpsc::{self, unbounded_channel, UnboundedReceiver, UnboundedSender};
use tracing::{debug, error, info, trace};

pub enum TurnToSpeak {
    User,
    Bot,
}

pub struct App;

impl App {
    pub async fn run_until_exit(args: Args) -> Result<(), anyhow::Error> {
        let env = Arc::new(Env::new(&args)?);

        let (app_tx, mut app_rx) = unbounded_channel::<Event>();
        let (f_tx, f_rx) = unbounded_channel::<Event>();
        let (b_tx, b_rx) = unbounded_channel::<Event>();

        let mut frontend =
            frontend::State::new(f_rx, b_tx, app_tx.clone(), env.clone(), &args).await?;
        let mut backend =
            backend::State::new(b_rx, f_tx, app_tx.clone(), env.clone(), &args).await?;

        trace!("frontend and backend state has been initialized, starting main loop");

        'main_loop: loop {
            trace!("requesting frontend update");
            let frontend_fut = frontend::tick(&mut frontend, args.action());
            trace!("requesting backend update");
            let backend_fut = backend::tick(&mut backend, args.action());

            let (frontend_result, backend_result) = tokio::join!(frontend_fut, backend_fut);
            frontend_result?;
            trace!("frontend update done");
            backend_result?;
            trace!("backend update done");

            trace!("checking for app events");
            'event_loop: loop {
                match app_rx.try_recv() {
                    Ok(event) => {
                        if let Event::Quit = event {
                            frontend::quit(frontend)?;
                            debug!("frontend is done quitting");
                            backend::quit(backend)?;
                            debug!("backend is done quitting");
                            info!("Thanks for chatting!");

                            break 'main_loop;
                        }
                    }
                    Err(e) => match e {
                        mpsc::error::TryRecvError::Empty => {
                            break 'event_loop;
                        }
                        mpsc::error::TryRecvError::Disconnected => {
                            error!("app_rx channel closed");
                            break 'main_loop;
                        }
                    },
                }
            }
        }

        Ok(())
    }
}

type EventRx = UnboundedReceiver<Event>;
type EventTx = UnboundedSender<Event>;

#[derive(Debug)]
pub enum Event {
    /// Any handler receiving this event should put its affairs in order.
    Quit,
    UserMessage(String),
    ConversationUpdated(Vec<Message>),
    StatusUpdated(String),
}
