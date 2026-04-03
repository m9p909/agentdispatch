use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use tokio::task::JoinSet;
use uuid::Uuid;

use crate::error::Result;
use crate::messages::message_service::MessageService;
use crate::sessions::session_service::SessionService;
use crate::telegram::db as telegram_db;
use crate::telegram::db::TelegramConfig;
use crate::telegram::telegram_adapter::{TelegramAdapter, TelegramUpdate};
use crate::telegram::telegram_registry::TelegramRegistry;
use crate::telegram::telegram_service::TelegramConnectorService;

fn backoff(failures: u32) -> Duration {
    // Exponential backoff: 2^failures seconds, capped at 5 minutes.
    Duration::from_secs(2u64.pow(failures.min(8)).min(300))
}

// ===== Supervisor =====

pub struct TelegramSupervisor {
    registry: Arc<TelegramRegistry>,
    telegram: TelegramAdapter,
    connector_service: TelegramConnectorService,
    session_service: SessionService,
    message_service: MessageService,
    basic_user_id: Uuid,
}

impl TelegramSupervisor {
    pub fn new(
        registry: Arc<TelegramRegistry>,
        telegram: TelegramAdapter,
        connector_service: TelegramConnectorService,
        session_service: SessionService,
        message_service: MessageService,
        basic_user_id: Uuid,
    ) -> Self {
        Self {
            registry,
            telegram,
            connector_service,
            session_service,
            message_service,
            basic_user_id,
        }
    }

    /// Spawns the supervisor as a background tokio task.
    pub fn start(self) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move { self.supervisor_loop().await })
    }

    async fn supervisor_loop(self) {
        let mut join_set: JoinSet<Uuid> = JoinSet::new();
        // In-process guard against double-spawning the same connector.
        let mut active: HashSet<Uuid> = HashSet::new();

        tracing::info!(
            "TelegramSupervisor started (instance {})",
            self.registry.instance_id
        );

        loop {
            // Reap finished/panicked tasks and update the active set.
            while let Some(result) = join_set.try_join_next() {
                match result {
                    Ok(connector_id) => {
                        active.remove(&connector_id);
                        tracing::info!("Connector task {} finished", connector_id);
                    }
                    Err(e) => {
                        tracing::error!("Connector task panicked: {}", e);
                        // We don't know which connector panicked, so clear the active
                        // set to allow re-claiming on the next cycle.
                        active.clear();
                    }
                }
            }

            // Discover and claim connectors whose lease is expired or unclaimed.
            match self.registry.list_claimable().await {
                Err(e) => tracing::error!("list_claimable failed: {}", e),
                Ok(claimable) => {
                    if claimable.is_empty() {
                        tracing::debug!("No connectors to claim");
                    }

                    for config in claimable {
                        if active.contains(&config.id) {
                            continue;
                        }

                        match self.registry.try_claim(config.id).await {
                            Err(e) => {
                                tracing::error!("try_claim failed for {}: {}", config.id, e)
                            }
                            Ok(false) => {} // Another instance won the race
                            Ok(true) => {
                                tracing::info!(
                                    "Claimed connector {} (agent {})",
                                    config.id,
                                    config.agent_id
                                );

                                // Decrypt the bot token before spawning so the task
                                // doesn't need access to CryptoService.
                                let token =
                                    match self.connector_service.get_decrypted_token(config.id).await {
                                        Ok(t) => t,
                                        Err(e) => {
                                            tracing::error!(
                                                "Failed to decrypt token for {}: {}",
                                                config.id,
                                                e
                                            );
                                            let _ = self.registry.release(config.id).await;
                                            continue;
                                        }
                                    };

                                active.insert(config.id);
                                let connector_id = config.id;

                                let reg = self.registry.clone();
                                let tg = self.telegram.clone();
                                let cs = self.connector_service.clone();
                                let ss = self.session_service.clone();
                                let ms = self.message_service.clone();
                                let uid = self.basic_user_id;

                                join_set.spawn(async move {
                                    run_connection(config, token, reg, tg, cs, ss, ms, uid).await;
                                    connector_id
                                });
                            }
                        }
                    }
                }
            }

            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }
}

// ===== Per-connection poller task =====

