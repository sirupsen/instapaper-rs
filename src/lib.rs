//! Rust wrapper for the Instapaper public API.  The official API's documentation can be found
//! [here](https://www.instapaper.com/api). Note that in order to receive a consumer key and secret
//! to access the API you must fill out [this
//! form](https://www.instapaper.com/main/request_oauth_consumer_token). See the `Client` struct for all methods made available.
//!
//! ## Installation
//!
//! Add `instapaper = "*"` to your `Cargo.toml`.
//!
//! ## Example
//!
//! ```
//! extern crate dotenv;
//!
//! use dotenv::dotenv;
//! use std::env;
//!
//! dotenv().ok();
//!
//! for (key, value) in env::vars() {
//!   println!("{}: {}", key, value);
//! }
//!
//! // Instapaper uses the archaic Oauth1 which requires the username and password in order to
//! // receive an oauth token required for further operations.
//! let client = instapaper::authenticate(
//!     &env::var("INSTAPAPER_USERNAME").unwrap(),
//!     &env::var("INSTAPAPER_PASSWORD").unwrap(),
//!     &env::var("INSTAPAPER_CONSUMER_KEY").unwrap(),
//!     &env::var("INSTAPAPER_CONSUMER_SECRET").unwrap(),
//! ).expect("failed to authenticate");
//!
//!// Now the `oauth_key` and `oauth_secret` on `instapaper::Client` has been set to make it valid
//!// for API actions
//! client.add("https://sirupsen.com/read", "How I Read", "").unwrap();
//! println!("{:?}", client.bookmarks().unwrap());
//!
//! println!("Client {{");
//! println!("  consumer_key: {}", client.consumer_key);
//! println!("  consumer_secret: {}", client.consumer_secret);
//! println!("  oauth_key: {}", client.oauth_key.as_ref().unwrap());
//! println!("  oauth_secret: {}", client.oauth_secret.as_ref().unwrap());
//! println!("}}");
//!
//! // You can save the Oauth authentication details to e.g. an enviroment file or wherever you
//! // store secrets and discard the username and password.
//! let client2 = instapaper::Client {
//!     consumer_key: env::var("INSTAPAPER_CONSUMER_KEY").unwrap().to_owned(),
//!     consumer_secret: env::var("INSTAPAPER_CONSUMER_SECRET").unwrap().to_owned(),
//!     oauth_key: client.oauth_key,
//!     oauth_secret: client.oauth_secret,
//! };
//!
//! println!("{:?}", client2.bookmarks().unwrap());
//! ```
//!
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
#[macro_use]
extern crate failure;
extern crate oauth1;
extern crate reqwest;
extern crate url;

#[cfg(test)]
extern crate mockito;

use std::borrow::Cow;
use std::collections::HashMap;
use std::iter::FromIterator;

use oauth1::Token;
use url::Url;

use failure::Error;

#[cfg(not(test))]
const URL: &str = "https://www.instapaper.com";
#[cfg(test)]
const URL: &str = mockito::SERVER_URL;

type Result<T> = std::result::Result<T, Error>;

/// The client instance to perform actions on. The `consumer_key` and `consumer_secret` are
/// obtained through Instapaper's API documentation. The `oauth_key` and `oauth_secret` are
/// obtained with the user's `username`, `password`, `consumer_key`, and `consumer_secret` by
/// calling `authenticate()` on a Client.
#[derive(Debug, Clone, Default)]
pub struct Client {
    pub consumer_key: String,
    pub consumer_secret: String,
    pub oauth_key: Option<String>,
    pub oauth_secret: Option<String>,
}

/// Individual bookmarks, which is the API's lingo for a piece of media to be consumer later
/// (video, article, etc.)
#[derive(Deserialize, Debug, Clone, Default)]
#[cfg_attr(test, derive(Serialize))]
pub struct Bookmark {
    pub title: String,
    pub hash: String,
    pub bookmark_id: i64,
    pub progress_timestamp: f64,
    pub description: String,
    pub url: String,
    pub time: f64,
    pub starred: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub private_source: String,
}

