/*
	This file is part of depi.
	depi is free software: you can redistribute it and/or modify
	it under the terms of the GNU Affero General Public License as published by
	the Free Software Foundation, either version 3 of the License, or
	(at your option) any later version.
	depi is distributed in the hope that it will be useful,
	but WITHOUT ANY WARRANTY; without even the implied warranty of
	MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
	GNU Affero General Public License for more details.
	You should have received a copy of the GNU Affero General Public License
	along with depi.  If not, see <https://www.gnu.org/licenses/>.
*/

#![cfg_attr(feature = "dox", feature(doc_cfg))]
#![allow(clippy::needless_doctest_main)]

use arti_hyper::*;
use async_ftp::FtpStream;
use futures::{StreamExt, TryStreamExt};
use hyper::Body;
use ipfs_api_backend_hyper::{IpfsApi, IpfsClient};
use tls_api::{TlsConnector, TlsConnectorBuilder};

/// Comply with a request using the IPFS scheme with a daemon
///
/// # Arguments
///
/// * `url` - The URL for the IPFS resource
pub async fn handle_ipfs_request_using_api(url: String) -> Vec<u8> {
	let decoded_url = urlencoding::decode(&url).unwrap();
	let ipfs_path = decoded_url.replacen("ipfs://", "", 1);
	return download_ipfs_file_from_api(ipfs_path).await;
}

/// Download an IPFS file to the local machine using an existing IPFS node
///
/// # Arguments
///
/// * `file_hash` - The CID of the file
pub async fn download_ipfs_file_from_api(file_hash: String) -> Vec<u8> {
	let client: IpfsClient = IpfsClient::default();

	match client
		.cat(&file_hash)
		.map_ok(|chunk| chunk.to_vec())
		.try_concat()
		.await
	{
		Ok(res) => res,
		Err(e) => e.to_string().as_bytes().to_vec(),
	}
}

/// Download a file to the local machine using HTTP(S)
///
/// # Arguments
///
/// * `url` - The URL of the file
pub async fn download_http_file(url: String) -> Vec<u8> {
	let stream = reqwest::get(url).await.unwrap().bytes_stream();
	tokio::pin!(stream);
	let mut file_vec: Vec<u8> = vec![];
	loop {
		match stream.next().await {
			Some(Ok(bytes)) => {
				file_vec.extend(bytes);
			}
			Some(Err(e)) => {
				eprintln!("Error: {e}");
			}
			None => break,
		}
	}
	file_vec
}

/// Download a file to the local machine using Tor
///
/// # Arguments
///
/// * `url` - The URL of the file
pub async fn download_tor_file(url: String) -> Vec<u8> {
	let config = arti_client::TorClientConfig::default();
	let tor_client = arti_client::TorClient::create_bootstrapped(config)
		.await
		.unwrap();
	let tls_connector = tls_api_native_tls::TlsConnector::builder()
		.unwrap()
		.build()
		.unwrap();
	let tor_connector = ArtiHttpConnector::new(tor_client, tls_connector);
	let http = hyper::Client::builder().build::<_, Body>(tor_connector);
	let mut resp = http.get(url.try_into().unwrap()).await.unwrap();
	return hyper::body::to_bytes(resp.body_mut())
		.await
		.unwrap()
		.to_vec();
}

/// Download a file to the local machine using FTP(S)
///
/// # Arguments
///
/// * `host` - The address of the FTP server
///
/// * `port` - Optional: The port of the FTP server
///
/// * `path` - The path to the file on the FTP server
///
/// * `username` - Optional: The username for the FTP server
///
/// * `password` - Optional: The password for the FTP server
pub async fn download_ftp_file(
	host: &str,
	port: Option<u16>,
	path: &str,
	username: &str,
	password: Option<&str>,
) -> Vec<u8> {
	let host_port = host.to_string() + ":" + port.unwrap_or(21).to_string().as_str();
	let mut ftp_stream = FtpStream::connect(&host_port)
		.await
		.unwrap_or_else(|_| panic!("Failed to connect to FTP server (`{host_port}`)."));
	if !username.is_empty() || password.is_some() {
		let pass_field = if password.is_none() {
			""
		} else {
			password.unwrap()
		};
		ftp_stream
			.login(username, pass_field)
			.await
			.unwrap_or_else(|_| {
				panic!(
					"Failed to login to FTP server at `{host_port}` (username `{username}`, password: `{pass_field}`)."
				)
			});
	};

	let remote_file = ftp_stream.simple_retr(path).await.unwrap_or_else(|_| {
		panic!(
			"Failed to download file `{path}` from FTP server at `{host_port}`."
		)
	});
	let bytes = remote_file.into_inner();

	ftp_stream
		.quit()
		.await
		.unwrap_or_else(|_| panic!("Failed to quit FTP connection to `{host_port}`."));

	bytes
}
