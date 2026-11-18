mod slogger;

use tracing::level_filters::LevelFilter;
use tracing_subscriber::fmt::time::UtcTime;
use tracing_subscriber::prelude::*;
use worker::*;

use tracing_web::MakeConsoleWriter;

use worker::postgres_tls::PassthroughTls;

fn main() {
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_ansi(false) // Only partially supported across JavaScript runtimes
        .with_timer(UtcTime::rfc_3339()) // std::time is not available in browsers
        .with_writer(MakeConsoleWriter)
        .with_filter(LevelFilter::TRACE);
    tracing_subscriber::registry().with(fmt_layer).init();
}

#[event(fetch)]
async fn main(_req: Request, env: Env, _ctx: Context) -> anyhow::Result<Response> {
    let hyperdrive = env.hyperdrive("HYPERDRIVE")?;

    // Connect using Worker Socket
    let socket = Socket::builder()
        .secure_transport(SecureTransport::StartTls)
        .connect(hyperdrive.host(), hyperdrive.port())?;

    let socket = slogger::LoggingSocket::new(socket);

    let config = hyperdrive
        .connection_string()
        .parse::<tokio_postgres::Config>()?;

    let (mut client, connection) = config.connect_raw(socket, PassthroughTls).await?;

    wasm_bindgen_futures::spawn_local(async move {
        if let Err(error) = connection.await {
            console_log!("connection error: {:?}", error);
        }
    });

    let tx = client.transaction().await?;
    let statement = tx.prepare("SELECT generate_series(1, 10)").await?;

    let portal = tx.bind(&statement, &[]).await?;
    let rows = tx.query_portal(&portal, 10).await?;

    let out = rows.into_iter().map(|r| r.get::<_, i32>(0)).collect::<Vec<_>>();

    // Enqueue portal close
    // https://github.com/rust-postgres/rust-postgres/blob/f1cb6ec0d5766b136cbd68f3010d64142a5daa66/tokio-postgres/src/portal.rs#L14-L25
    drop(portal);

    // Give the connection a chance to process the close; Drop just queues it
    tx.query_one("SELECT 1", &[]).await?;
    tx.commit().await?;

    console_log!("{:?}", out);

    Ok(Response::ok("ok")?)
}