/// Bare-bones information about the user.
#[derive(Deserialize, Debug, Clone, Default)]
#[cfg_attr(test, derive(Serialize))]
pub struct User {
    pub username: String,
    pub user_id: i64,
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(rename = "subscription_is_active")]
    pub subscription: String,
}

/// Individual article highlights.
#[derive(Deserialize, Debug, Clone, Default)]
#[cfg_attr(test, derive(Serialize))]
pub struct Highlight {
    pub highlight_id: i64,
    pub bookmark_id: i64,
    pub text: String,
    pub note: Option<String>,
    pub time: i64,
    pub position: i64,
    #[serde(rename = "type")]
    pub kind: String,
}

/// API response from `bookmarks()` which contains highlights and bookmarks.
#[derive(Deserialize, Debug, Clone, Default)]
#[cfg_attr(test, derive(Serialize))]
pub struct List {
    pub bookmarks: Vec<Bookmark>,
    pub user: User,
    pub highlights: Vec<Highlight>,
    #[serde(default)]
    pub delete_ids: Vec<i64>,
}

/// Must be called to obtain the `oauth_key` and `oauth_secret`. Once you have them, you don't need
/// to call this every time you want to access the API. You can store the resulting client's
/// attributes somewhere and instantiate it yourself without this method. See the module-level
/// documentation for a complete example.
pub fn authenticate(username: &str, password: &str, consumer_key: &str, consumer_secret: &str) -> Result<Client> {
    let mut params: HashMap<&str, Cow<str>> = HashMap::new();
    params.insert("x_auth_username", Cow::Borrowed(username));
    params.insert("x_auth_password", Cow::Borrowed(password));
    params.insert("x_auth_mode", Cow::Borrowed("client_auth"));

    let mut client = Client {
        consumer_key: consumer_key.to_owned(),
        consumer_secret: consumer_secret.to_owned(),
        oauth_key: None,
        oauth_secret: None,
    };

    let mut response = signed_request("oauth/access_token", params, &client)?;
    let qline = response.text()?;

    // TODO: This is such a roundabout way to properly parse the URI params, but I haven't found
    // another API and this function doesn't take anything but a fully qualified path.
    let qline = format!("https://junk.com/?{}", qline);
    let url = Url::parse(&qline)?;
    let query_params: HashMap<String, String> = HashMap::from_iter(url.query_pairs().into_owned());

    let oauth_token = query_params.get("oauth_token");
    let oauth_secret_token = query_params.get("oauth_token_secret");

    if oauth_token.is_none() || oauth_secret_token.is_none() {
        Err(format_err!("oauth_tokens not both in response: {}", qline))
    } else {
        client.oauth_key = Some(oauth_token.unwrap().to_owned());
        client.oauth_secret = Some(oauth_secret_token.unwrap().to_owned());
        Ok(client)
    }
}

impl Client {
    /// Verifies credentials, mostly used for testing.
    pub fn verify(&self) -> Result<User> {
        let params = HashMap::new();
        let mut response = signed_request("account/verify_credentials", params, self)?;
        let users: Vec<User> = response.json()?;
        Ok(users[0].clone())
    }

    /// Move a `Bookmark` to the archive folder.
    pub fn archive(&self, bookmark_id: i64) -> Result<Bookmark> {
        let bookmark_id_string = bookmark_id.to_string();
        let mut params: HashMap<&str, Cow<str>> = HashMap::new();
        params.insert("bookmark_id", Cow::Borrowed(&bookmark_id_string));
        let mut response = signed_request("bookmarks/archive", params, self)?;
        let bookmarks: Vec<Bookmark> = response.json()?;
        Ok(bookmarks[0].clone())
    }

