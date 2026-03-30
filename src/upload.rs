use s3::{Auth, Client};

use crate::config::S3Config;

pub async fn upload(config: &S3Config, filename: &str, body: Vec<u8>) -> Result<String, s3::Error> {
    let S3Config {
        url,
        bucket,
        region,
        credentials,
    } = config;

    let client = Client::builder(url)
        .unwrap()
        .region(region)
        .auth(Auth::Static(credentials.clone()))
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

// TODO: add upload integration test; this requires running garage instance.
/*
#[tokio::test]
async fn test_upload() {
    assert!(
        upload(
            &S3Config {
                url: "http://localhost:3900".into(),
                bucket: "test".into(),
                region: "garage".into(),
                credentials: s3::Credentials {
                    access_key_id: "GKf50f4b4e49d26d3fa94b1e53".to_string(),
                    secret_access_key:
                        "ea61d254f42cd1a14f75fdc44dee3f9740cb02bedd2a08061f29c52a7f95531c"
                            .to_string(),
                    session_token: None,
                },
            },
            "test",
            vec![0, 1, 2, 3],
        )
        .await
        .is_ok()
    );
}
*/
