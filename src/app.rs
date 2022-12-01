mod backend;
mod frontend;

use backend::BackendState;
use frontend::FrontendState;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender, self};
use tracing::{debug, info, error};

use crate::message::Message;

pub struct App {}

impl App {
    pub async fn run_until_exit() -> Result<(), anyhow::Error> {
        let (app_tx, mut app_rx) = unbounded_channel::<Event>();
        let (f_tx, f_rx) = unbounded_channel::<Event>();
        let (b_tx, b_rx) = unbounded_channel::<Event>();

        let mut frontend = FrontendState::new(f_rx, b_tx, app_tx.clone()).await?;
        let mut backend = BackendState::new(b_rx, f_tx, app_tx).await?;

        'main_loop: loop {
            let frontend_fut = frontend.tick();
            let backend_fut = backend.tick();

            let (frontend_result, backend_result) = tokio::join!(frontend_fut, backend_fut);
            frontend_result?;
            backend_result?;

            'event_loop: loop {
                match app_rx.try_recv() {
                    Ok(event) => match event {
                            Event::Quit => {
                                frontend.quit().await?;
                                debug!("frontend is done quitting");
                                backend.quit().await?;
                                debug!("backend is done quitting");
                                info!("Thanks for chatting!");
    
                                break 'main_loop;
                            }
                            _ => {}
                    },
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

enum Event {
    /// Any handler receiving this event should put its affairs in order.
    Quit,
    UserMessage(String),
    BotMessage(Message),
    ConversationUpdated(Vec<Message>),
    StatusUpdated(String),
}
