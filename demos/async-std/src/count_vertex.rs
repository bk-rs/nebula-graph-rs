/*
cargo run -p nebula-demo-async-std --bin count_vertex 127.0.0.1 45500 nba 1 player
*/

use std::collections::BTreeMap;
use std::env;
use std::error;
use std::net::Ipv4Addr;

use async_std::net::TcpStream;

use fbthrift_transport::{futures_io::transport::AsyncTransport, AsyncTransportConfiguration};
use nebula_client::{
    MetaClient, MetaTransportResponseHandler, StorageClient, StorageTransportResponseHandler,
};
use nebula_fbthrift_meta::types::ErrorCode as MErrorCode;
use nebula_fbthrift_storage::types::ScanVertexRequest;

#[async_std::main]
async fn main() -> Result<(), Box<dyn error::Error>> {
    run().await
}

async fn run() -> Result<(), Box<dyn error::Error>> {
    let metad_host = env::args()
        .nth(1)
        .unwrap_or_else(|| env::var("METAD_HOST").unwrap_or_else(|_| "127.0.0.1".to_owned()));
    let metad_port: u16 = env::args()
        .nth(2)
        .unwrap_or_else(|| env::var("METAD_PORT").unwrap_or_else(|_| "45500".to_owned()))
        .parse()
        .unwrap();
    let space_name = env::args()
        .nth(3)
        .unwrap_or_else(|| env::var("SPACE_NAME").unwrap_or_else(|_| "nba".to_owned()));
    let partition: u16 = env::args()
        .nth(4)
        .unwrap_or_else(|| env::var("PARTITION").unwrap_or_else(|_| "1".to_owned()))
        .parse()
        .unwrap();
    let tag_name = env::args()
        .nth(5)
        .unwrap_or_else(|| env::var("TAG_NAME").unwrap_or_else(|_| "player".to_owned()));

    println!(
        "count_vertex {} {} {} {} {}",
        metad_host, metad_port, space_name, partition, tag_name
    );

    //
    let metad_addr = format!("{}:{}", metad_host, metad_port);
    let meta_stream = TcpStream::connect(metad_addr).await?;

    let meta_transport = AsyncTransport::new(
        meta_stream,
        AsyncTransportConfiguration::new(MetaTransportResponseHandler),
    );
    let meta_client = MetaClient::new(meta_transport);

    let res = meta_client.get_space(&space_name).await?;
    println!("{:?}", res);
    assert_eq!(res.code, MErrorCode::SUCCEEDED);
    let space_id = res.item.space_id;
    println!("space_id {}", space_id);

    let res = meta_client
        .list_parts(space_id, vec![partition as i32])
        .await?;
    println!("{:?}", res);
    assert_eq!(res.code, MErrorCode::SUCCEEDED);
    let part = res.parts.first().unwrap();
    let part_id = part.part_id;
    println!("part_id {}", part_id);
    let part_leader_addr = part.to_owned().leader.unwrap();
    let part_storage_addr = format!(
        "{}:{}",
        Ipv4Addr::from(part_leader_addr.ip as u32),
        part_leader_addr.port
    );
    println!("part_storage_addr {}", part_storage_addr);

    let res = meta_client.list_tags(space_id).await?;
    println!("{:?}", res);
    assert_eq!(res.code, MErrorCode::SUCCEEDED);
    let tag = res.tags.iter().find(|x| x.tag_name == tag_name).unwrap();
    let tag_id = tag.tag_id;
    println!("tag_id {}", tag_id);

    //
    let storage_stream = TcpStream::connect(part_storage_addr).await?;

    let mut storage_transport_configuration =
        AsyncTransportConfiguration::new(StorageTransportResponseHandler);
    storage_transport_configuration.set_max_parse_response_bytes_count(10);
    storage_transport_configuration.set_max_buf_size(1024 * 1024 * 32);
    storage_transport_configuration.set_buf_size(1024 * 1024 * 4);

    let storage_transport = AsyncTransport::new(storage_stream, storage_transport_configuration);
    let storage_client = StorageClient::new(storage_transport);

    let mut total_count = 0;
    let mut next_cursor = None;

    loop {
        let mut return_columns = BTreeMap::new();
        return_columns.insert(tag_id, vec![]);

        let req = ScanVertexRequest {
            space_id,
            part_id,
            cursor: next_cursor,
            return_columns,
            all_columns: false,
            limit: 100000,
            start_time: 1598918400,
            end_time: 1604188800,
        };

        let res = storage_client.scan_vertex(&req).await?;

        assert_eq!(res.result.failed_codes.len(), 0);

        total_count += res.vertex_data.len();
        next_cursor = Some(res.next_cursor);
        println!(
            "latency_in_us {}, tag_id {}, len {}, total_count {}",
            res.result.latency_in_us,
            tag_id,
            res.vertex_data.len(),
            total_count,
        );

        if !res.has_next {
            break;
        }
    }

    println!("total_count: {}", total_count);

    println!("done");

    Ok(())
}
