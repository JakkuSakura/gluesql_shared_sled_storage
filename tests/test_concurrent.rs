use eyre::Result;
use gluesql_core::prelude::{Glue, Payload};
use gluesql_shared_sled_storage::SharedSledStorage;
use sled::Config;

async fn get_length(table: &mut Glue<SharedSledStorage>) -> Result<usize> {
    let payloads = table.execute("SELECT * FROM t;").await?;
    match payloads.into_iter().next().unwrap() {
        Payload::Select { labels: _, rows } => Ok(rows.len()),
        _ => unreachable!(),
    }
}
#[tokio::test]
async fn test_concurrent_access() -> Result<()> {
    let db = SharedSledStorage::new(Config::default(), false);
    let mut table = Glue::new(db.clone());
    let _ = table.execute("CREATE TABLE t (a INT);").await;
    let len = get_length(&mut table).await?;
    println!("Before Length: {}", len);
    table.execute("DELETE FROM t;").await?;
    let len = get_length(&mut table).await?;
    println!("After Length: {}", len);
    assert_eq!(len, 0);

    let localset = tokio::task::LocalSet::new();
    localset
        .run_until(async {
            {
                tokio::task::spawn_local(async move {
                    let mut table = Glue::new(db.clone());
                    for i in 0..100 {
                        println!("Inserting {}", i);
                        table
                            .execute(format!("INSERT INTO t (a) VALUES ({});", i).as_str())
                            .await
                            .unwrap();
                        tokio::task::yield_now().await;
                    }
                });
            }

            loop {
                let payloads = table.execute("SELECT * FROM t;").await?;
                match payloads.into_iter().next().unwrap() {
                    Payload::Select { labels: _, rows } => {
                        println!("Rows: {}", rows.len());
                        if rows.len() == 100 {
                            break;
                        }
                    }
                    _ => unreachable!(),
                }
                tokio::task::yield_now().await;
            }
            Ok(())
        })
        .await
}
