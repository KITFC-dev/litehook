//! This code is based on this example from tgt:
//! https://github.com/FedericoBruzzone/tgt/blob/main/examples/telegram.rs

use tdlib_rs::{
    enums::{AuthorizationState, MessageContent, Update},
    functions,
};
use tokio::sync::mpsc;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio_util::sync::CancellationToken;

use super::TelegramClientConfig;
use crate::{events::Event, model::Post};

pub struct TelegramClient {
    pub cfg: TelegramClientConfig,
    pub client_id: i32,

    tx: mpsc::Sender<Event>,
    auth_rx: UnboundedReceiver<AuthorizationState>,
    auth_tx: UnboundedSender<AuthorizationState>,

    shutdown: CancellationToken,
}

impl TelegramClient {
    pub fn new(cfg: TelegramClientConfig, tx: mpsc::Sender<Event>) -> Self {
        let client_id = tdlib_rs::create_client();
        let shutdown = CancellationToken::new();
        let (auth_tx, auth_rx) = mpsc::unbounded_channel();

        Self {
            cfg,
            tx,
            auth_rx,
            auth_tx,
            client_id,
            shutdown,
        }
    }

    pub async fn run(&mut self) -> anyhow::Result<()> {
        let auth_tx = self.auth_tx.clone();
        let shutdown = self.shutdown.clone();
        let client_id = self.client_id;
        let tx = self.tx.clone();
        let webhook_url = self.cfg.webhook_url.clone();
        let channels = self.cfg.channel_ids.clone();

        // Spawn blocking because TDLib's tdlib_rs::receive() is a blocking function.
        tokio::task::spawn_blocking(move || {
            loop {
                if shutdown.is_cancelled() {
                    break;
                }
                if let Some((update, id)) = tdlib_rs::receive() {
                    if id != client_id {
                        continue;
                    }
                    match update {
                        Update::AuthorizationState(u) => {
                            let _ = auth_tx.send(u.authorization_state);
                        }
                        Update::NewMessage(u) => {
                            let msg = &u.message;
                            let chat_id = msg.chat_id.to_string();

                            // Check if its the message we want
                            if !channels.contains(&chat_id) || msg.is_outgoing {
                                continue;
                            }

                            // Get author
                            let author_id = match &msg.sender_id {
                                tdlib_rs::enums::MessageSender::User(u) => {
                                    Some(u.user_id.to_string())
                                }
                                tdlib_rs::enums::MessageSender::Chat(c) => {
                                    Some(c.chat_id.to_string())
                                }
                            };

                            // Send message to event handler
                            match &msg.content {
                                MessageContent::MessageText(m) => {
                                    let _ = tx.blocking_send(Event::NewMessage(
                                        webhook_url.clone(),
                                        Post {
                                            id: msg.chat_id.to_string(),
                                            author: author_id,
                                            text: Some(m.text.text.clone()),
                                            ..Default::default()
                                        },
                                    ));
                                }

                                MessageContent::MessagePhoto(m) => {
                                    let _ = tx.blocking_send(Event::NewMessage(
                                        webhook_url.clone(),
                                        Post {
                                            id: msg.chat_id.to_string(),
                                            author: author_id,
                                            text: Some(m.caption.text.clone()),
                                            media: Some(
                                                m.photo
                                                    .sizes
                                                    .iter()
                                                    .map(|s| s.photo.id.to_string())
                                                    .collect(),
                                            ),
                                            ..Default::default()
                                        },
                                    ));
                                }

                                MessageContent::MessageVideo(m) => {
                                    let _ = tx.blocking_send(Event::NewMessage(
                                        webhook_url.clone(),
                                        Post {
                                            id: msg.chat_id.to_string(),
                                            author: author_id,
                                            text: Some(m.caption.text.clone()),
                                            media: Some(vec![m.video.video.id.to_string()]),
                                            ..Default::default()
                                        },
                                    ));
                                }

                                _ => {}
                            }
                        }
                        _ => {}
                    }
                }
            }
        });

        self.send_tdlib_parameters().await?;
        self.handle_authorization_state().await?;
        functions::set_log_verbosity_level(2, self.client_id)
            .await
            .unwrap();

        tracing::info!("telegram client {} authenticated", self.cfg.id);
        Ok(())
    }

