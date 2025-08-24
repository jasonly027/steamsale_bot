use poise::CreateReply;

use crate::{Error, framework::Context};

#[poise::command(slash_command)]
pub async fn test(ctx: Context<'_>) -> Result<(), Error> {
    ctx.send(CreateReply::default().content("Hello World"))
        .await?;
    Ok(())
}
