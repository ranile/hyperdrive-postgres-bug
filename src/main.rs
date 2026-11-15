mod slogger;

use std::str::FromStr;

use sqlx::{PgConnection, Row, postgres::PgConnectOptions};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::fmt::time::UtcTime;
use tracing_subscriber::prelude::*;
use worker::*;

use slogger::LoggingSocket;
use tracing_web::MakeConsoleWriter;

fn main() {
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_ansi(false) // Only partially supported across JavaScript runtimes
        .with_timer(UtcTime::rfc_3339()) // std::time is not available in browsers
        .with_writer(MakeConsoleWriter)
        .with_filter(LevelFilter::TRACE);
    tracing_subscriber::registry().with(fmt_layer).init();
}

#[event(fetch)]
async fn fetch(_req: Request, env: Env, _ctx: Context) -> Result<Response> {
    let hyperdrive = env.hyperdrive("HYPERDRIVE")?;
    let connection_string = hyperdrive.connection_string();
    let opts = PgConnectOptions::from_str(&connection_string).map_err(|e| {
        worker::Error::RustError(format!("invalid Hyperdrive connection string: {e}"))
    })?;

    let socket = Socket::builder()
        .secure_transport(SecureTransport::StartTls)
        .connect(opts.get_host(), opts.get_port())?;
    let socket = LoggingSocket::new(socket);
    // This has to be given a Socket because DNS resolution for *.worker.alt addresses is broken on
    // workerd: https://github.com/cloudflare/workerd/issues/8391
    let mut conn = PgConnection::connect_raw_tokio(socket, &opts)
        .await
        .map_err(|e| Error::RustError(format!("failed to connect to database: {e}")))?;

    let row = sqlx::query("SELECT uuidv7()::text")
        .fetch_one(&mut conn)
        .await
        .map_err(|e| Error::RustError(format!("select uuidv7() failed: {e}")))?;
    let uuid: String = row.get(0);
    Response::ok(uuid)
}
