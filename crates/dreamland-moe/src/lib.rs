use anyhow::{bail, Context, Result};
use dreamland_core::{
    ContentPolicy, Continuation, DiscoverySource, FeedKind, NetworkPolicy, PaginationRequest,
    PopularPeriod, Post, PostQueryRequest, PostRef, ProxyMode, Rating, SiteError, SiteErrorCode,
    SiteId, SitePage, TagCategory, TagSuggestion, TagSuggestionRequest,
};
use serde::Deserialize;

const USER_AGENT: &str = concat!("Dreamland/", env!("CARGO_PKG_VERSION"));

#[derive(Debug, Clone, Deserialize, PartialEq)]
struct ImagePost {
    id: u64,
    #[serde(default)]
    tags: String,
    author: Option<String>,
    creator_id: Option<u64>,
    md5: Option<String>,
    source: Option<String>,
    parent_id: Option<u64>,
    #[serde(default)]
    has_children: bool,
    created_at: Option<WireTimestamp>,
    width: Option<u32>,
    height: Option<u32>,
    file_url: Option<String>,
    sample_url: Option<String>,
    preview_url: Option<String>,
    rating: Option<String>,
    score: Option<i32>,
    file_size: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(untagged)]
enum WireTimestamp {
    Text(String),
    UnixSeconds(i64),
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(untagged)]
enum PostResponse {
    Legacy(Vec<ImagePost>),
    VersionTwo { posts: Vec<ImagePost> },
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
struct TagRecord {
    name: String,
    count: Option<u64>,
    #[serde(rename = "type")]
    category: Option<u8>,
    ambiguous: Option<bool>,
}

pub fn decode_posts(body: &[u8], site_id: &str) -> Result<Vec<Post>> {
    let response =
        serde_json::from_slice::<PostResponse>(body).context("decode Moebooru post response")?;
    let images = match response {
        PostResponse::Legacy(images) | PostResponse::VersionTwo { posts: images } => images,
    };
    Ok(images
        .into_iter()
        .map(|image| map_post(image, site_id))
        .collect())
}

fn map_post(image: ImagePost, site_id: &str) -> Post {
    Post {
        post: PostRef {
            site: SiteId::new(site_id),
            id: image.id.to_string(),
        },
        tags: image.tags.split_whitespace().map(str::to_owned).collect(),
        author: image.author,
        creator_id: image.creator_id,
        md5: image.md5,
        source: image.source,
        parent_id: image.parent_id.map(|id| id.to_string()),
        has_children: image.has_children,
        created_at: normalize_timestamp(image.created_at),
        width: image.width,
        height: image.height,
        rating: match image.rating.as_deref() {
            Some("s") => Rating::Safe,
            Some("q") => Rating::Questionable,
            Some("e") => Rating::Explicit,
            _ => Rating::Unknown,
        },
        score: image.score,
        preview_url: image.preview_url,
        sample_url: image.sample_url,
        full_url: image.file_url,
        file_size: image.file_size,
    }
}

fn normalize_timestamp(value: Option<WireTimestamp>) -> Option<String> {
    match value? {
        WireTimestamp::Text(value) => Some(value),
        WireTimestamp::UnixSeconds(seconds) => Some(unix_seconds_to_rfc3339(seconds)),
    }
}

fn unix_seconds_to_rfc3339(seconds: i64) -> String {
    let days = seconds.div_euclid(86_400);
    let day_seconds = seconds.rem_euclid(86_400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let day_of_era = z - era * 146_097;
    let year_of_era = (day_of_era - day_of_era / 1_460 + day_of_era / 36_524
        - day_of_era / 146_096)
        .div_euclid(365);
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_part = (5 * day_of_year + 2).div_euclid(153);
    let day = day_of_year - (153 * month_part + 2).div_euclid(5) + 1;
    let month = month_part + if month_part < 10 { 3 } else { -9 };
    let year = year + i64::from(month <= 2);
    let hour = day_seconds / 3_600;
    let minute = day_seconds % 3_600 / 60;
    let second = day_seconds % 60;
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

pub fn tag_expression(expression: &str, policy: ContentPolicy) -> Result<String> {
    if expression.trim().is_empty() {
        bail!("Moebooru tag expression must not be empty");
    }
    if expression.chars().any(char::is_control) {
        bail!("Moebooru tag expression contains a control character");
    }

    let policy_tag = match policy {
        ContentPolicy::SafeOnly => Some("rating:s"),
        ContentPolicy::AllowQuestionable => Some("-rating:e"),
        ContentPolicy::AllowExplicit | ContentPolicy::ExplicitOnly => None,
    };
    let mut result = expression.to_owned();
    if let Some(policy_tag) = policy_tag {
        if !result.split_whitespace().any(|term| term == policy_tag) {
            if !result.ends_with(char::is_whitespace) {
                result.push(' ');
            }
            result.push_str(policy_tag);
        }
    }
    if policy == ContentPolicy::ExplicitOnly
        && !result.split_whitespace().any(|term| term == "rating:e")
    {
        if !result.ends_with(char::is_whitespace) {
            result.push(' ');
        }
        result.push_str("rating:e");
    }
    Ok(result)
}

pub fn tag_query_params(
    expression: &str,
    policy: ContentPolicy,
    page: u32,
    page_size: u16,
) -> Result<Vec<(&'static str, String)>> {
    if page == 0 {
        bail!("Moebooru page numbers start at 1");
    }
    if page_size == 0 {
        bail!("Moebooru page size must be greater than zero");
    }
    Ok(vec![
        ("tags", tag_expression(expression, policy)?),
        ("page", page.to_string()),
        ("limit", page_size.to_string()),
    ])
}

pub fn tag_endpoint(base_url: &str) -> Result<String> {
    let mut url = reqwest::Url::parse(base_url).context("parse Moebooru API URL")?;
    url.set_path("/tag.json");
    url.set_query(None);
    Ok(url.to_string())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CalendarDate {
    year: i32,
    month: u32,
    day: u32,
}

fn parse_calendar_date(value: &str) -> Result<CalendarDate> {
    let mut parts = value.split('-');
    let year_text = parts.next().unwrap_or_default();
    let month_text = parts.next().unwrap_or_default();
    let day_text = parts.next().unwrap_or_default();
    let date = CalendarDate {
        year: year_text.parse().unwrap_or_default(),
        month: month_text.parse().unwrap_or_default(),
        day: day_text.parse().unwrap_or_default(),
    };
    if parts.next().is_some()
        || year_text.len() != 4
        || month_text.len() != 2
        || day_text.len() != 2
        || !year_text.chars().all(|value| value.is_ascii_digit())
        || !month_text.chars().all(|value| value.is_ascii_digit())
        || !day_text.chars().all(|value| value.is_ascii_digit())
        || !(1..=12).contains(&date.month)
        || !(1..=days_in_month(date.year, date.month)).contains(&date.day)
    {
        bail!("popular anchor date must use YYYY-MM-DD");
    }
    Ok(date)
}

fn is_leap_year(year: i32) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        2 if is_leap_year(year) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

fn add_days(mut date: CalendarDate, days: i32) -> CalendarDate {
    let step = if days < 0 { -1 } else { 1 };
    for _ in 0..days.unsigned_abs() {
        if step > 0 {
            if date.day == days_in_month(date.year, date.month) {
                date.day = 1;
                if date.month == 12 {
                    date.month = 1;
                    date.year += 1;
                } else {
                    date.month += 1;
                }
            } else {
                date.day += 1;
            }
        } else if date.day == 1 {
            if date.month == 1 {
                date.month = 12;
                date.year -= 1;
            } else {
                date.month -= 1;
            }
            date.day = days_in_month(date.year, date.month);
        } else {
            date.day -= 1;
        }
    }
    date
}

fn monday_first_weekday(date: CalendarDate) -> u32 {
    let (year, month) = if date.month < 3 {
        (date.year - 1, date.month + 12)
    } else {
        (date.year, date.month)
    };
    let century_year = year % 100;
    let century = year / 100;
    let saturday_first = (date.day as i32
        + (13 * (month as i32 + 1)) / 5
        + century_year
        + century_year / 4
        + century / 4
        + 5 * century)
        % 7;
    ((saturday_first + 5) % 7) as u32
}

fn format_calendar_date(date: CalendarDate) -> String {
    format!("{:04}-{:02}-{:02}", date.year, date.month, date.day)
}

pub fn popular_tag_expression(period: PopularPeriod, anchor_date: &str) -> Result<String> {
    let anchor = parse_calendar_date(anchor_date)?;
    let (start, end) = match period {
        PopularPeriod::Day => (anchor, anchor),
        PopularPeriod::Week => {
            let start = add_days(anchor, -(monday_first_weekday(anchor) as i32));
            (start, add_days(start, 6))
        }
        PopularPeriod::Month => {
            let start = CalendarDate { day: 1, ..anchor };
            (
                start,
                add_days(start, days_in_month(start.year, start.month) as i32 - 1),
            )
        }
    };
    let range = if start == end {
        format_calendar_date(start)
    } else {
        format!(
            "{}..{}",
            format_calendar_date(start),
            format_calendar_date(end)
        )
    };
    Ok(format!("date:{range} order:score"))
}

pub async fn query_posts(
    api_url: &str,
    request: &PostQueryRequest,
    network: &NetworkPolicy,
    site_id: &str,
    site_name: &str,
) -> Result<SitePage> {
    query_posts_with_cookie(api_url, request, network, None, site_id, site_name).await
}

pub async fn query_posts_with_cookie(
    api_url: &str,
    request: &PostQueryRequest,
    network: &NetworkPolicy,
    cookie_header: Option<&str>,
    site_id: &str,
    site_name: &str,
) -> Result<SitePage> {
    let (endpoint, params, page_size, fixed_window) = request_parts(api_url, request)?;
    let client = build_client_with_cookie(network, cookie_header)?;
    let mut posts = decode_posts(
        &get_bytes(&client, &endpoint, &params, network, site_name).await?,
        site_id,
    )?;
    retain_content_policy(&mut posts, request.query.content_policy);
    let returned_size = u16::try_from(posts.len()).unwrap_or(u16::MAX);
    Ok(SitePage {
        posts,
        continuation: Continuation::None,
        total: None,
        page_size: if fixed_window {
            returned_size
        } else {
            page_size
        },
        session: None,
    })
}

pub fn request_parts(
    api_url: &str,
    request: &PostQueryRequest,
) -> Result<(String, Vec<(&'static str, String)>, u16, bool)> {
    let (page, page_size) = match request.pagination {
        PaginationRequest::First { page_size } => (1, page_size),
        PaginationRequest::Page { number, page_size } => (number, page_size),
        PaginationRequest::FixedWindow => (1, 0),
    };
    let mut params = Vec::new();
    let (endpoint, fixed_window) = match &request.query.source {
        DiscoverySource::Browse
        | DiscoverySource::Feed {
            kind: FeedKind::Latest,
        } => {
            if matches!(request.pagination, PaginationRequest::FixedWindow) {
                bail!("latest Moebooru posts require page pagination");
            }
            params.push(("page", page.to_string()));
            params.push(("limit", page_size.to_string()));
            (api_url.to_owned(), false)
        }
        DiscoverySource::Search { .. } => {
            if matches!(request.pagination, PaginationRequest::FixedWindow) {
                bail!("Moebooru tag search requires page pagination");
            }
            (api_url.to_owned(), false)
        }
        DiscoverySource::Feed {
            kind:
                FeedKind::Popular {
                    period,
                    anchor_date,
                },
        } => {
            if matches!(request.pagination, PaginationRequest::FixedWindow) {
                bail!("Moebooru popular feeds use page pagination");
            }
            params.extend(tag_query_params(
                &popular_tag_expression(*period, anchor_date)?,
                request.query.content_policy,
                page,
                page_size,
            )?);
            (api_url.to_owned(), false)
        }
    };

    if let DiscoverySource::Search { expression } = &request.query.source {
        params.extend(tag_query_params(
            expression,
            request.query.content_policy,
            page,
            page_size,
        )?);
    } else if matches!(
        request.query.source,
        DiscoverySource::Browse
            | DiscoverySource::Feed {
                kind: FeedKind::Latest,
            }
    ) {
        if let Some(policy_tag) = content_policy_tag(request.query.content_policy) {
            params.push(("tags", policy_tag.to_owned()));
        }
    }
    Ok((endpoint, params, page_size, fixed_window))
}

fn content_policy_tag(policy: ContentPolicy) -> Option<&'static str> {
    match policy {
        ContentPolicy::SafeOnly => Some("rating:s"),
        ContentPolicy::AllowQuestionable => Some("-rating:e"),
        ContentPolicy::AllowExplicit | ContentPolicy::ExplicitOnly => None,
    }
}

fn matches_content_policy(post: &Post, policy: ContentPolicy) -> bool {
    match policy {
        ContentPolicy::SafeOnly => post.rating == Rating::Safe,
        ContentPolicy::AllowQuestionable => {
            matches!(post.rating, Rating::Safe | Rating::Questionable)
        }
        ContentPolicy::AllowExplicit => true,
        ContentPolicy::ExplicitOnly => post.rating == Rating::Explicit,
    }
}

pub fn retain_content_policy(posts: &mut Vec<Post>, policy: ContentPolicy) {
    posts.retain(|post| matches_content_policy(post, policy));
}

fn build_client(network: &NetworkPolicy) -> Result<reqwest::Client> {
    build_client_with_cookie(network, None)
}

fn build_client_with_cookie(
    network: &NetworkPolicy,
    cookie_header: Option<&str>,
) -> Result<reqwest::Client> {
    let mut builder = reqwest::Client::builder().user_agent(USER_AGENT);
    match &network.proxy {
        ProxyMode::Auto => {}
        ProxyMode::Direct => builder = builder.no_proxy(),
        ProxyMode::Manual { url } => builder = builder.proxy(reqwest::Proxy::all(url)?),
    }
    if let Some(cookie_header) = cookie_header {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::COOKIE,
            reqwest::header::HeaderValue::from_str(cookie_header)?,
        );
        builder = builder.default_headers(headers);
    }
    Ok(builder.build()?)
}

pub fn map_http_status(status: reqwest::StatusCode, site_name: &str) -> SiteError {
    let (code, retryable) = match status {
        reqwest::StatusCode::UNAUTHORIZED | reqwest::StatusCode::FORBIDDEN => {
            (SiteErrorCode::AuthRequired, false)
        }
        reqwest::StatusCode::TOO_MANY_REQUESTS => (SiteErrorCode::RateLimited, true),
        status if status.is_client_error() => (SiteErrorCode::InvalidRequest, false),
        _ => (SiteErrorCode::NetworkFailed, true),
    };
    SiteError {
        code,
        message: format!("{site_name} request failed with HTTP {status}"),
        retryable,
    }
}

async fn get_bytes(
    client: &reqwest::Client,
    endpoint: &str,
    params: &[(&str, String)],
    network: &NetworkPolicy,
    site_name: &str,
) -> Result<Vec<u8>> {
    let mut retry = 0;
    loop {
        let response = client.get(endpoint).query(params).send().await?;
        if response.status().is_success() {
            return Ok(response.bytes().await?.to_vec());
        }

        let status = response.status();
        if retry >= network.max_retries || !matches!(status.as_u16(), 429 | 500 | 502 | 503 | 504) {
            let error = map_http_status(status, site_name);
            bail!("{}", error.message);
        }
        let delay_ms = response
            .headers()
            .get(reqwest::header::RETRY_AFTER)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<u64>().ok())
            .map(|seconds| seconds.saturating_mul(1_000))
            .unwrap_or_else(|| {
                network
                    .retry_delay_ms
                    .saturating_mul(1_u64 << retry.min(16))
            })
            .min(network.max_retry_delay_ms);
        tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
        retry += 1;
    }
}

pub async fn fetch_bytes(
    endpoint: &str,
    params: &[(&str, String)],
    network: &NetworkPolicy,
    site_name: &str,
) -> Result<Vec<u8>> {
    let client = build_client(network)?;
    get_bytes(&client, endpoint, params, network, site_name).await
}

fn map_tag_category(category: Option<u8>) -> Option<TagCategory> {
    category.map(|value| match value {
        0 => TagCategory::General,
        1 => TagCategory::Artist,
        3 => TagCategory::Copyright,
        4 => TagCategory::Character,
        5 => TagCategory::Metadata,
        value => TagCategory::Unknown(value),
    })
}

pub fn decode_tag_suggestions(body: &[u8]) -> Result<Vec<TagSuggestion>> {
    let records =
        serde_json::from_slice::<Vec<TagRecord>>(body).context("decode Moebooru tag response")?;
    Ok(records
        .into_iter()
        .map(|record| TagSuggestion {
            name: record.name,
            category: map_tag_category(record.category),
            post_count: record.count,
            ambiguous: record.ambiguous,
            aliases: Vec::new(),
        })
        .collect())
}

pub async fn fetch_tag_suggestions(
    url: &str,
    request: &TagSuggestionRequest,
    network: &NetworkPolicy,
    site_name: &str,
) -> Result<Vec<TagSuggestion>> {
    let params = [
        ("name", request.query.clone()),
        ("limit", request.limit.to_string()),
    ];
    let client = build_client(network)?;
    decode_tag_suggestions(&get_bytes(&client, url, &params, network, site_name).await?)
}

pub async fn set_favorite(
    api_url: &str,
    post_id: &str,
    favorite: bool,
    cookie_header: &str,
    network: &NetworkPolicy,
    site_name: &str,
) -> Result<()> {
    if post_id.is_empty() || !post_id.chars().all(|value| value.is_ascii_digit()) {
        bail!("{site_name} post id must be numeric");
    }
    let mut endpoint = reqwest::Url::parse(api_url).context("parse Moebooru API URL")?;
    endpoint.set_path("/post/vote.json");
    endpoint.set_query(None);
    let client = build_client_with_cookie(network, Some(cookie_header))?;
    let response = client
        .post(endpoint)
        .form(&[("id", post_id), ("score", if favorite { "3" } else { "2" })])
        .send()
        .await?;
    if response.status().is_success() {
        Ok(())
    } else {
        bail!("{}", map_http_status(response.status(), site_name).message)
    }
}

pub async fn fetch_images(url: &str, page: usize, site_id: &str) -> Result<Vec<Post>> {
    let network = NetworkPolicy::default();
    let client = build_client(&network)?;
    let params = [("page", page.to_string())];
    decode_posts(
        &get_bytes(&client, url, &params, &network, "Moebooru").await?,
        site_id,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const IMAGE_JSON: &str = r#"{
        "id": 407162,
        "tags": "barefoot blue_eyes cirno",
        "author": "otaku_emmy",
        "created_at": 1786390664,
        "file_url": "https://konachan.net/image/file.png",
        "sample_url": "https://konachan.net/sample/file.jpg",
        "preview_url": "https://konachan.net/data/preview/file.jpg",
        "rating": "s",
        "score": 17,
        "md5": "512100c4db5c913ec30c6ed3c5b913d6",
        "width": 5302,
        "height": 2832
    }"#;

    #[test]
    fn decodes_both_moebooru_post_envelopes_with_site_identity() {
        let legacy = decode_posts(format!("[{IMAGE_JSON}]").as_bytes(), "konachan").unwrap();
        let version_two = decode_posts(
            format!(r#"{{"posts":[{IMAGE_JSON}]}}"#).as_bytes(),
            "konachan",
        )
        .unwrap();

        assert_eq!(legacy, version_two);
        assert_eq!(legacy[0].post.site.as_str(), "konachan");
        assert_eq!(
            legacy[0].created_at.as_deref(),
            Some("2026-08-10T19:37:44Z")
        );
    }

    #[test]
    fn preserves_tag_query_and_popular_window_rules() {
        assert_eq!(
            tag_expression("artist_name -sketch", ContentPolicy::SafeOnly).unwrap(),
            "artist_name -sketch rating:s"
        );
        assert_eq!(
            popular_tag_expression(PopularPeriod::Month, "2026-08-10").unwrap(),
            "date:2026-08-01..2026-08-31 order:score"
        );
    }

    #[test]
    fn maps_tag_categories() {
        let tags = decode_tag_suggestions(
            br#"[{"name":"artist_name","count":12,"type":1,"ambiguous":false}]"#,
        )
        .unwrap();

        assert_eq!(tags[0].category, Some(TagCategory::Artist));
        assert_eq!(tags[0].post_count, Some(12));
    }
}
