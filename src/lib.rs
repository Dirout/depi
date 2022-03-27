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

use futures::{StreamExt, TryStreamExt};
use ipfs_api_backend_hyper::{IpfsApi, IpfsClient};

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
				eprintln!("Error: {}", e);
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
	let tor_connector = arti_hyper::ArtiHttpConnector::new(tor_client, tls_connector);
	let http = hyper::Client::builder().build::<_, Body>(tor_connector);
	let mut resp = http.get(url.try_into().unwrap()).await.unwrap();
	return hyper::body::to_bytes(resp.body_mut()).await.unwrap();
}
