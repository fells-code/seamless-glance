use aws_types::region::Region;
use futures::future::join_all;
use std::future::Future;

pub async fn fetch_all_regions<T, F, Fut>(regions: &[Region], fetcher: F) -> Vec<T>
where
    T: Send + 'static,
    F: Fn(Region) -> Fut + Copy + Send + Sync + 'static,
    Fut: Future<Output = Result<Vec<T>, String>> + Send,
{
    let futures = regions.iter().cloned().map(fetcher);

    let results = join_all(futures).await;

    let mut all = Vec::new();

    for result in results {
        match result {
            Ok(items) => all.extend(items),
            Err(err) => {
                eprintln!("Regional fetch failed: {}", err);
            }
        }
    }

    all
}