async fn run_connection(
    config: TelegramConfig,
    decrypted_token: String,
    registry: Arc<TelegramRegistry>,
    telegram: TelegramAdapter,
    connector_service: TelegramConnectorService,
    session_service: SessionService,
    message_service: MessageService,
    basic_user_id: Uuid,
) {
    let connector_id = config.id;
    let mut failures: u32 = 0;

    let mut offset = match registry.get_offset(connector_id).await {
        Ok(o) => o,
        Err(e) => {
            tracing::error!("Failed to get offset for {}: {}", connector_id, e);
            let _ = registry.release(connector_id).await;
            return;
        }
    };

    // Spawn a heartbeat sub-task that renews the lease every 10 seconds.
    let hb_registry = registry.clone();
    let hb_handle = tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(10));
        interval.tick().await; // skip the immediate first tick
        loop {
            interval.tick().await;
            if let Err(e) = hb_registry.heartbeat(connector_id).await {
                tracing::error!("Heartbeat failed for {}: {}", connector_id, e);
            }
        }
    });

    tracing::info!(
        "Polling started for connector {} (agent {}), offset={}",
        connector_id,
        config.agent_id,
        offset
    );

    loop {
        let updates = telegram.get_updates(&decrypted_token, offset).await;

        let updates = match updates {
            Ok(u) => {
                failures = 0;
                u
            }
            Err(e) => {
                failures += 1;
                let delay = backoff(failures);
                tracing::error!(
                    "get_updates failed for {} (attempt {}): {}. Retrying in {:?}",
                    connector_id,
                    failures,
                    e,
                    delay
                );
                if failures >= 10 {
                    tracing::error!(
                        "Too many consecutive failures for connector {}, stopping",
                        connector_id
                    );
                    break;
                }
                tokio::time::sleep(delay).await;
                continue;
            }
        };

        if updates.is_empty() {
            continue;
        }

        // Advance offset BEFORE processing — crash-safe re-delivery on restart.
        let new_offset = updates.iter().map(|u| u.update_id).max().unwrap() + 1;
        if let Err(e) = registry.save_offset(connector_id, new_offset).await {
            tracing::error!("Failed to save offset for {}: {}", connector_id, e);
        }
        offset = new_offset;

        for update in &updates {
            if let Err(e) = process_update(
                update,
                &config,
                &decrypted_token,
                &telegram,
                &connector_service,
                &session_service,
                &message_service,
                basic_user_id,
            )
            .await
            {
                tracing::error!(
                    "Failed to process update {} for connector {}: {}",
                    update.update_id,
                    connector_id,
                    e
                );
            }
        }
    }

    hb_handle.abort();
    if let Err(e) = registry.release(connector_id).await {
        tracing::error!("Failed to release connector {}: {}", connector_id, e);
    }
    tracing::info!("Connector {} stopped", connector_id);
}

// ===== Update processing =====

async fn process_update(
    update: &TelegramUpdate,
    config: &TelegramConfig,
    decrypted_token: &str,
    telegram: &TelegramAdapter,
    connector_service: &TelegramConnectorService,
    session_service: &SessionService,
    message_service: &MessageService,
    basic_user_id: Uuid,
) -> Result<()> {
    let message = match &update.message {
        Some(m) => m,
        None => return Ok(()), // Not a message update (e.g. edited_message, channel_post)
    };

    let text = match &message.text {
        Some(t) => t.clone(),
        None => return Ok(()), // Photo, sticker, etc. — skip
    };

    let from = match &message.from {
        Some(u) => u,
        None => return Ok(()), // Anonymous sender
    };

    if from.is_bot {
        return Ok(()); // Ignore other bots
    }

    let telegram_user_id = from.id;
    let chat_id = message.chat.id;
    let telegram_message_id = message.message_id;

    // Whitelist check
    let whitelist = connector_service.get_whitelist(config.agent_id).await?;
    if !whitelist.contains(&telegram_user_id) {
        tracing::info!(
            "Telegram user {} not whitelisted for agent {}",
            telegram_user_id,
            config.agent_id
        );
        let _ = telegram
            .send_message(
                decrypted_token,
                chat_id,
                "You are not authorized to use this bot.",
            )
            .await;
        return Ok(());
    }

    // Idempotency: insert BEFORE processing so a crash during processing results in
    // re-delivery (safe) rather than lost processing.
    let pool = connector_service.db.get_pool();
    let is_new =
        telegram_db::insert_processed_update(pool, config.id, telegram_message_id).await?;
    if !is_new {
        tracing::debug!(
            "Skipping already-processed message {} for connector {}",
            telegram_message_id,
            config.id
        );
        return Ok(());
    }

    // Find or create the session for this (agent, telegram_user) pair
    let session_id =
        find_or_create_session(session_service, pool, config.agent_id, basic_user_id, telegram_user_id)
            .await?;

    // Show typing indicator immediately, then keep refreshing it every 4 s
    // while the LLM processes (the indicator expires on Telegram's side after ~5 s).
    let _ = telegram.send_typing(decrypted_token, chat_id).await;
    let typing_tg = telegram.clone();
    let typing_token = decrypted_token.to_string();
    let typing_handle = tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(4)).await;
            let _ = typing_tg.send_typing(&typing_token, chat_id).await;
        }
    });

    // Run the full streaming + agentic loop, collect the final assistant text
    let ai_response = message_service.stream_to_text(session_id, text).await;
    typing_handle.abort();
    let ai_response = ai_response?;

    telegram
        .send_message(decrypted_token, chat_id, &ai_response)
        .await?;

    tracing::info!(
        "Processed Telegram message {} from user {} for agent {}",
        telegram_message_id,
        telegram_user_id,
        config.agent_id
    );

    Ok(())
}

async fn find_or_create_session(
    session_service: &SessionService,
    pool: &sqlx::PgPool,
    agent_id: Uuid,
    user_id: Uuid,
    telegram_user_id: i64,
) -> Result<Uuid> {
    let title = format!("telegram:{}", telegram_user_id);

    if let Some(session) =
        telegram_db::find_session_by_agent_and_title(pool, agent_id, &title).await?
    {
        return Ok(session.id);
    }

    let resp = session_service
        .create_session(agent_id, user_id, Some(&title))
        .await?;
    Ok(resp.id)
}
