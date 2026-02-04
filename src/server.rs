use actix_web::{App, HttpResponse, HttpServer, get, web};
use anyhow::bail;
use std::{
    net::{Ipv4Addr, Ipv6Addr},
    sync::Arc,
};
use tokio::{
    net::TcpListener,
    sync::{Mutex, oneshot},
};

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, clap::ValueEnum)]
pub enum Bind {
    /// Try IPv6 first, then fall back to IPv4
    #[default]
    Prefer6,
    /// Try IPv4 first, then fall back to IPv6
    Prefer4,
    /// Only try IPv6
    Only6,
    /// Only try IPv4
    Only4,
}

impl Bind {
    pub async fn into_acceptor(self, port: u16) -> anyhow::Result<TcpListener> {
        Ok(match self {
            Self::Prefer6 => match TcpListener::bind((Ipv6Addr::LOCALHOST, port)).await {
                Ok(acceptor) => acceptor,
                Err(err) => {
                    log::info!("Failed to bind to IPv6 localhost, trying IPv4 instead: {err}");
                    match TcpListener::bind((Ipv4Addr::LOCALHOST, port)).await {
                        Ok(acceptor) => acceptor,
                        Err(err) => {
                            log::error!("Failed to bind to either IPv6 or IPv4: {err}");
                            bail!("Unable to bind to IPv4 or IPv6: {err}");
                        }
                    }
                }
            },
            Self::Prefer4 => match TcpListener::bind((Ipv4Addr::LOCALHOST, port)).await {
                Ok(acceptor) => acceptor,
                Err(err) => {
                    log::info!("Failed to bind to IPv4 localhost, trying IPv6 instead: {err}");
                    match TcpListener::bind((Ipv6Addr::LOCALHOST, port)).await {
                        Ok(acceptor) => acceptor,
                        Err(err) => {
                            log::error!("Failed to bind to either IPv6 or IPv4: {err}");
                            bail!("Unable to bind to IPv4 or IPv6: {err}");
                        }
                    }
                }
            },
            Self::Only6 => TcpListener::bind((Ipv6Addr::LOCALHOST, port)).await?,
            Self::Only4 => TcpListener::bind((Ipv4Addr::LOCALHOST, port)).await?,
        })
    }
}

pub struct FlowResult {
    pub code: String,
    pub state: Option<String>,
}

pub struct Server {
    pub port: u16,
    pub rx: oneshot::Receiver<anyhow::Result<FlowResult>>,
}

#[derive(Clone, Debug, serde::Deserialize)]
struct ResponseQuery {
    code: String,
    state: Option<String>,
}

#[derive(Clone)]
struct State {
    pub tx: Arc<Mutex<Option<oneshot::Sender<anyhow::Result<FlowResult>>>>>,
}

#[get("/")]
async fn receive(
    state: web::Data<State>,
    web::Query(query): web::Query<ResponseQuery>,
) -> HttpResponse {
    let Some(tx) = state.tx.lock().await.take() else {
        return HttpResponse::Gone().finish();
    };

    let result = FlowResult {
        code: query.code,
        state: query.state,
    };

    if tx.send(Ok(result)).is_err() {
        log::info!("failed to report token, receiver is already closed");
        return HttpResponse::Gone().finish();
    }

    HttpResponse::Ok().body("You can now close this window")
}

impl Server {
    pub async fn new(bind: Bind, port: Option<u16>) -> anyhow::Result<Self> {
        let (tx, rx) = oneshot::channel();

        let port = port.unwrap_or_default();

        let acceptor = bind.into_acceptor(port).await?;
        let acceptor = acceptor.into_std()?;

        let port = acceptor.local_addr()?.port();

        let tx = Arc::new(Mutex::new(Some(tx)));

        tokio::spawn(async move {
            if let Err(err) = run_http(acceptor, web::Data::new(State { tx: tx.clone() })).await
                && let Some(tx) = tx.lock().await.take()
            {
                // we still have a sender, we respond with our error
                let _ = tx.send(Err(err));
            }
        });

        Ok(Server { port, rx })
    }

    pub async fn receive_token(self) -> anyhow::Result<FlowResult> {
        self.rx.await?
    }
}

async fn run_http(acceptor: std::net::TcpListener, state: web::Data<State>) -> anyhow::Result<()> {
    let http = HttpServer::new(move || App::new().service(receive).app_data(state.clone()))
        .workers(1)
        .listen(acceptor)?;

    http.run().await?;

    Ok(())
}
