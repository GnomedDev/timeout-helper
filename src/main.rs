use poise::serenity_prelude as serenity;

use anyhow::Result;

type Context<'a> = poise::Context<'a, (), anyhow::Error>;

fn ephemeral(content: &'static str) -> poise::CreateReply {
    poise::CreateReply::default()
        .content(content)
        .ephemeral(true)
}

async fn parse_duration(ctx: &Context<'_>, duration: &str) -> Result<Option<serenity::Timestamp>> {
    let now = std::time::SystemTime::now();
    let expire_time = if let Some(duration) = duration.strip_suffix('m') {
        let minutes: u64 = duration.parse()?;
        if minutes > 60 {
            ctx.send(ephemeral("1h max for timeouts rn")).await?;
            return Ok(None);
        };

        now + std::time::Duration::from_secs(minutes * 60)
    } else if duration == "1h" {
        now + std::time::Duration::from_secs(60 * 60)
    } else {
        ctx.send(ephemeral("Must be in format {}m or 1h")).await?;
        return Ok(None);
    };

    let expire_time_secs = expire_time
        .duration_since(std::time::SystemTime::UNIX_EPOCH)?
        .as_secs();

    serenity::Timestamp::from_unix_timestamp(expire_time_secs as i64)
        .map(Some)
        .map_err(Into::into)
}

#[poise::command(slash_command, guild_only)]
async fn timeout(
    ctx: Context<'_>,
    target: serenity::UserId,
    duration: String,
    #[rest] reason: String,
) -> Result<()> {
    let Some(expire_time) = parse_duration(&ctx, &duration).await? else {
        return Ok(());
    };

    let guild_id = ctx.guild_id().unwrap();
    let mut member = ctx.http().get_member(guild_id, target).await?;

    let builder = serenity::EditMember::new()
        .disable_communication_until_datetime(expire_time)
        .audit_log_reason(&reason);

    member.edit(ctx, builder).await?;

    let resp = format!("Timed {} out for `{duration}`.", member.display_name());
    ctx.say(resp).await?;

    Ok(())
}

#[poise::command(prefix_command, owners_only)]
async fn register(ctx: Context<'_>) -> Result<()> {
    poise::builtins::register_application_commands(ctx, false).await?;
    Ok(())
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let token = std::env::var("TOKEN").unwrap();

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![timeout(), register()],
            prefix_options: poise::PrefixFrameworkOptions {
                mention_as_prefix: true,
                ..Default::default()
            },
            ..Default::default()
        })
        .setup(|_, _, _| Box::pin(async move { Ok(()) }))
        .build();

    let mut client = serenity::Client::builder(&token, serenity::GatewayIntents::empty())
        .framework(framework)
        .await?;

    client.start().await?;

    Ok(())
}
