use std::{io::Read, path::Path};

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use mcml_names::i18_items::error_type::{CoreResult, ErrorData, ErrorType};
use md5::Md5;
use sha1::Sha1;
use sha2::{Sha256, Sha512};
use tokio::io::{AsyncRead, AsyncReadExt};

use digest::{Digest, DynDigest};

use crate::path_helper;

/// 校验类型
pub enum HashType {
    Md5,
    Sha1,
    Sha256,
    Sha512,
}

/// 将字节数组格式化为十六进制字符串（小写）
fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

fn create_hasher(hash_type: HashType) -> Box<dyn DynDigest> {
    match hash_type {
        HashType::Md5 => Box::new(Md5::new()),
        HashType::Sha1 => Box::new(Sha1::new()),
        HashType::Sha256 => Box::new(Sha256::new()),
        HashType::Sha512 => Box::new(Sha512::new()),
    }
}

/// 生成校验值
/// - `hash_type`: 校验类型
/// - `data`: 需要校验的数据
pub fn gen_hash(hash_type: HashType, data: &[u8]) -> String {
    let mut hasher = create_hasher(hash_type);
    hasher.update(data);
    bytes_to_hex(&hasher.finalize())
}

/// 从字符串生成校验值
/// - `hash_type`: 校验类型
/// - `data`: 需要计算的数据
pub fn gen_hash_from_string(hash_type: HashType, data: &str) -> String {
    let mut hasher = create_hasher(hash_type);
    hasher.update(data.as_bytes());
    bytes_to_hex(&hasher.finalize())
}

/// 从数据流生成校验值
/// - `hash_type`: 校验类型
/// - `reader`: 需要计算的数据流
pub fn gen_hash_from_reader<R: Read>(hash_type: HashType, reader: &mut R) -> CoreResult<String> {
    let mut hasher = create_hasher(hash_type);
    let mut buffer = [0u8; 1024];

    loop {
        let len = reader.read(&mut buffer).map_err(|err| {
            ErrorType::StreamError(ErrorData {
                error: err.to_string(),
            })
        })?;

        if len == 0 {
            break;
        }
        hasher.update(&buffer[..len]);
    }
    Ok(bytes_to_hex(&hasher.finalize()))
}

/// 异步从数据流生成校验值
/// - `hash_type`: 校验类型
/// - `reader`: 需要计算的数据流
pub async fn gen_hash_from_reader_async<R: AsyncRead + Unpin>(
    hash_type: HashType,
    reader: &mut R,
) -> CoreResult<String> {
    let mut hasher = create_hasher(hash_type);
    let mut buffer = [0u8; 1024];

    loop {
        let len = reader.read(&mut buffer).await.map_err(|err| {
            ErrorType::StreamError(ErrorData {
                error: err.to_string(),
            })
        })?;

        if len == 0 {
            break;
        }
        hasher.update(&buffer[..len]);
    }
    Ok(bytes_to_hex(&hasher.finalize()))
}

/// 从文件生成校验值
/// - `hash_type`: 校验类型
/// - `file`: 文件路径
pub fn gen_hash_from_file<P: AsRef<Path>>(hash_type: HashType, file: P) -> CoreResult<String> {
    let mut file = path_helper::open_read(file)?;
    gen_hash_from_reader(hash_type, &mut file)
}

/// 异步从文件生成校验值
/// - `hash_type`: 校验类型
/// - `file`: 文件路径
pub async fn gen_hash_from_file_async<P: AsRef<Path>>(
    hash_type: HashType,
    file: P,
) -> CoreResult<String> {
    let mut file = path_helper::open_read_async(file).await?;
    gen_hash_from_reader_async(hash_type, &mut file).await
}

/// 生成 Base64（字符串）
/// - `input`: 需要生成的数据
pub fn gen_base64(input: &str) -> String {
    BASE64.encode(input.as_bytes())
}

/// 反解 Base64
/// - `input`: Base64字符串
pub fn de_base64(input: &str) -> CoreResult<String> {
    let bytes = BASE64.decode(input).map_err(|err| {
        ErrorType::Base64Error(ErrorData {
            error: err.to_string(),
        })
    })?;
    Ok(String::from_utf8(bytes).map_err(|err| {
        ErrorType::Base64Error(ErrorData {
            error: err.to_string(),
        })
    })?)
}
