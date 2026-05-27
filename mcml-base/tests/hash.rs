use mcml_base::hash_helper;

#[test]
fn test_hash_functions() {
    let data = b"Hello, World!";

    let md5 = hash_helper::gen_md5_from_bytes(data);
    println!("MD5: {}", md5);

    let sha1 = hash_helper::gen_sha1_from_bytes(data);
    println!("SHA1: {}", sha1);

    let sha256 = hash_helper::gen_sha256_from_bytes(data);
    println!("SHA256: {}", sha256);

    let input = "Hello, World!";
    let base64 = hash_helper::gen_base64(input);
    println!("Base64: {}", base64);

    let decoded = hash_helper::de_base64(&base64).unwrap();
    println!("Decoded: {}", decoded);
}

#[test]
fn test_string_input() {
    let text = "测试文本";
    let sha1 = hash_helper::gen_sha1_from_string(text);
    let sha256 = hash_helper::gen_sha256_from_string(text);
    println!("Text SHA1: {}", sha1);
    println!("Text SHA256: {}", sha256);
}

#[test]
fn test_file_hash() {
    let temp_file = std::env::temp_dir().join("test_hash.txt");
    std::fs::write(&temp_file, "Hello, World!").unwrap();

    let sha1 = hash_helper::gen_sha1_from_file(&temp_file).unwrap();
    let sha256 = hash_helper::gen_sha256_from_file(&temp_file).unwrap();

    println!("File SHA1: {}", sha1);
    println!("File SHA256: {}", sha256);

    std::fs::remove_file(temp_file).unwrap();
}
