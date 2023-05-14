use hyper::{Body, Request, Response, Server};
use hyper::service::{make_service_fn, service_fn};
use metrics::{counter, gauge, histogram};
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;

async fn serve_req(_req: Request<Body>, handle: Arc<PrometheusHandle>) -> Result<Response<Body>, Infallible> {
    let metrics = handle.render();
    Ok(Response::new(Body::from(metrics)))
}

async fn run_server(addr: SocketAddr, handle: Arc<PrometheusHandle>) {
    let make_svc = make_service_fn(move |_conn| {
        let handle = handle.clone();
        async move {
            Ok::<_, Infallible>(service_fn(move |req| serve_req(req, handle.clone())))
        }
    });

    let server = Server::bind(&addr).serve(make_svc);

    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }
}

#[tokio::main]
async fn main() {
    let (recorder, exporter) = PrometheusBuilder::new().build().expect("Failed to create a recorder");
    let handle = Arc::new(recorder.handle());

    metrics::set_boxed_recorder(Box::new(recorder)).expect("Failed to set recorder");

    counter!("my_counter", 1);
    gauge!("my_gauge", 42.0);
    histogram!("my_histogram", 42.0);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    
    // Spawn the metrics exporter
    tokio::spawn(async move {
        tokio::pin!(exporter);

        if let Err(err) = exporter.await {
            eprintln!("Error running the exporter: {}", err);
        }
    });

    // Run the server
    run_server(addr, handle).await;
}
