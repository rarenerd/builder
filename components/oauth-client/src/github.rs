// Copyright (c) 2018 Chef Software Inc. and/or applicable contributors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use reqwest::{Client, header::{Accept, Authorization, Bearer, Headers, qitem}, mime::{Mime, TopLevel, SubLevel}};
use serde_json;

use error::{Error, Result};
use types::*;

pub struct GitHub;

// TODO: Yes the JSON parsing here is untyped, and that is on purpose. All the types for these GH
// responses live in the github-api-client crate and it doesn't feel right for this crate to take
// a dependency on that crate just to get those types in. Maybe we can revisit this in the future
// to see what a better way of sharing types between crates would look like (as this feels like
// a common problem in our codebase) but for now, I'm doing it this way. The odds of GH changing
// their response JSON seem low.
impl OAuthProvider for GitHub {
    fn authenticate(&self, client: &Client, code: &str) -> Result<String> {
        let url = format!(
            "{}?client_id={}&client_secret={}&code={}",
            client.config.token_url,
            client.config.client_id,
            client.config.client_secret,
            code
        );

        let resp = client.post(&url).send().map_err(Error::HttpClient)?;

        if resp.status.is_success() {
            let msg: serde_json::Value = resp.json().map_err(Error::HttpClient)?;
            Ok(msg["access_token"])
        } else {
            let body = resp.text().map_err(Error::HttpClient)?;
            Err(Error::HttpResponse(resp.status, body))
        }
    }

    fn user(&self, client: &Client, token: &str) -> Result<User> {
        let mut headers = Headers::new();
        headers.set(Accept(vec![
            qitem(
                Mime(TopLevel::Application, SubLevel::Json, vec![])
            ),
            qitem("application/vnd.github.v3+json".parse().unwrap()),
            qitem(
                "application/vnd.github.machine-man-preview+json"
                    .parse()
                    .unwrap()
            ),
        ]));
        headers.set(Authorization(Bearer { token: token.to_string() }));

        let resp = client.headers(headers).get(&client.config.user_url).send().map_err(Error::HttpClient)?;

        if resp.status.is_success() {
            let msg: serde_json::Value = resp.json().map_err(Error::HttpClient)?;
            let user = User {
                id: user["id"].to_string(),
                username: user["login"],
                email: user["email"]
            };
            Ok(user)
        } else {
            let body = resp.text().map_err(Error::HttpClient)?;
            Err(Error::HttpResponse(resp.status, body))
        }
    }

    fn name(&self) -> String {
        String::from("github")
    }
}
