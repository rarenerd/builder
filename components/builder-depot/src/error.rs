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

use std::error;
use std::ffi;
use std::io;
use std::fmt;
use std::result;

use hab_core;
use hab_core::package::{self, Identifiable};
use rusoto_s3;
use hab_net;
use hyper;

#[derive(Debug)]
pub enum Error {
    BadPort(String),
    BadBucket(rusoto_s3::ListBucketsError),
    CaughtPanic(String, String),
    ChannelAlreadyExists(String),
    ChannelDoesNotExist(String),
    HabitatCore(hab_core::Error),
    HabitatNet(hab_net::error::LibError),
    HTTP(hyper::status::StatusCode),
    InvalidPackageIdent(String),
    IO(io::Error),
    MessageTypeNotFound,
    NetError(hab_net::NetError),
    NoXFilename,
    NoFilePart,
    NulError(ffi::NulError),
    ObjectError(rusoto_s3::ListObjectsError),
    PackageIsAlreadyInChannel(String, String),
    PackageUpload(rusoto_s3::PutObjectError),
    PackageDownload(rusoto_s3::GetObjectError),
    RemotePackageNotFound(package::PackageIdent),
    WriteSyncFailed,
}

pub type Result<T> = result::Result<T, Error>;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let msg = match *self {
            Error::BadPort(ref e) => format!("{} is an invalid port. Valid range 1-65535.", e),
            Error::BadBucket(ref e) => format!("{}", e),
            Error::CaughtPanic(ref msg, ref source) => {
                format!("Caught a panic during upload: {}. {}", msg, source)
            }
            Error::ChannelAlreadyExists(ref e) => format!("{} already exists.", e),
            Error::ChannelDoesNotExist(ref e) => format!("{} does not exist.", e),
            Error::HabitatCore(ref e) => format!("{}", e),
            Error::HabitatNet(ref e) => format!("{}", e),
            Error::HTTP(ref e) => format!("{}", e),
            Error::InvalidPackageIdent(ref e) => {
                format!(
                    "Invalid package identifier: {:?}. A valid identifier is in the form \
                         origin/name (example: acme/redis)",
                    e
                )
            }
            Error::IO(ref e) => format!("{}", e),
            Error::MessageTypeNotFound => format!("Unable to find message for given type"),
            Error::NetError(ref e) => format!("{}", e),
            Error::NoXFilename => {
                format!("Invalid download from Builder - missing X-Filename header")
            }
            Error::NoFilePart => {
                format!(
                    "An invalid path was passed - we needed a filename, and this path does \
                         not have one"
                )
            }
            Error::NulError(ref e) => format!("{}", e),
            Error::ObjectError(ref e) => format!("{}", e),
            Error::PackageIsAlreadyInChannel(ref p, ref c) => {
                format!("{} is already in the {} channel.", p, c)
            }
            Error::PackageUpload(ref e) => format!("{}", e),
            Error::PackageDownload(ref e) => format!("{}", e),
            Error::RemotePackageNotFound(ref pkg) => {
                if pkg.fully_qualified() {
                    format!("Cannot find package in any sources: {}", pkg)
                } else {
                    format!("Cannot find a release of package in any sources: {}", pkg)
                }
            }
            Error::WriteSyncFailed => {
                format!("Could not write to destination; perhaps the disk is full?")
            }
        };
        write!(f, "{}", msg)
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::BadPort(_) => "Received an invalid port or a number outside of the valid range.",
            Error::BadBucket(ref err) => err.description(),
            Error::CaughtPanic(_, _) => "Caught a panic.",
            Error::ChannelAlreadyExists(_) => "Channel already exists.",
            Error::ChannelDoesNotExist(_) => "Channel does not exist.",
            Error::HabitatCore(ref err) => err.description(),
            Error::HabitatNet(ref err) => err.description(),
            Error::HTTP(_) => "Received an HTTP error",
            Error::InvalidPackageIdent(_) => {
                "Package identifiers must be in origin/name format (example: acme/redis)"
            }
            Error::IO(ref err) => err.description(),
            Error::NetError(ref err) => err.description(),
            Error::NulError(_) => {
                "An attempt was made to build a CString with a null byte inside it"
            }
            Error::ObjectError(ref err) => err.description(),
            Error::PackageIsAlreadyInChannel(_, _) => "Package is already in channel",
            Error::PackageUpload(ref err) => err.description(),
            Error::PackageDownload(ref err) => err.description(),
            Error::RemotePackageNotFound(_) => "Cannot find a package in any sources",
            Error::NoXFilename => "Invalid download from Builder - missing X-Filename header",
            Error::NoFilePart => {
                "An invalid path was passed - we needed a filename, and this path does not have one"
            }
            Error::MessageTypeNotFound => "Unable to find message for given type",
            Error::WriteSyncFailed => {
                "Could not write to destination; bytes written was 0 on a non-0 buffer"
            }
        }
    }
}

impl From<hab_core::Error> for Error {
    fn from(err: hab_core::Error) -> Error {
        Error::HabitatCore(err)
    }
}

impl From<ffi::NulError> for Error {
    fn from(err: ffi::NulError) -> Error {
        Error::NulError(err)
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::IO(err)
    }
}

impl From<hab_net::error::LibError> for Error {
    fn from(err: hab_net::error::LibError) -> Error {
        Error::HabitatNet(err)
    }
}

impl From<hab_net::NetError> for Error {
    fn from(err: hab_net::NetError) -> Error {
        Error::NetError(err)
    }
}

impl From<rusoto_s3::ListBucketsError> for Error {
    fn from(err: rusoto_s3::ListBucketsError) -> Error {
        Error::BadBucket(err)
    }
}

impl From<rusoto_s3::ListObjectsError> for Error {
    fn from(err: rusoto_s3::ListObjectsError) -> Error {
        Error::ObjectError(err)
    }
}
