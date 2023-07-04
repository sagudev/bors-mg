//! This is the library of the bors bot.
pub mod bors;
pub mod config;
pub mod github;
pub mod models;
pub mod permissions;
pub mod utils;

use bors::handle_bors_event;
use config::{APP_ID, CMD_PREFIX, PAT, PRIVATE_KEY, WEBHOOK_SECRET};
pub use console_error_panic_hook::set_once as set_panic_hook;
use github::webhook::GitHubWebhook;
use tracing_subscriber::fmt::format::Pretty;
use tracing_subscriber::fmt::time::UtcTime;
use tracing_subscriber::prelude::__tracing_subscriber_SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_web::{performance_layer, MakeConsoleWriter};
use worker::*;

mod cf;

//.route("/github", post(github_webhook_handler))

#[event(fetch, respond_with_errors)]
pub async fn main(req: Request, env: Env, _ctx: worker::Context) -> Result<Response> {
    set_panic_hook();
    // tracer
    let fmt_layer = tracing_subscriber::fmt::layer()
        .json()
        .with_ansi(false) // Only partially supported across JavaScript runtimes
        .with_timer(UtcTime::rfc_3339()) // std::time is not available in browsers
        .with_writer(MakeConsoleWriter) // write events to the console
        .with_level(true)
        .with_target(false);
    let perf_layer = performance_layer().with_details_from_fields(Pretty::default());
    tracing_subscriber::registry()
        .with(fmt_layer)
        .with(perf_layer)
        .init();

    // make sure of env
    if let Ok(pat) = env.secret("PAT") {
        PAT.set(pat.to_string()).unwrap();
    }
    WEBHOOK_SECRET
        .set(
            env.secret("WEBHOOK_SECRET")
                .expect("No WEBHOOK_SECRET secret")
                .to_string(),
        )
        .unwrap();
    CMD_PREFIX
        .set(
            env.var("CMD_PREFIX")
                .map(|x| x.to_string())
                .unwrap_or("@bors-servo".to_string()),
        )
        .unwrap();
    /*
    there are actually two modes we could run
    - pat only, where we need to manually register hook(s)
    - app, where pat is just assistant bot acc
    - PAT only
    */
    // so these are optional
    if let Ok(app_id) = env.secret("APP_ID") {
        APP_ID.set(app_id.to_string()).unwrap();
    }
    if let Ok(private_key) = env.secret("PRIVATE_KEY") {
        PRIVATE_KEY.set(private_key.to_string()).unwrap();
    }

    // Create an instance of the Router, which can use parameters (/user/:name) or wildcard values
    // (/file/*pathname). Alternatively, use `Router::with_data(D)` and pass in arbitrary data for
    // routes to access and share using the `ctx.data()` method.
    let router = Router::new();

    router
        // listener on app webhooks
        .post_async("/app", |_req, _ctx| async move {
            Response::error("Bad Request", 400)
        })
        // listener on manual webhooks
        /*.post_async("/hook", |mut req, _ctx| async move {
            let span = tracing::info_span!("Hook");
            // parse
            let event = GitHubWebhook::from_request(&mut req).await.map_err();
            // ex
            handle_bors_event(event.0).await.unwrap();
            /*
            let form = req.json().form_data().await?;
            if let Some(entry) = form.get("file") {
                match entry {
                    FormEntry::File(file) => {
                        let bytes = file.bytes().await?;
                    }
                    FormEntry::Field(_) => return Response::error("Bad Request", 400),
                }
                // ...
                if let Some(permissions) = form.get("permissions") {
                    // permissions == "a,b,c,d"
                }
                // or call `form.get_all("permissions")` if using multiple entries per field
            }
            Response::error("Bad Request", 400)*/
            Response::ok("OK")
        })*/
        .run(req, env)
        .await
}
