/*
cargo run -p nebula-graph-demo-tokio --bin v2_bb8_graph_pool 127.0.0.1 3699 user 'password'
*/

use std::env;
use std::io;

use bb8_nebula_graph::v2::{NebulaGraphClientConfiguration, NebulaGraphConnectionManager};

#[tokio::main]
async fn main() -> io::Result<()> {
    run().await
}

async fn run() -> io::Result<()> {
    let domain = env::args()
        .nth(1)
        .unwrap_or_else(|| env::var("DOMAIN").unwrap_or("127.0.0.1".to_owned()));
    let port: u16 = env::args()
        .nth(2)
        .unwrap_or_else(|| env::var("PORT").unwrap_or("3699".to_owned()))
        .parse()
        .unwrap();
    let username = env::args()
        .nth(3)
        .unwrap_or_else(|| env::var("USERNAME").unwrap_or("user".to_owned()));
    let password = env::args()
        .nth(4)
        .unwrap_or_else(|| env::var("PASSWORD").unwrap_or("password".to_owned()));
    let space = env::args().nth(5).or_else(|| env::var("SPACE").ok());

    println!(
        "v2_bb8_graph_pool {} {} {} {} {:?}",
        domain, port, username, password, space
    );

    //
    let configuration =
        NebulaGraphClientConfiguration::new(domain, port, username, password, space);
    let manager = NebulaGraphConnectionManager::new(configuration);
    let pool = bb8::Pool::builder()
        .max_size(10)
        .test_on_check_out(false)
        .build(manager)
        .await?;

    //
    let session = pool.get().await.unwrap();

    let res = session
        .execute(&"SHOW SPACES;".as_bytes().to_vec())
        .await
        .unwrap();
    println!("{:?}", res);

    let res = session
        .execute(&"SHOW HOSTS;".as_bytes().to_vec())
        .await
        .unwrap();
    println!("{:?}", res);

    println!("done");

    Ok(())
}