    async fn send_tdlib_parameters(&self) -> anyhow::Result<()> {
        let db_dir = format!("data/td/{}/db", self.cfg.id);
        let files_dir = format!("data/td/{}/files", self.cfg.id);

        tokio::fs::create_dir_all(&db_dir).await?;
        tokio::fs::create_dir_all(&files_dir).await?;

        functions::set_tdlib_parameters(
            false,
            db_dir,
            files_dir,
            String::new(),
            true,
            true,
            true,
            false,
            self.cfg.api_id,
            self.cfg.api_hash.clone(),
            "en".to_string(),
            "Litehook Telegram Client".to_string(),
            String::new(),
            "1.0".to_string(),
            self.client_id,
        )
        .await
        .map_err(|e| anyhow::anyhow!("set_tdlib_parameters failed: {}", e.message))?;

        Ok(())
    }

    async fn handle_authorization_state(&mut self) -> anyhow::Result<()> {
        while let Some(state) = self.auth_rx.recv().await {
            match state {
                // AuthorizationState::WaitTdlibParameters => {
                //     self.send_tdlib_parameters().await?;
                // }
                AuthorizationState::WaitPhoneNumber => {
                    tracing::info!("sending phone number");
                    loop {
                        match functions::set_authentication_phone_number(
                            self.cfg.phone_number.clone(),
                            None,
                            self.client_id,
                        )
                        .await
                        {
                            Ok(_) => break,
                            Err(e) => tracing::error!("phone error: {}", e.message),
                        }
                    }
                }
                AuthorizationState::WaitCode(info) => {
                    tracing::info!(
                        "code sent via {:?} to {}",
                        info.code_info.r#type,
                        info.code_info.phone_number
                    );
                    loop {
                        let (ntf_tx, ntf_rx) = tokio::sync::oneshot::channel();
                        if self
                            .tx
                            .send(Event::InputRequest(
                                "Enter Telegram code:".to_string(),
                                ntf_tx,
                            ))
                            .await
                            .is_err()
                        {
                            break;
                        }
                        let code = match ntf_rx.await {
                            Ok(c) => c,
                            Err(_) => break,
                        };
                        match functions::check_authentication_code(code, self.client_id).await {
                            Ok(_) => break,
                            Err(e) => tracing::error!("code error: {}", e.message),
                        }
                    }
                }
                AuthorizationState::WaitPassword(_) => loop {
                    let (ntf_tx, ntf_rx) = tokio::sync::oneshot::channel();
                    if self
                        .tx
                        .send(Event::InputRequest(
                            "Enter 2FA password:".to_string(),
                            ntf_tx,
                        ))
                        .await
                        .is_err()
                    {
                        break;
                    }
                    let pw = match ntf_rx.await {
                        Ok(p) => p,
                        Err(_) => break,
                    };
                    match functions::check_authentication_password(pw, self.client_id).await {
                        Ok(_) => break,
                        Err(e) => tracing::error!("2fa error: {}", e.message),
                    }
                },
                AuthorizationState::Ready => break,
                AuthorizationState::Closed => {
                    self.shutdown.cancel();
                    break;
                }
                _ => {}
            }
        }

        Ok(())
    }

    pub async fn stop(&mut self) -> anyhow::Result<()> {
        tracing::info!("stopping client {}", self.cfg.id);
        self.shutdown.cancel();
        functions::close(self.client_id)
            .await
            .map_err(|e| anyhow::anyhow!(e.message))?;
        Ok(())
    }
}
