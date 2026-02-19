use std::io::Cursor;
use std::io::Read;

#[test]
fn test_zstd_roundtrip() {
    let data = b"Hello MyCute World";
    let compressed = zstd::stream::encode_all(Cursor::new(data), 0).expect("compression failed");
    let decoded = zstd::stream::decode_all(Cursor::new(compressed)).expect("decompression failed");
    assert_eq!(data.to_vec(), decoded);
}

#[test]
fn test_tar_creation_in_memory() {
    let mut ar = tar::Builder::new(Vec::new());
    
    // Add a file
    let mut header = tar::Header::new_gnu();
    header.set_size(4);
    header.set_cksum();
    ar.append_data(&mut header, "test.txt", &b"test"[..]).unwrap();
    
    let archive_data = ar.into_inner().unwrap();
    
    // Read it back
    let mut ar_read = tar::Archive::new(Cursor::new(archive_data));
    let mut entries = ar_read.entries().unwrap();
    
    let mut entry = entries.next().unwrap().unwrap();
    assert_eq!(entry.path().unwrap().to_str(), Some("test.txt"));
    let mut content = String::new();
    entry.read_to_string(&mut content).unwrap();
    assert_eq!(content, "test");
}
