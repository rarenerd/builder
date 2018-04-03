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

use types::*;

pub struct GitHub;

impl OAuthProvider for GitHub {
    fn authenticate(&self, code: &str) -> OAuthResult<String> {
        let url = Url::parse(&format!(
            "{}?client_id={}&client_secret={}&code={}",
            self.token_url,
            self.client_id,
            self.client_secret,
            code
        )).map_err(OAuthError::HttpClientParse)?;

        Counter::Authenticate.increment();
        let mut rep = match http_post(url, None::<String>) {
            Ok(r) => r,
            Err(e) => return Err(OAuthError::Hub(e.to_string())),
        };
        let mut body = String::new();
        rep.read_to_string(&mut body)?;

        if rep.status.is_success() {
            debug!("GitHub response body, {}", body);
            match serde_json::from_str::<AuthOk>(&body) {
                Ok(msg) => Ok(msg.access_token),
                Err(e) => Err(OAuthError::Serialization(e)),
            }
        } else {
            Err(OAuthError::HttpResponse(rep.status, body))
        }
    }

    fn user(&self, token: &OAuthUserToken) -> OAuthResult<OAuthUser> {
        let url = Url::parse(&format!("{}/user", self.api_url)).unwrap();
        Counter::UserApi("user").increment();
        let mut rep = match http_get(url, Some(token)) {
            Ok(r) => r,
            Err(e) => return Err(OAuthError::Hub(e.to_string())),
        };
        let mut body = String::new();
        rep.read_to_string(&mut body)?;
        debug!("GitHub response body, {}", body);

        if rep.status != StatusCode::Ok {
            return Err(OAuthError::HttpResponse(rep.status, body));
        }

        let user: User = serde_json::from_str(&body).map_err(
            OAuthError::Serialization,
        )?;

        Ok(OAuthUser {
            id: user.id.to_string(),
            username: user.login,
            email: user.email,
        })
    }

    fn name(&self) -> String {
        String::from("github")
    }
}