    /// List all bookmarks and highlights in a folder. You'll need to obtain the folder id through either the API
    /// or the URL on Instapaper. `unread` and `archive` work as strings.
    pub fn bookmarks_in(&self, folder: &str) -> Result<List> {
        let mut params: HashMap<&str, Cow<str>> = HashMap::new();
        params.insert("limit", Cow::Borrowed("500"));
        params.insert("folder_id", Cow::Borrowed(folder));
        let mut response = signed_request("bookmarks/list", params, self)?;
        response.json().map_err(|x| x.into())
    }


    /// List all bookmarks and highlights in the `unread` folder.
    pub fn bookmarks(&self) -> Result<List> {
        self.bookmarks_in("unread")
    }

    /// Add a bookmark. Pass a blank `title` and `description` if you want Instapaper's default.
    pub fn add(&self, url: &str, title: &str, description: &str) -> Result<Bookmark> {
        let mut params: HashMap<&str, Cow<str>> = HashMap::new();
        params.insert("url", Cow::Borrowed(&url));
        if !title.is_empty() {
            params.insert("title", Cow::Borrowed(&title));
        }
        if !description.is_empty() {
            params.insert("description", Cow::Borrowed(&description));
        }

        let mut response = signed_request("bookmarks/add", params, self)?;
        let bookmarks: Vec<Bookmark> = response.json()?;
        Ok(bookmarks[0].clone())
    }
}

