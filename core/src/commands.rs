extern crate tokio;
extern crate tokio_core;
use crate::connection_rib::connection_router;
use crate::network::dtls::{dtls_listener, dtls_test_client, dtls_to_peer};
use crate::network::tcp::{tcp_listener, tcp_to_peer};
use futures::future;
use tokio::sync::mpsc::{self};
use tonic::{transport::Server, Request, Response, Status};
use utils::app_config::AppConfig;
use utils::error::Result;

use crate::gdp_proto::globaldataplane_client::GlobaldataplaneClient;
use crate::gdp_proto::globaldataplane_server::{Globaldataplane, GlobaldataplaneServer};
use crate::gdp_proto::{GdpPacket, GdpResponse, GdpUpdate};
use crate::network::grpc::GDPService;

#[cfg(feature = "ros")]
use crate::network::ros::{ros_subscriber, ros_sample, ros_publisher};
#[cfg(feature = "ros")]
use crate::network::ros::ros_subscriber_image;
// const TCP_ADDR: &'static str = "127.0.0.1:9997";
// const DTLS_ADDR: &'static str = "127.0.0.1:9232";
// const GRPC_ADDR: &'static str = "0.0.0.0:50001";

/// inspired by https://stackoverflow.com/questions/71314504/how-do-i-simultaneously-read-messages-from-multiple-tokio-channels-in-a-single-t
/// TODO: later put to another file
#[tokio::main]
async fn router_async_loop() {
    let config = AppConfig::fetch().expect("App config unable to load");
    info!("{:#?}", config);
    let mut future_handles = Vec::new();

    // initialize the address binding
    let all_addr = "0.0.0.0"; //optionally use [::0] for ipv6 address
    let tcp_bind_addr = format!("{}:{}", all_addr, config.tcp_port);
    let dtls_bind_addr = format!("{}:{}", all_addr, config.dtls_port);
    // let grpc_bind_addr = format!("{}:{}", all_addr, config.grpc_port);

    // rib_rx <GDPPacket = [u8]>: forward gdppacket to rib
    let (rib_tx, rib_rx) = mpsc::unbounded_channel();
    // channel_tx <GDPChannel = <gdp_name, sender>>: forward channel maping to rib
    let (channel_tx, channel_rx) = mpsc::unbounded_channel();
    // stat_tx <GdpUpdate proto>: any status update from other routers
    let (stat_tx, stat_rx) = mpsc::unbounded_channel();

    let tcp_sender_handle = tokio::spawn(tcp_listener(
        tcp_bind_addr,
        rib_tx.clone(),
        channel_tx.clone(),
    ));
    future_handles.push(tcp_sender_handle);

    let dtls_sender_handle = tokio::spawn(dtls_listener(
        dtls_bind_addr,
        rib_tx.clone(),
        channel_tx.clone(),
    ));
    future_handles.push(dtls_sender_handle);

    if (config.peer_with_gateway){
        let peer_advertisement = tokio::spawn(tcp_to_peer(
            config.default_gateway.into(),
            rib_tx.clone(),
            channel_tx.clone(),
        ));
        future_handles.push(peer_advertisement);
    }


    // grpc
    // TODO: uncomment for grpc
    // let psl_service = GDPService {
    //     rib_tx: rib_tx.clone(),
    //     status_tx: stat_tx,
    // };

    // let serve = Server::builder()
    //     .add_service(GlobaldataplaneServer::new(psl_service))
    //     .serve(grpc_bind_addr.parse().unwrap());
    // let manager_handle = tokio::spawn(async move {
    //     if let Err(e) = serve.await {
    //         eprintln!("Error = {:?}", e);
    //     }
    // });
    // let grpc_server_handle = manager_handle;
    // future_handles.push(grpc_server_handle);

    let rib_handle = tokio::spawn(connection_router(
        rib_rx,     // receive packets to forward
        stat_rx,    // recevie control place info, e.g. routing
        channel_rx, // receive channel information for connection rib
    ));
    future_handles.push(rib_handle);

    #[cfg(feature = "ros")]
    for ros_config in config.ros {
        let ros_handle = match ros_config.local.as_str() {
            "pub" => {
                match ros_config.topic_type.as_str() {
                    "sensor_msgs/msg/CompressedImage" => {
                        tokio::spawn(
                            ros_subscriber_image(
                                rib_tx.clone(), channel_tx.clone(), 
                                ros_config.node_name, 
                                ros_config.topic_name, 
                                ros_config.topic_type
                            )
                        )
                    }
                    _ => {
                        tokio::spawn(
                            ros_subscriber(
                                rib_tx.clone(), channel_tx.clone(), 
                                ros_config.node_name, 
                                ros_config.topic_name, 
                                ros_config.topic_type
                            )
                        )
                    }
                }
            }
            _ => {tokio::spawn(
                ros_publisher(
                    rib_tx.clone(), channel_tx.clone(), 
                    ros_config.node_name, 
                    ros_config.topic_name, 
                    ros_config.topic_type
                )
            )
            }
        };
        future_handles.push(ros_handle);
    }


    future::join_all(future_handles).await;
}

/// Show the configuration file
pub fn router() -> Result<()> {
    warn!("router is started!");

    // NOTE: uncomment to use pnet
    // libpnet::pnet_proc_loop();
    router_async_loop();

    Ok(())
}

/// Show the configuration file
pub fn config() -> Result<()> {
    let config = AppConfig::fetch()?;
    info!("{:#?}", config);

    Ok(())
}

/// Simulate an error
pub fn simulate_error() -> Result<()> {
    let config = AppConfig::fetch().expect("App config unable to load");
    info!("{:#?}", config);
    // test_cert();
    // get address from default gateway

    #[cfg(feature = "ros")]
    ros_sample();
    // TODO: uncomment them
    let test_router_addr = format!("{}:{}", config.default_gateway, config.dtls_port);
    println!("{}", test_router_addr);
    dtls_test_client("128.32.37.48:9232".into()).expect("DLTS Client error");
    Ok(())
}
