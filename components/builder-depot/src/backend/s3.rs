// Copyright (c) 2016-2017 Chef Software Inc. and/or applicable contributors
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

//! Pkg storage backend variant which uses S3 (or an API compatible clone) for
//! hart storage.
//!
//! Has been tested against AWS S3.
//!
//! All packages are stored in a single bucket, using the fully qualified
//! package ident as the key, prefixed with `pkgs`
//!
//! # Configuration
//!
//! Currently the S3Handler must be configured with both an access key
//! ID and a secret access key.
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{PathBuf, Path};
use std::str::FromStr;

use hab_core::package::{PackageTarget, Identifiable, PackageIdent};

use rusoto::{Region, credential::StaticProvider, reactor} ;
use rusoto_s3::{S3, S3Client, GetObjectRequest, PutObjectRequest, ListObjectsRequest};
use futures::{Future, Stream};

use DepotUtil;
use config::S3Cfg;

use error::{Result, Error};

pub struct S3Handler {
    client: S3Client<StaticProvider, reactor::RequestDispatcher>,
    bucket: String,
}

impl S3Handler {
    // The S3 Handler struct contains all of the credential
    // and target information that we should need to perfom
    // any backend operations
    pub fn new(config: &S3Cfg) -> S3Handler {
        let region =
            match config.endpoint {
                Some(ref value) => {
                    Region::Custom { name: "minio_s3".to_owned(), endpoint: value.to_string() }
                },
                None => Region::from_str(config.aws_region.as_str()).unwrap()
        };
        let aws_id = &config.aws_key_id;
        let aws_secret = &config.aws_secret_key;
        let cred_provider = StaticProvider::new_minimal(aws_id.to_string(), aws_secret.to_string());
        let client = S3Client::new(reactor::RequestDispatcher::default(), cred_provider, region);
        let bucket = &config.s3bucket_name;

        S3Handler {
            client: client,
            bucket: bucket.to_string(),
        }
    }

    // Helper function for programmatic creation of
    // the s3 object key
    pub fn s3_key<T>(&self, ident: &T, target: &PackageTarget) -> PathBuf
    where
        T: Identifiable,
    {
        Path::new(ident.origin())
            .join(format!("{}", ident.name()))
            .join(format!("{}", ident.version().unwrap()))
            .join(format!("{}", ident.release().unwrap()))
            .join(format!("{}", target.architecture))
            .join(format!("{}", target.platform))
    }

    // Helper function to validate whether or not a package
    // exists in the configured bucket
    pub fn exists<T>(&self, ident: &T, target: &PackageTarget) -> Result<bool>
    where
        T: Identifiable,
    {
        let mut request = ListObjectsRequest::default();
        let hotpocket = self.bucket.to_string();
        let key = self.s3_key(ident, target).to_string_lossy().into_owned();
        request.bucket = hotpocket.clone();

        match self.client.list_buckets().sync() {
            Ok(bucket_list) => {
                match bucket_list.buckets {
                    Some(bkts) => {
                        for bucket in bkts {
                            if bucket.name.unwrap() == hotpocket {
                                match self.client.list_objects(&request).sync() {
                                    Ok(object_list) => {
                                        match object_list.contents{
                                            Some(contents) => {
                                                for object in contents {
                                                    if object.key.unwrap() == key {
                                                        return Ok(true);
                                                    }
                                                }
                                                return Ok(false);
                                            },
                                            None => return Ok(false),
                                        }
                                    },
                                    Err(e) => return Err(Error::ObjectError(e)),
                                }
                            }
                        }
                        return Ok(false);
                    }
                    None => Ok(false),
                }
            },
            Err(e) => Err(Error::BadBucket(e)),
        }
    }

    pub fn upload(&self, hart: &PathBuf, ident: &PackageIdent, target: &PackageTarget) -> Result<()>
    {
        let key = self.s3_key(ident, target).to_string_lossy().into_owned();
        let buck = self.bucket.to_string();
        let file = File::open(hart).unwrap();
        let mut reader = BufReader::new(file);
        let mut object:Vec<u8> = Vec::new();
        let _result = reader.read_to_end(&mut object);

        let mut request = PutObjectRequest::default();
        request.key = key;
        request.bucket = buck;
        request.body = Some(object);
        request.content_type = Some("archive".to_string());
        // TODO (ian): A maximum single PUT to S3 is 5gb
        // rusoto supports PUT in chunks.  This upload
        // should chunk the upload request to stay <limit
        match self.client.put_object(&request).sync() {
             Ok(_) => Ok(()), // normal result
             Err(e) => {
                 Err(Error::PackageUpload(e))
             }
         }
    }

    pub fn download(&self, loc: &PathBuf, ident: &PackageIdent, target: &PackageTarget) -> Result<()> {
        let mut request = GetObjectRequest::default();
        let key = self.s3_key(ident, target).to_string_lossy().into_owned();
        request.bucket = self.bucket.clone();
        request.key = key;

        let payload = self.client.get_object(&request).sync();
        let body = match payload {
            Ok(response) => response.body, // normal result
            Err(e) => {
                return Err(Error::PackageDownload(e));
            }
        };

        let file = body.expect("Downloaded pkg archive empty!").concat2();
        let _result = DepotUtil::write_archive(&loc, file.wait().unwrap());
        Ok(())
    }

}