fn signed_request(
    action: &str,
    params: HashMap<&'static str, Cow<str>>,
    client: &Client,
) -> reqwest::Result<reqwest::Response> {
    let http_client = reqwest::Client::new();
    let url = format!("{}/api/1.1/{}", URL, action);
    let empty = String::new();
    let token = Token::new(
        client.oauth_key.as_ref().unwrap_or(&empty),
        client.oauth_secret.as_ref().unwrap_or(&empty),
    );
    let oauth: Option<&Token> = if client.oauth_key.as_ref().is_some() {
        Some(&token)
    } else {
        None
    };

    let request = http_client
        .post(&url)
        .form(&params)
        .header(
            reqwest::header::AUTHORIZATION,
            oauth1::authorize(
                "POST",
                &url,
                &Token::new(
                    &client.consumer_key,
                    &client.consumer_secret,
                ),
                oauth,
                Some(params),
            ),
        ).build()?;
    http_client.execute(request)?.error_for_status()
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::mock;

    fn client() -> Client {
        Client {
            consumer_key: String::new(),
            consumer_secret: String::new(),
            oauth_key: Some(String::new()),
            oauth_secret: Some(String::new()),
        }
    }

    #[test]
    fn test_add_bookmark() {
        let bookmark = vec![Bookmark {
            title: "How I Read".to_string(),
            ..Bookmark::default()
        }];
        let json = serde_json::to_string(&bookmark).unwrap();

        let _m = mock("POST", "/api/1.1/bookmarks/add")
            .with_status(201)
            .with_header("content-type", "application/json")
            .with_body(&json)
            .create();

        let result = client().add("https://sirupsen.com/read", "How I Read", "");
        assert!(result.is_ok(), result.err().unwrap().to_string())
    }

    #[test]
    fn test_add_bookmark_garbage_json() {
        let _m = mock("POST", "/api/1.1/bookmarks/add")
            .with_status(201)
            .with_header("content-type", "application/json")
            .with_body(r#"[garbageeee]"#)
            .create();

        let result = client().add("https://sirupsen.com/read", "How I Read", "");
        assert!(result.is_err(), "Expected an error on garbage");
        let err = result.err().unwrap();
        assert_eq!("expected value at line 1 column 2", err.to_string());
    }

    #[test]
    fn test_add_bookmark_error_code() {
        let _m = mock("POST", "/api/1.1/bookmarks/add")
            .with_status(500)
            .with_header("content-type", "application/json")
            .with_body(r#""#)
            .create();

        let result = client().add("https://sirupsen.com/read", "How I Read", "");
        assert!(result.is_err(), "Expected an error on 500");
    }

    #[test]
    fn test_authenticate() {
        let _m = mock("POST", "/api/1.1/oauth/access_token")
            .with_status(200)
            .with_header("content-type", "application/text")
            .with_body(r#"oauth_token=token&oauth_token_secret=secret"#)
            .create();

        let result = authenticate("username", "password", "key", "secret");
        assert!(result.is_ok(), result.err().unwrap().to_string());
        let client = result.unwrap();
        assert_eq!("token", client.oauth_key.unwrap());
        assert_eq!("secret", client.oauth_secret.unwrap());
    }

    #[test]
    fn test_authenticate_reversed() {
        let _m = mock("POST", "/api/1.1/oauth/access_token")
            .with_status(200)
            .with_header("content-type", "application/text")
            .with_body(r#"oauth_token_secret=secret&oauth_token=token"#)
            .create();

        let result = authenticate("username", "password", "key", "secret");
        assert!(result.is_ok(), result.err().unwrap().to_string());
        let client = result.unwrap();
        assert_eq!("token", client.oauth_key.unwrap());
        assert_eq!("secret", client.oauth_secret.unwrap());
    }

    #[test]
    fn test_authenticate_corrupted_qline() {
        let _m = mock("POST", "/api/1.1/oauth/access_token")
            .with_status(200)
            .with_header("content-type", "application/text")
            .with_body(r#"badqline"#)
            .create();

        let result = authenticate("username", "password", "key", "secret");
        assert!(result.is_err(), "Expected an error");
        let err = result.err().unwrap();
        assert_eq!(
            "oauth_tokens not both in response: https://junk.com/?badqline",
            err.to_string()
        )
    }

    #[test]
    fn test_authenticate_qline_one_good_result() {
        let _m = mock("POST", "/api/1.1/oauth/access_token")
            .with_status(200)
            .with_header("content-type", "application/text")
            .with_body(r#"oauth_token=1&oauth_noep=walrus"#)
            .create();

        let result = authenticate("username", "password", "key", "secret");
        assert!(result.is_err(), "Expected an error");
        let err = result.err().unwrap();
        assert_eq!(
            "oauth_tokens not both in response: https://junk.com/?oauth_token=1&oauth_noep=walrus",
            err.to_string()
        )
    }

    #[test]
    fn test_bookmarks() {
        let list = List::default();
        let json = serde_json::to_string(&list).unwrap();

        let _m = mock("POST", "/api/1.1/bookmarks/list")
            .with_status(201)
            .with_header("content-type", "application/json")
            .with_body(&json)
            .create();

        let result = client().bookmarks();
        assert!(result.is_ok(), result.err().unwrap().to_string())
    }

    #[test]
    fn test_bookmarks_error_status() {
        let _m = mock("POST", "/api/1.1/bookmarks/list")
            .with_status(500)
            .with_header("content-type", "application/json")
            .with_body("argh error!")
            .create();

        let result = client().bookmarks();
        assert!(result.is_err(), "Expected an error on 500");
    }

    #[test]
    fn test_verify() {
        let user = vec![User::default()];
        let json = serde_json::to_string(&user).unwrap();

        let _m = mock("POST", "/api/1.1/account/verify_credentials")
            .with_status(201)
            .with_header("content-type", "application/json")
            .with_body(&json)
            .create();

        let result = client().verify();
        assert!(result.is_ok(), result.err().unwrap().to_string())
    }

    #[test]
    fn test_verify_server_error() {
        let _m = mock("POST", "/api/1.1/account/verify_credentials")
            .with_status(500)
            .with_header("content-type", "application/json")
            .with_body("omgggg")
            .create();

        let result = client().verify();
        assert!(result.is_err(), "Expected an error on 500");
    }
}
