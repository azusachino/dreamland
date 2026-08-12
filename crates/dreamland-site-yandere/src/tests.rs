use super::*;
use dreamland_core::{FeedKind, Rating, SiteErrorCode, TagCategory};

const IMAGE_JSON: &str = r#"{
    "id": 123456,
    "tags": "test tag1 tag2",
    "author": "board_user",
    "creator_id": 452674,
    "source": "https://www.pixiv.net/artworks/148271180",
    "parent_id": 123455,
    "has_children": true,
    "created_at": "2026-08-11T01:02:03.000Z",
    "width": 1920,
    "height": 1080,
    "file_url": "https://example.com/image.jpg",
    "sample_url": "https://example.com/sample.jpg",
    "preview_url": "https://example.com/preview.jpg",
    "rating": "s",
    "score": 42,
    "md5": "abcdef123456",
    "file_size": 1048576
}"#;

#[test]
fn site_uses_the_stable_yandere_id() {
    assert_eq!(SITE_ID, "yandere");
}

#[test]
fn maps_wire_shape_to_neutral_post() {
    let posts = decode_posts(format!("[{IMAGE_JSON}]").as_bytes()).unwrap();
    let post = &posts[0];

    assert_eq!(post.post.site.as_str(), SITE_ID);
    assert_eq!(post.post.id, "123456");
    assert_eq!(post.tags, vec!["test", "tag1", "tag2"]);
    assert_eq!(post.author.as_deref(), Some("board_user"));
    assert_eq!(post.creator_id, Some(452674));
    assert_eq!(
        post.source.as_deref(),
        Some("https://www.pixiv.net/artworks/148271180")
    );
    assert_eq!(post.parent_id.as_deref(), Some("123455"));
    assert!(post.has_children);
    assert_eq!(post.created_at.as_deref(), Some("2026-08-11T01:02:03.000Z"));
    assert_eq!(post.rating, Rating::Safe);
    assert_eq!(
        variant_url(post, MediaVariant::Full),
        post.full_url.as_deref()
    );
}

