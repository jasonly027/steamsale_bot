use poise::serenity_prelude as serenity;

use crate::{Result, framework::Context};

/// Shows the commands list and FAQ.
#[poise::command(slash_command, user_cooldown = 3)]
pub async fn help(ctx: Context<'_>) -> Result<()> {
    ctx.send(
        poise::CreateReply::default().embed(
            serenity::CreateEmbed::new()
                .title("Commands and FAQ")
                .field("/bind <text_channel>",
                    "Set the channel where alerts are sent. Sends to the server default channel by default.",
                    false
                )
                .field("/set_discount_threshold <threshold> <appid1, appid2, ...>",
                    "Set the minimum discount percentage warranting an alert of an app sale. By default, the threshold is 1%. App IDs can be referenced that this threshold specifically applies to.",
                    false)
                .field("/add_apps <appid1, appid2, ...> <threshold>",
                    "Add apps to the tracker. A discount threshold can be stated that applies specifically to these apps.",
                    false)
                .field("/remove_apps <appid1, appid2, ...>",
                    "Remove apps from the tracker.",
                    false)
                .field("/search <query>",
                    "Search for an app to add to the tracker.",
                    false)
                .field("/list_apps", 
                    "List apps being tracked and their discount thresholds.",
                    false)
                .field("/clear_apps",
                    "Remove all apps from the tracker.",
                    false)
                .field("How often does the bot check for sales?",
                    "The bot begins checking at 10:00AM PDT daily.",
                    true)
                .field("Why aren't alerts showing up?", 
                    "Please try reconfiguring discount thresholds or the bound text channel.",
                    true)
        ),
    )
    .await?;

    Ok(())
}
