# instapaper

Rust wrapper for the Instapaper public API.  The official API's documentation can be found
[here](https://www.instapaper.com/api). Note that in order to receive a consumer key and secret
to access the API you must fill out [this
form](https://www.instapaper.com/main/request_oauth_consumer_token). See the `Client` struct for all methods made available.

### Example

```rust
extern crate dotenv;

use dotenv::dotenv;
use std::env;

dotenv().ok();

for (key, value) in env::vars() {
  println!("{}: {}", key, value);
}

// Instapaper uses the archaic Oauth1 which requires the username and password in order to
// receive an oauth token required for further operations.
let client = instapaper::authenticate(
    &env::var("INSTAPAPER_USERNAME").unwrap(),
    &env::var("INSTAPAPER_PASSWORD").unwrap(),
    &env::var("INSTAPAPER_CONSUMER_KEY").unwrap(),
    &env::var("INSTAPAPER_CONSUMER_SECRET").unwrap(),
).expect("failed to authenticate");

// Now the `oauth_key` and `oauth_secret` on `instapaper::Client` has been set to make it valid
// for API actions
client.add("https://sirupsen.com/read", "How I Read", "").unwrap();
println!("{:?}", client.bookmarks().unwrap());

println!("Client {{");
println!("  consumer_key: {}", client.consumer_key);
println!("  consumer_secret: {}", client.consumer_secret);
println!("  oauth_key: {}", client.oauth_key.as_ref().unwrap());
println!("  oauth_secret: {}", client.oauth_secret.as_ref().unwrap());
println!("}}");

// You can save the Oauth authentication details to e.g. an enviroment file or wherever you
// store secrets and discard the username and password.
let client2 = instapaper::Client {
    consumer_key: env::var("INSTAPAPER_CONSUMER_KEY").unwrap().to_owned(),
    consumer_secret: env::var("INSTAPAPER_CONSUMER_SECRET").unwrap().to_owned(),
    oauth_key: client.oauth_key,
    oauth_secret: client.oauth_secret,
};

println!("{:?}", client2.bookmarks().unwrap());
```


License: MIT
