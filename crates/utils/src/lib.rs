use off_chain_state::{Keccak256Hasher, SmtValue, Value};
use primitives::types::ProfitStateData;
use sparse_merkle_tree::{
    merge::{into_merge_value1, MergeValue},
    H256,
};

#[derive(Debug, Clone)]
pub struct SMTBitMap(pub H256);
impl SMTBitMap {
    pub fn reverse(&mut self) -> std::result::Result<(), &'static str> {
        let v: [u8; 32] = self.0.clone().into();
        let s: Vec<String> = v.to_vec().iter().map(|n| format!("{:08b}", n)).collect();
        let s_c = s.concat();
        let r_s = reverse_binary(&s_c);
        let mut res: Vec<u8> = binary_string_to_vec_u8(&r_s)?;
        res.reverse();
        let res: [u8; 32] = res.try_into().unwrap();
        self.0 = res.into();
        Ok(())
    }
}

impl From<H256> for SMTBitMap {
    fn from(value: H256) -> Self {
        Self(value)
    }
}

impl From<SMTBitMap> for H256 {
    fn from(value: SMTBitMap) -> Self {
        value.0
    }
}

fn reverse_binary(binary_string: &str) -> String {
    let reversed_chars: Vec<char> = binary_string.chars().rev().collect();
    reversed_chars.iter().collect()
}

fn binary_string_to_vec_u8(binary_string: &str) -> Result<Vec<u8>, &'static str> {
    if binary_string.len() % 8 != 0 && binary_string.len() != 32 {
        return Err("Binary string length must be a multiple of 8");
    }

    let mut result = Vec::new();
    for chunk in binary_string.chars().collect::<Vec<char>>().chunks(8) {
        let byte_str: String = chunk.into_iter().collect();
        let byte = u8::from_str_radix(&byte_str, 2).map_err(|_| "Invalid binary string")?;
        result.push(byte);
    }
    Ok(result)
}

pub fn get_no1_merge_value(
    path: H256,
    v: SmtValue<ProfitStateData>,
    leaves_bitmap: H256,
) -> (u8, H256) {
    let mut merge_value = MergeValue::zero();
    let mut n = 0;
    for i in 0..=u8::MAX {
        if leaves_bitmap.get_bit(i) {
            merge_value = into_merge_value1::<Keccak256Hasher>(path, v.to_h256(), i);
            n = i;
            break;
        }
    }
    match merge_value {
        MergeValue::MergeWithZero {
            base_node: _,
            zero_bits,
            zero_count: _,
        } => {
            return (n, zero_bits);
        }
        _ => (255, H256::zero()),
    }
}

pub fn vec_unique<D>(mut v: Vec<D>) -> Vec<D>
where
    D: PartialEq + std::cmp::Ord,
{
    v.sort();
    v.dedup();
    v
}

#[test]
fn test() {
    let a: H256 = [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 17, 0, 0, 0, 0, 0, 0, 0, 15, 0,
        0, 32,
    ]
    .into();
    println!("{:?}", a);
    let mut s: SMTBitMap = a.into();
    s.reverse().unwrap();
    println!("{:?}", s.0);

    let v = vec![1, 5, 5, 3, 4, 5, 6, 7, 8];
    let v = vec_u64_unique(v);
    println!("{:?}", v);
}
