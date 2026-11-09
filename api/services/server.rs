//! HTTP/1 server policy: bounded header reads, streaming-safe bodies and graceful drain.
use super::live::LiveUpdates;
use axum::Router;
use hyper_util::{
    rt::{TokioIo, TokioTimer},
    service::TowerToHyperService,
};
use std::{future::Future, time::Duration};
use tokio::{net::TcpListener, sync::watch, task::JoinSet};
pub async fn serve(
    listener: TcpListener,
    router: Router,
    live: LiveUpdates,
    shutdown: impl Future<Output = ()>,
) -> std::io::Result<()> {
    tokio::pin!(shutdown);
    let (stop, _) = watch::channel(false);
    let mut connections = JoinSet::new();
    loop {
        tokio::select! {
            biased;
            _=&mut shutdown=>break,
            Some(result)=connections.join_next(),if !connections.is_empty()=>{if let Err(error)=result{eprintln!("connection task: {error}");}},
            incoming=listener.accept()=>{
                let (stream,_)=incoming?;let router=router.clone();let mut stop=stop.subscribe();
                connections.spawn(async move{
                    let mut builder=hyper::server::conn::http1::Builder::new();
                    builder.timer(TokioTimer::new()).header_read_timeout(Duration::from_secs(10));
                    let connection=builder.serve_connection(TokioIo::new(stream),TowerToHyperService::new(router));
                    tokio::pin!(connection);
                    let result=tokio::select!{
                        result=&mut connection=>result,
                        _=stop.changed()=>{connection.as_mut().graceful_shutdown();connection.await},
                    };
                    if let Err(error)=result{eprintln!("HTTP connection: {error}");}
                });
            }
        }
    }
    live.shutdown();
    stop.send_replace(true);
    if tokio::time::timeout(Duration::from_secs(5), async {
        while connections.join_next().await.is_some() {}
    })
    .await
    .is_err()
    {
        eprintln!("graceful shutdown exceeded 5 seconds");
        connections.abort_all();
    }
    Ok(())
}
