use std::{
    error::Error,
    fs::File,
    io::{BufReader, Read},
    path::Path,
};

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use sha1::{Digest, Sha1};
use sha2::{Sha256, Sha512};
use tokio::io::AsyncReadExt;

/// 将字节数组格式化为十六进制字符串（小写）
fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// 获取 MD5 值（字节数组）
pub fn gen_md5_from_bytes(data: &[u8]) -> String {
    let digest = md5::compute(data);
    bytes_to_hex(&digest.0)
}

/// 获取 SHA1 值（字节数组）
pub fn gen_sha1_from_bytes(data: &[u8]) -> String {
    let mut hasher = Sha1::new();
    hasher.update(data);
    let result = hasher.finalize();
    bytes_to_hex(&result)
}

/// 获取 SHA1 值（字符串）
pub fn gen_sha1_from_string(input: &str) -> String {
    gen_sha1_from_bytes(input.as_bytes())
}

/// 获取 SHA256 值（字符串）
pub fn gen_sha256_from_string(input: &str) -> String {
    gen_sha256_from_bytes(input.as_bytes())
}

/// 获取 SHA256 值（文件路径）
pub fn gen_sha256_from_file<P: AsRef<Path>>(file: P) -> Result<String, Box<dyn Error>> {
    let file = File::open(file)?;
    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut buffer = Vec::new();
    reader.read_to_end(&mut buffer)?;
    hasher.update(&buffer);
    let result = hasher.finalize();
    Ok(bytes_to_hex(&result))
}

/// 获取 SHA1 值（Stream 流）
pub fn gen_sha1_from_reader<R: Read>(reader: &mut R) -> Result<String, Box<dyn Error>> {
    let mut hasher = Sha1::new();
    let mut buffer = Vec::new();
    reader.read_to_end(&mut buffer)?;
    hasher.update(&buffer);
    let result = hasher.finalize();
    Ok(bytes_to_hex(&result))
}

/// 获取 SHA1 值（文件路径）
pub fn gen_sha1_from_file<P: AsRef<Path>>(file: P) -> Result<String, Box<dyn Error>> {
    let file = File::open(file)?;
    let mut reader = BufReader::new(file);
    gen_sha1_from_reader(&mut reader)
}

/// 获取 MD5 值（Stream 流）
pub fn gen_md5_from_reader<R: Read>(reader: &mut R) -> Result<String, Box<dyn Error>> {
    let mut buffer = Vec::new();
    reader.read_to_end(&mut buffer)?;
    let digest = md5::compute(&buffer);
    Ok(bytes_to_hex(&digest.0))
}

/// 获取 SHA1 值（异步 Stream 流）
pub async fn gen_sha1_from_reader_async<R: tokio::io::AsyncRead + Unpin>(
    reader: &mut R,
) -> Result<String, Box<dyn Error>> {
    let mut hasher = Sha1::new();
    let mut buffer = Vec::new();
    reader.read_to_end(&mut buffer).await?;
    hasher.update(&buffer);
    let result = hasher.finalize();
    Ok(bytes_to_hex(&result))
}

/// 获取 SHA256 值（字节数组）
pub fn gen_sha256_from_bytes(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    bytes_to_hex(&result)
}

/// 获取 SHA256 值（Stream 流）
pub fn gen_sha256_from_reader<R: Read>(reader: &mut R) -> Result<String, Box<dyn Error>> {
    let mut hasher = Sha256::new();
    let mut buffer = Vec::new();
    reader.read_to_end(&mut buffer)?;
    hasher.update(&buffer);
    let result = hasher.finalize();
    Ok(bytes_to_hex(&result))
}

/// 获取 SHA256 值（异步 Stream 流）
pub async fn gen_sha256_from_reader_async<R: tokio::io::AsyncRead + Unpin>(
    reader: &mut R,
) -> Result<String, Box<dyn Error>> {
    let mut hasher = Sha256::new();
    let mut buffer = Vec::new();
    reader.read_to_end(&mut buffer).await?;
    hasher.update(&buffer);
    let result = hasher.finalize();
    Ok(bytes_to_hex(&result))
}

/// 获取 SHA512 值（异步 Stream 流）
pub async fn gen_sha512_from_reader_async<R: tokio::io::AsyncRead + Unpin>(
    reader: &mut R,
) -> Result<String, Box<dyn Error>> {
    let mut hasher = Sha512::new();
    let mut buffer = Vec::new();
    reader.read_to_end(&mut buffer).await?;
    hasher.update(&buffer);
    let result = hasher.finalize();
    Ok(bytes_to_hex(&result))
}

/// 生成 Base64（字符串）
pub fn gen_base64(input: &str) -> String {
    BASE64.encode(input.as_bytes())
}

/// 反解 Base64
pub fn de_base64(input: &str) -> Result<String, Box<dyn Error>> {
    let bytes = BASE64.decode(input)?;
    Ok(String::from_utf8(bytes)?)
}
