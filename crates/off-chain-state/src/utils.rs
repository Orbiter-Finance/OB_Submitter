use super::*;

pub fn address_convert_to_h256(address: Address) -> H256 {
    let mut hasher = new_blake2b();
    let mut buf = [0u8; 32];
    hasher.update(address.as_bytes());
    hasher.finalize(&mut buf);
    buf.into()
}