#[test]
fn decodes_legacy_array_and_version_two_envelope() {
    let legacy = decode_posts(format!("[{IMAGE_JSON}]").as_bytes()).unwrap();
    let version_two = decode_posts(format!(r#"{{"posts":[{IMAGE_JSON}]}}"#).as_bytes()).unwrap();

    assert_eq!(legacy, version_two);
    assert_eq!(legacy.len(), 1);
}

#[test]
fn missing_metadata_is_not_fabricated() {
    let posts = decode_posts(br#"[{"id":7,"tags":"artist -tag","rating":"x"}]"#).unwrap();

    assert_eq!(posts[0].width, None);
    assert_eq!(posts[0].full_url, None);
    assert_eq!(posts[0].rating, Rating::Unknown);
    assert_eq!(posts[0].tags, vec!["artist", "-tag"]);
}

#[test]
fn numeric_created_at_is_normalized_for_moebooru_variants() {
    let posts = decode_posts(br#"[{"id":7,"created_at":1786390664}]"#).unwrap();

    assert_eq!(posts[0].created_at.as_deref(), Some("2026-08-10T19:37:44Z"));
}

#[test]
fn malformed_post_payload_is_a_decode_error() {
    assert!(decode_posts(br#"{"unexpected":true}"#).is_err());
}

#[test]
fn tag_query_preserves_negative_terms_and_adds_safe_filter() {
    assert_eq!(
        tag_expression("artist_name -sketch", ContentPolicy::SafeOnly).unwrap(),
        "artist_name -sketch rating:s"
    );
    assert_eq!(
        tag_query_params(
            "artist_name -sketch",
            ContentPolicy::AllowQuestionable,
            2,
            40
        )
        .unwrap(),
        vec![
            ("tags", "artist_name -sketch -rating:e".to_owned()),
            ("page", "2".to_owned()),
            ("limit", "40".to_owned()),
        ]
    );
}

#[test]
fn tag_query_rejects_empty_or_invalid_page() {
    assert!(tag_expression(" ", ContentPolicy::SafeOnly).is_err());
    assert!(tag_query_params("tag", ContentPolicy::SafeOnly, 0, 40).is_err());
    assert!(tag_query_params("tag", ContentPolicy::SafeOnly, 1, 0).is_err());
}

#[test]
fn popular_modes_map_to_paged_tag_expressions() {
    assert_eq!(
        popular_tag_expression(PopularPeriod::Day, "2026-08-10").unwrap(),
        "date:2026-08-10 order:score"
    );
    assert_eq!(
        popular_tag_expression(PopularPeriod::Week, "2026-08-10").unwrap(),
        "date:2026-08-10..2026-08-16 order:score"
    );
    assert_eq!(
        popular_tag_expression(PopularPeriod::Month, "2026-08-10").unwrap(),
        "date:2026-08-01..2026-08-31 order:score"
    );
    assert!(popular_tag_expression(PopularPeriod::Day, "2026-02-29").is_err());
}

#[test]
fn tag_suggestions_map_category_metadata() {
    let suggestions = decode_tag_suggestions(
        br#"[{"name":"artist_name","count":12,"type":1,"ambiguous":false}]"#,
    )
    .unwrap();

    assert_eq!(suggestions[0].name, "artist_name");
    assert_eq!(suggestions[0].category, Some(TagCategory::Artist));
    assert_eq!(suggestions[0].post_count, Some(12));
    assert_eq!(suggestions[0].ambiguous, Some(false));
    assert!(suggestions[0].aliases.is_empty());
}

#[test]
fn pools_map_to_site_neutral_records() {
    let pools =
        decode_pools(br#"[{"id":12,"name":"art book","is_public":true,"post_count":4}]"#).unwrap();

    assert_eq!(pools[0].site.as_str(), SITE_ID);
    assert_eq!(pools[0].id, "12");
    assert_eq!(pools[0].name, "art book");
    assert_eq!(pools[0].post_count, 4);
    assert!(pools[0].public);
}

#[test]
fn pool_query_is_a_safe_exact_tag_expression() {
    assert_eq!(pool_posts_expression("42").unwrap(), "pool:42");
    assert!(pool_posts_expression("42 order:score").is_err());
}

#[test]
fn pool_zip_endpoint_is_site_owned_and_validated() {
    assert_eq!(
        pool_zip_endpoint("https://yande.re/post.json", "42").unwrap(),
        "https://yande.re/pool/zip/42"
    );
    assert!(pool_zip_endpoint("https://yande.re/post.json", "42/x").is_err());
}

#[test]
fn query_mapping_keeps_search_and_popular_pagination_distinct() {
    let search = PostQueryRequest {
        query: dreamland_core::ReplayableQuery {
            source: DiscoverySource::Search {
                expression: "artist_name -sketch".to_owned(),
            },
            content_policy: ContentPolicy::SafeOnly,
        },
        pagination: PaginationRequest::Page {
            number: 2,
            page_size: 40,
        },
    };
    let (_, search_params, _, _) = request_parts("https://yande.re/post.json", &search).unwrap();
    assert_eq!(
        search_params,
        vec![
            ("tags", "artist_name -sketch rating:s".to_owned()),
            ("page", "2".to_owned()),
            ("limit", "40".to_owned()),
        ]
    );

    let popular = PostQueryRequest {
        query: dreamland_core::ReplayableQuery {
            source: DiscoverySource::Feed {
                kind: FeedKind::Popular {
                    period: PopularPeriod::Month,
                    anchor_date: "2026-08-10".to_owned(),
                },
            },
            content_policy: ContentPolicy::SafeOnly,
        },
        pagination: PaginationRequest::Page {
            number: 2,
            page_size: 40,
        },
    };
    let (endpoint, params, _, fixed_window) =
        request_parts("https://yande.re/post.json", &popular).unwrap();
    assert_eq!(endpoint, "https://yande.re/post.json");
    assert_eq!(
        params,
        vec![
            (
                "tags",
                "date:2026-08-01..2026-08-31 order:score rating:s".to_owned()
            ),
            ("page", "2".to_owned()),
            ("limit", "40".to_owned()),
        ]
    );
    assert!(!fixed_window);
    assert!(request_parts(
        "https://yande.re/post.json",
        &PostQueryRequest {
            pagination: PaginationRequest::FixedWindow,
            ..popular
        }
    )
    .is_err());
}

#[test]
fn http_statuses_map_to_stable_site_errors() {
    assert_eq!(
        map_http_status(reqwest::StatusCode::TOO_MANY_REQUESTS).code,
        SiteErrorCode::RateLimited
    );
    assert!(!map_http_status(reqwest::StatusCode::UNAUTHORIZED).retryable);
    assert!(map_http_status(reqwest::StatusCode::SERVICE_UNAVAILABLE).retryable);
}

#[tokio::test]
async fn yande_client_identifies_dreamland() {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = Vec::new();
        let mut buffer = [0_u8; 4096];
        loop {
            let count = stream.read(&mut buffer).unwrap();
            request.extend_from_slice(&buffer[..count]);
            if request.windows(4).any(|window| window == b"\r\n\r\n") {
                break;
            }
        }
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\n[]")
            .unwrap();
        request
    });
    let response = build_client(&NetworkPolicy {
        proxy: ProxyMode::Direct,
        ..NetworkPolicy::default()
    })
    .unwrap()
    .get(format!("http://{address}/post.json"))
    .send()
    .await
    .unwrap();
    assert!(response.status().is_success());
    let request = server.join().unwrap();
    assert!(request
        .windows(b"user-agent: dreamland/".len())
        .any(|window| { window.eq_ignore_ascii_case(b"user-agent: dreamland/") }));
}

#[test]
fn default_config_is_loaded_from_the_bundled_toml_not_hardcoded() {
    assert_eq!(default_config().api_url, "https://yande.re/post.json");
}

#[test]
fn authentication_adapter_owns_cookie_mapping() {
    use dreamland_core::{AuthenticationCapability, BrowserCookie};
    let session = Adapter::default()
        .session_from_cookies(&[BrowserCookie {
            name: "user_info".to_owned(),
            value: "12345;20;1".to_owned(),
        }])
        .unwrap()
        .unwrap();
    assert_eq!(session.user_id(), "12345");
    assert_eq!(session.site().as_str(), SITE_ID);
    assert!(Adapter::default()
        .session_from_cookies(&[])
        .unwrap()
        .is_none());
}

#[test]
fn browser_post_url_is_site_owned_and_validated() {
    let config = default_config();
    assert_eq!(
        browser_post_url(&config.browser_url, "407162").unwrap(),
        "https://yande.re/post/show/407162"
    );
    assert!(browser_post_url(&config.browser_url, "not-a-number").is_err());
}
