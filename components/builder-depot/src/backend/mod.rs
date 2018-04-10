// Copyright (c) 2016 Chef Software Inc. and/or applicable contributors
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

//! Contract for storage and retrieval of job logs from long-term
//! storage.
//!
//! As jobs are running, their log output is collected in files on the
//! job server. Once they are complete, however, we would like to
//! store them elsewhere for safety; the job server should be
//! stateless.
//!

pub mod s3;

use error::Result;
use std::path::PathBuf;

use hab_core::package::{PackageTarget, PackageIdent};

/// Currently implemented artifact store backends
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ArtBackend {
    S3,
}

pub trait ArtifactHandler {
    /// More space for notes
    fn s3_key(&self, ident: &PackageIdent, target: &PackageTarget) -> PathBuf;

    /// Takes a hartfile and pushes it to S3
    fn upload(&self, hart: &PathBuf, ident: &PackageIdent, target: &PackageTarget) -> Result<()>;

    /// Hold that thought
    fn download(&self, loc: &PathBuf, hart: &PackageIdent, target: &PackageTarget) -> Result<()>;

    /// Checks whether a package exists in S3
    fn exists<T>(&self, ident: &T, target: &PackageTarget) -> Result<bool>;
}
