use s3::{Auth, Client, Credentials};

use crate::config::S3Config;

pub async fn upload(config: &S3Config, filename: &str, body: Vec<u8>) -> Result<String, s3::Error> {
    let S3Config {
        url,
        bucket,
        region,
        key,
        secret,
    } = config;

    let client = Client::builder(url)
        .unwrap()
        .region(region)
        .auth(Auth::Static(Credentials::new(key, secret).unwrap()))
        // TODO: build credentials (and check?) when loading config so panic happens on startup
        .build()
        .unwrap();

    client
        .objects()
        .put(bucket, filename)
        .content_type("application/pdf")
        .body_bytes(body)
        .send()
        .await
        .map(|_| format!("{url}/{filename}"))
}

/*
// TODO: add back upload test.
#[tokio::test]
async fn test_upload() {
    let _ = upload(
        &S3Config {
            url: "http://localhost:3900".into(),
            bucket: "test".into(),
            region: "garage".into(),
            key: "GKf50f4b4e49d26d3fa94b1e53".into(),
            secret: "ea61d254f42cd1a14f75fdc44dee3f9740cb02bedd2a08061f29c52a7f95531c".into(),
        },
        "test",
        vec![0, 1, 2, 3],
    )
    .await;
}
*/
