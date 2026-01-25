use std::sync::Arc;

use poise::serenity_prelude::{self as serenity, GatewayIntents, GuildId};
use sqlx::PgPool;
use tracing::{error, info, warn};

use crate::bot::data::Data;
use crate::bot::error::Error;
use crate::commands;
use crate::config::Settings;
use crate::handlers::event_handler::event_handler;
use crate::services::jtc::{channel_deleter, deadline_tracker, queue};

pub async fn run(settings: Settings, pool: PgPool) -> Result<(), Error> {
    let data = Arc::new(Data::new(pool, settings.clone()));

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![
                commands::setup::setup(),
                commands::stats::stats(),
                commands::owner::mute::mute(),
                commands::owner::mute::unmute(),
                commands::owner::ban::vcban(),
                commands::owner::transfer::transfer(),
                commands::owner::rename::rename(),
                commands::owner::retag::retag(),
            ],
            prefix_options: poise::PrefixFrameworkOptions {
                prefix: None, // Disable prefix commands - only use slash commands
                ..Default::default()
            },
            event_handler: |ctx, event, framework, data| {
                Box::pin(event_handler(ctx, event, framework, data))
            },
            on_error: |error| {
                Box::pin(async move {
                    match error {
                        poise::FrameworkError::Command { error, ctx, .. } => {
                            error!("Command error: {:?}", error);
                            let _ = ctx.say(format!("Error: {}", error)).await;
                        }
                        poise::FrameworkError::ArgumentParse { error, ctx, .. } => {
                            let _ = ctx.say(format!("Invalid argument: {}", error)).await;
                        }
                        poise::FrameworkError::UnknownCommand { .. } => {
                            // Ignore unknown command errors - bot only uses slash commands
                            // This happens when users ping the bot or use prefix commands
                        }
                        err => {
                            error!("Framework error: {:?}", err);
                        }
                    }
                })
            },
            ..Default::default()
        })
        .setup(|ctx, ready, framework| {
            Box::pin(async move {
                info!("Bot connected as {}", ready.user.name);

                // Clean up orphaned voice channels from previous session
                match channel_deleter::cleanup_orphaned_channels(&ctx.http, &data.pool, &data).await {
                    Ok((deleted, restored)) => {
                        if deleted > 0 || restored > 0 {
                            info!(
                                "Startup cleanup: deleted {} orphaned channels, restored {} to cache",
                                deleted, restored
                            );
                        }
                    }
                    Err(e) => {
                        warn!("Failed to clean up orphaned channels: {:?}", e);
                    }
                }

                // Start background task for VC naming deadline enforcement
                deadline_tracker::spawn_deadline_checker(ctx.http.clone(), data.clone());
                info!("Started VC naming deadline checker");

                // Create JTC queue for processing users waiting in JTC channels
                let (queue_tx, queue_rx) = tokio::sync::mpsc::unbounded_channel();
                
                // Check for users in JTC channels and queue them
                let ctx_clone = ctx.clone();
                let data_clone = data.clone();
                tokio::spawn(async move {
                    // Wait a few seconds for cache to populate
                    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
                    
                    match queue::check_jtc_channels_on_startup(&ctx_clone, &data_clone, &queue_tx).await {
                        Ok(count) => {
                            if count > 0 {
                                info!("Queued {} users waiting in JTC channels", count);
                            }
                        }
                        Err(e) => {
                            warn!("Failed to check JTC channels on startup: {:?}", e);
                        }
                    }
                });

                // Start queue processor
                queue::spawn_queue_processor(ctx.clone(), data.clone(), queue_rx);
                info!("Started JTC queue processor");

                // Check for empty channels after a short delay (to allow cache to populate)
                let ctx_clone = ctx.clone();
                let data_clone = data.clone();
                tokio::spawn(async move {
                    // Wait 5 seconds for cache to populate
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                    match channel_deleter::cleanup_empty_channels(&ctx_clone, &data_clone).await {
                        Ok(deleted) => {
                            if deleted > 0 {
                                info!("Deleted {} empty channels on startup", deleted);
                            }
                        }
                        Err(e) => {
                            warn!("Failed to cleanup empty channels on startup: {:?}", e);
                        }
                    }
                });

                // Register commands globally or per-guild based on GUILD_ID env var
                match data.settings.guild_id {
                    Some(guild_id) => {
                        let guild_id = GuildId::new(guild_id);
                        info!("=== REGISTERING AS GUILD-SPECIFIC COMMANDS ===");
                        info!("Guild ID: {}", guild_id);
                        info!("Number of commands: {}", framework.options().commands.len());
                        
                        // First, delete all global commands to avoid conflicts
                        info!("Deleting any existing global commands...");
                        match ctx.http.get_global_commands().await {
                            Ok(global_commands) => {
                                if !global_commands.is_empty() {
                                    info!("Found {} global commands, deleting them...", global_commands.len());
                                    for cmd in &global_commands {
                                        if let Err(e) = ctx.http.delete_global_command(cmd.id).await {
                                            warn!("Failed to delete global command {}: {:?}", cmd.name, e);
                                        } else {
                                            info!("  [OK] Deleted global command: /{}", cmd.name);
                                        }
                                    }
                                } else {
                                    info!("[OK] No global commands to delete");
                                }
                            }
                            Err(e) => {
                                warn!("Could not check for global commands: {:?}", e);
                            }
                        }
                        
                        info!("Using poise::builtins::register_in_guild() - this registers commands ONLY for this specific guild");
                        
                        match poise::builtins::register_in_guild(ctx, &framework.options().commands, guild_id).await {
                            Ok(_) => {
                                info!("[OK] Poise reports successful registration");
                                
                                // Verify by querying Discord's API for GUILD commands
                                match ctx.http.get_guild_commands(guild_id).await {
                                    Ok(registered) => {
                                        info!("[OK] Discord API confirms {} GUILD commands registered for guild {}", registered.len(), guild_id);
                                        for cmd in &registered {
                                            info!("  → Guild command: /{} (id: {})", cmd.name, cmd.id);
                                        }
                                        
                                        // Also check global commands to make sure we didn't register there
                                        match ctx.http.get_global_commands().await {
                                            Ok(global) => {
                                                if !global.is_empty() {
                                                    warn!("[WARN] WARNING: Found {} global commands. Guild commands should be separate.", global.len());
                                                    for cmd in &global {
                                                        info!("  → Global command: /{} (id: {})", cmd.name, cmd.id);
                                                    }
                                                } else {
                                                    info!("[OK] Confirmed: No global commands registered (correct)");
                                                }
                                            }
                                            Err(_) => {
                                                // Ignore - can't check global commands
                                            }
                                        }
                                        
                                        info!("=== IMPORTANT: If commands don't appear in Discord ===");
                                        info!("1. Ensure bot was invited with 'applications.commands' scope");
                                        info!("2. Re-invite URL: https://discord.com/api/oauth2/authorize?client_id={}&permissions=0&scope=bot%20applications.commands", ready.user.id);
                                        info!("3. Fully restart Discord client (not just reload)");
                                        info!("4. Wait 1-2 minutes for Discord cache to update");
                                    }
                                    Err(e) => {
                                        error!("[FAIL] Could not verify guild commands: {:?}", e);
                                        error!("This likely means the bot lacks 'applications.commands' scope!");
                                        error!("Re-invite the bot with: https://discord.com/api/oauth2/authorize?client_id={}&permissions=0&scope=bot%20applications.commands", ready.user.id);
                                    }
                                }
                            }
                            Err(e) => {
                                error!("[FAIL] Failed to register guild commands: {:?}", e);
                                error!("Guild ID used: {}", guild_id);
                                error!("Re-invite URL: https://discord.com/api/oauth2/authorize?client_id={}&permissions=0&scope=bot%20applications.commands", ready.user.id);
                                return Err(Error::Serenity(e));
                            }
                        }
                    }
                    None => {
                        info!("Attempting to register {} commands globally", framework.options().commands.len());
                        match poise::builtins::register_globally(ctx, &framework.options().commands).await {
                            Ok(_) => {
                                info!("Successfully registered {} commands globally", framework.options().commands.len());
                                info!("Note: Global commands can take up to 1 hour to appear in all servers");
                                
                                // Verify commands were registered
                                match ctx.http.get_global_commands().await {
                                    Ok(registered) => {
                                        info!("Verified: Discord reports {} commands registered globally", registered.len());
                                        for cmd in &registered {
                                            info!("  - Registered command: {} (id: {})", cmd.name, cmd.id);
                                        }
                                    }
                                    Err(e) => {
                                        error!("Warning: Could not verify registered commands: {:?}", e);
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Failed to register commands globally: {:?}", e);
                                return Err(Error::Serenity(e));
                            }
                        }
                    }
                }

                Ok(data)
            })
        })
        .build();

    let intents = GatewayIntents::GUILDS
        | GatewayIntents::GUILD_MEMBERS
        | GatewayIntents::GUILD_VOICE_STATES
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    let mut client = serenity::ClientBuilder::new(&settings.discord_token, intents)
        .framework(framework)
        .await
        .map_err(Error::Serenity)?;

    info!("Starting Discord client...");
    client.start().await.map_err(Error::Serenity)
}
