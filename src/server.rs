use actix_web::{get, web, App, HttpResponse, HttpServer};
use std::net::{Ipv6Addr, SocketAddr};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::{oneshot, Mutex};

pub struct FlowResult {
    pub code: String,
}

pub struct Server {
    pub port: u16,
    pub rx: oneshot::Receiver<anyhow::Result<FlowResult>>,
}

#[derive(Clone, Debug, serde::Deserialize)]
struct ResponseQuery {
    code: String,
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

    let result = FlowResult { code: query.code };

    if tx.send(Ok(result)).is_err() {
        log::info!("failed to report token, receiver is already closed");
        return HttpResponse::Gone().finish();
    }

    HttpResponse::Ok().body("You can now close this window")
}

impl Server {
    pub async fn new(port: Option<u16>) -> anyhow::Result<Self> {
        let (tx, rx) = oneshot::channel();

        let port = port.unwrap_or_default();
        let addr = SocketAddr::new(Ipv6Addr::LOCALHOST.into(), port);

        let acceptor = TcpListener::bind(addr).await?;
        let acceptor = acceptor.into_std()?;

        let port = acceptor.local_addr()?.port();

        let tx = Arc::new(Mutex::new(Some(tx)));

        tokio::spawn(async move {
            if let Err(err) = run_http(acceptor, web::Data::new(State { tx: tx.clone() })).await {
                if let Some(tx) = tx.lock().await.take() {
                    // we stil have a sender, we respond with our error
                    let _ = tx.send(Err(err));
                }
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
